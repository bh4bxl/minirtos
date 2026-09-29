mod serivce_table;
pub mod uart;

pub use serivce_table::{DriverService, DriverServiceConfig, DriverServiceTable};
pub use uart::{
    interface::UartDriver,
    uart_client::{Uart, UartId},
    uart_service::UartService,
};
