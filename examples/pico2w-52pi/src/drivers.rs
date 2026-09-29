use minirtos_abi::Priority;
use minirtos_drivers::uart::Pl011;
use minirtos_kernel::{
    MemoryBlock,
    interface::driver::{Driver, DriverConfig},
};
use minirtos_services::driver::{DriverServiceConfig, DriverServiceTable, UartService};

use crate::services::DRV_UART0;

const UART0_BASE: usize = 0x4007_0000;

pub fn init_driver_services() {
    defmt::info!("Init drivers");

    let mut driver_services = DriverServiceTable::new();

    let _ = driver_services.register(
        DriverServiceConfig {
            name: "uart0",
            stack_size: 1024,
            priority: Priority(100),
        },
        UartService::new(
            DRV_UART0,
            Pl011::<u32, u32>::new(DriverConfig {
                dev_mem_blocks: &[MemoryBlock::new(UART0_BASE, 0x1000)],
                interrupts: &[],
                dmas: &[],
            })
            .unwrap(),
        )
        .unwrap(),
    );

    let _ = driver_services.spawn_all();
}
