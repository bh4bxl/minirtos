use rp235x_hal::{
    pac,
    pio::{PIOBuilder, PIOExt, PinDir, Rx, SM0, ShiftDirection, StateMachine, Stopped, Tx},
};

use crate::{
    drivers::{
        delay_ns, gpio,
        wlan::cyw43::pio_ctrl::{PioCtrl, PioId},
    },
    sys::device_driver::DevError,
};

pub(super) struct PioSpi {
    gpio: &'static dyn gpio::interface::Gpio,
    clk: gpio::Pin,
    dio: gpio::Pin,
    cs: gpio::Pin,

    rx: Option<Rx<(pac::PIO0, SM0)>>,
    tx: Option<Tx<(pac::PIO0, SM0)>>,
    sm: Option<StateMachine<(pac::PIO0, SM0), Stopped>>,

    pio_ctrl: PioCtrl,
}

const IRQ_SAMPLE_DELAY_NS: u32 = 100;

//#[allow(dead_code)]
impl PioSpi {
    pub const fn new(
        gpio: &'static dyn gpio::interface::Gpio,
        clk: usize,
        dio: usize,
        cs: usize,
    ) -> Self {
        Self {
            gpio,
            clk: gpio::Pin(clk),
            dio: gpio::Pin(dio),
            cs: gpio::Pin(cs),
            rx: None,
            tx: None,
            sm: None,
            pio_ctrl: PioCtrl::new(PioId::Pio0),
        }
    }

    pub fn init(&self) {
        // Pins configuration
        self.gpio
            .pin_config(self.clk.0, gpio::Function::PIO0, gpio::Pull::None, None);
        self.gpio
            .pin_config(self.dio.0, gpio::Function::PIO0, gpio::Pull::None, None);
        self.gpio.pin_config(
            self.cs.0,
            gpio::Function::SIO,
            gpio::Pull::Up,
            Some(gpio::Direction::Output),
        );

        self.set_cs_high();
    }

    pub fn init_hw(&mut self, pio0: pac::PIO0, resets: &mut pac::RESETS) {
        let (mut pio, sm0, _, _, _) = pio0.split(resets);

        // Pio program, src: https://github.com/raspberrypi/pico-sdk/blob/master/src/rp2_common/pico_cyw43_driver/cyw43_bus_pio_spi.pio
        let program = pio::pio_file!(
            "src/drivers/wlan/cyw43/cyw43_bus_pio_spi.pio",
            select_program("spi_gap01_sample0")
        );

        let installed = pio.install(&program.program).unwrap();

        let installed = self.pio_ctrl.load_program(0, installed);

        let tx_only_program = installed.set_wrap(self.pio_ctrl.wrap_tx_only()).unwrap();

        let (mut sm, rx, tx) = PIOBuilder::from_installed_program(tx_only_program)
            .out_pins(self.dio.0 as u8, 1)
            .in_pin_base(self.dio.0 as u8)
            .side_set_pin_base(self.clk.0 as u8)
            .out_shift_direction(ShiftDirection::Left)
            .in_shift_direction(ShiftDirection::Left)
            .autopull(true)
            .pull_threshold(32)
            .autopush(true)
            .push_threshold(32)
            .clock_divisor_fixed_point(20, 0)
            .build(sm0);

        sm.set_pindirs([
            (self.clk.0 as u8, PinDir::Output),
            (self.dio.0 as u8, PinDir::Output),
        ]);

        self.rx = Some(rx);
        self.tx = Some(tx);
        self.sm = Some(sm);
    }

    #[inline]
    fn set_cs_high(&self) {
        self.gpio.set_level(&self.cs, gpio::Level::High);
    }

    #[inline]
    fn set_cs_low(&self) {
        self.gpio.set_level(&self.cs, gpio::Level::Low);
    }

    #[inline]
    fn start_spi_comms(&self) {
        self.gpio.set_function(&self.dio, gpio::Function::PIO0);
        self.gpio.set_function(&self.clk, gpio::Function::PIO0);
        self.gpio.set_level(&self.clk, gpio::Level::Low);

        self.set_cs_low();
    }

    #[inline]
    fn stop_spi_comms(&self) {
        self.set_cs_high();

        delay_ns(IRQ_SAMPLE_DELAY_NS);
    }

    #[inline]
    fn set_dio_input(&mut self) {
        let sm = self.sm.as_mut().unwrap();

        sm.set_pindirs([
            (self.clk.0 as u8, PinDir::Output),
            (self.dio.0 as u8, PinDir::Input),
        ]);
    }

    #[inline]
    fn set_dio_output(&mut self) {
        let sm = self.sm.as_mut().unwrap();

        sm.set_pindirs([
            (self.clk.0 as u8, PinDir::Output),
            (self.dio.0 as u8, PinDir::Output),
        ]);
    }

    fn reset_sm_for_tx(&mut self) {
        let mut sm = self.sm.take().unwrap();

        sm.drain_tx_fifo();

        sm.set_pindirs([
            (self.clk.0 as u8, PinDir::Output),
            (self.dio.0 as u8, PinDir::Output),
        ]);

        self.sm = Some(sm);
    }

    fn load_x(&mut self, value: u32) {
        {
            let tx = self.tx.as_mut().unwrap();
            while !tx.write(value) {}
        }

        let sm = self.sm.as_mut().unwrap();
        self.pio_ctrl.exec_out_x_32(sm);
    }

    fn load_y(&mut self, value: u32) {
        {
            let tx = self.tx.as_mut().unwrap();
            while !tx.write(value) {}
        }

        let sm = self.sm.as_mut().unwrap();
        self.pio_ctrl.exec_out_y_32(sm);
    }

    fn write_bytes(&mut self, tx_buf: &[u8]) -> Result<usize, DevError> {
        if tx_buf.is_empty() {
            return Err(DevError::InvalidArg);
        }

        if tx_buf.len() & 0b11 != 0 {
            return Err(DevError::InvalidArg);
        }

        if (tx_buf.as_ptr() as usize) & 0b11 != 0 {
            return Err(DevError::InvalidArg);
        }

        let tx_bits = tx_buf.len() as u32 * 8;

        self.reset_sm_for_tx();

        self.set_dio_output();

        self.load_x(tx_bits - 1);

        self.load_y(0);

        self.pio_ctrl.exec_jmp_start(self.sm.as_mut().unwrap());

        let sm = self.sm.take().unwrap().start();

        let words = unsafe {
            core::slice::from_raw_parts(
                tx_buf.as_ptr() as *const u32,
                tx_buf.len() / core::mem::size_of::<u32>(),
            )
        };

        {
            let tx = self.tx.as_mut().unwrap();
            for &word in words {
                while !tx.write(word.swap_bytes()) {}
            }
        }

        self.pio_ctrl.wait_idle();

        self.sm = Some(sm.stop());

        self.set_dio_input();

        //
        Ok(tx_buf.len())
    }

    fn write_read_bytes(&mut self, _tx_buf: &[u8], _rx_buf: &mut [u8]) -> Result<usize, DevError> {
        Err(DevError::Unsupported)
    }

    pub fn transfer(&mut self, tx_buf: &[u8], rx_buf: &mut [u8]) -> Result<usize, DevError> {
        match (!tx_buf.is_empty(), !rx_buf.is_empty()) {
            // TX only
            (true, false) => {
                self.start_spi_comms();
                let ret = self.write_bytes(tx_buf);
                self.stop_spi_comms();
                ret
            }
            // TX + RX
            (true, true) => {
                self.start_spi_comms();
                let ret = self.write_read_bytes(tx_buf, rx_buf);
                self.stop_spi_comms();
                ret
            }
            // RX only: SDK also unsupported
            (false, true) => Err(DevError::Unsupported),
            // Unreachable
            (false, false) => unreachable!(),
        }
    }
}
