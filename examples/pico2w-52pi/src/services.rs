use minirtos_abi::{Priority, ServiceId, SysError};
use minirtos_kernel::{stdio::register_console, sys::sleep_ms};
use minirtos_services::{
    driver::{Uart, UartId},
    system::{
        ServiceConfig,
        console::{console_client::ConsoleClient, console_service::ConsoleService},
        start_system_service,
    },
};
use static_cell::StaticCell;

//
// Service IDs
//
pub const DRV_UART0: ServiceId = ServiceId::from_raw(100);
pub const SYS_CONSOLE: ServiceId = ServiceId::from_raw(200);

static CONSOLE_UART: StaticCell<Uart> = StaticCell::new();
static CONSOLE_SERVICE: StaticCell<ConsoleService<'static>> = StaticCell::new();
static SYSTEM_CONSOLE: StaticCell<ConsoleClient> = StaticCell::new();

pub fn init_system_service() -> Result<(), SysError> {
    defmt::info!("Init system service");

    // Console backend
    let uart = CONSOLE_UART.init(Uart::open(UartId::new(DRV_UART0.raw()))?);

    // Console service
    let console_service = CONSOLE_SERVICE.init(ConsoleService::new(SYS_CONSOLE));

    console_service.set_backend(uart);

    let console_service_config = ServiceConfig {
        name: "console",
        stack_size: 1024,
        priority: Priority(100),
    };

    start_system_service(console_service, &console_service_config)?;
    defmt::warn!("con1");
    sleep_ms(100);

    let sys_con = ConsoleClient::open(SYS_CONSOLE)?;
    defmt::warn!("con2");

    // Register the new console instead of early uart console
    let console = SYSTEM_CONSOLE.init(sys_con);
    defmt::warn!("con3");

    register_console(console);

    Ok(())
}
