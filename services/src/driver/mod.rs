mod serivce_table;
mod uart;

pub use serivce_table::{DriverService, DriverServiceConfig, DriverServiceTable};
pub use uart::{
    UartConfig, UartOp,
    interface::UartDriver,
    uart_client::{Uart, UartId},
    uart_service::UartService,
};
