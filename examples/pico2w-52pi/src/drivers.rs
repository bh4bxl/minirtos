use minirtos_drivers::uart::{Pl011Config, UartPl011};
use minirtos_kernel::task::Priority;
use minirtos_services::driver::{DriverServiceConfig, DriverServiceTable, UartService};

use crate::UART0;

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
            UartPl011::new(),
            &Pl011Config {
                base_addr: 0x1234_5678,
            },
        )
        .unwrap(),
    );

    let _ = driver_services.spawn_all();
}
