use minirtos_abi::SysError;

pub mod console_client;
pub mod console_service;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConsoleOp {
    Register = 0,
    WritePending = 1,
    WaitRead = 2,
    ClearInput = 3,
    Flush = 4,
}

impl TryFrom<u32> for ConsoleOp {
    type Error = SysError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Register),
            1 => Ok(Self::WritePending),
            2 => Ok(Self::WaitRead),
            3 => Ok(Self::ClearInput),
            4 => Ok(Self::Flush),
            _ => Err(SysError::InvalidArgument),
        }
    }
}
