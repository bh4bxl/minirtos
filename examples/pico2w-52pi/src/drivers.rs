use alloc::vec;

use minirtos_drivers::uart::UartPl011;
use minirtos_kernel::{MemoryBlock, task::Priority};
use minirtos_services::driver::{
    DriverConfig, DriverServiceConfig, DriverServiceTable, UartService, interface::Driver,
};

use crate::UART0;

const UART0_BASE: usize = 0x4007_0000;

pub fn init_driver_services() {
    let mut driver_services = DriverServiceTable::new();

    let _ = driver_services.register(
        DriverServiceConfig {
            name: "uart0",
            stack_size: 1024,
            priority: Priority(100),
        },
        UartService::new(
            UART0,
            UartPl011::<u32, u32>::new(DriverConfig {
                dev_mem_blocks: vec![MemoryBlock::new(UART0_BASE, 0x1000)],
                interrupts: vec![],
                dmas: vec![],
            })
            .unwrap(),
        )
        .unwrap(),
    );

    let _ = driver_services.spawn_all();
}
