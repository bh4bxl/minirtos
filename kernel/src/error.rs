#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum SysError {
    InvalidArgument = -1,
    NoMemory = -2,
    NoResource = -3,
    Busy = -4,
    Timeout = -5,
    WouldBlock = -6,
    NotFound = -7,
    AlreadyExists = -8,
    NotSupported = -9,
    PermissionDenied = -10,

    InvalidState = -20,
    Closed = -21,

    Io = -40,
    DeviceError = -41,
    ProtocolError = -42,
}

impl TryFrom<i32> for SysError {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            -1 => Ok(Self::InvalidArgument),
            -2 => Ok(Self::NoMemory),
            -3 => Ok(Self::NoResource),
            -4 => Ok(Self::Busy),
            -5 => Ok(Self::Timeout),
            -6 => Ok(Self::WouldBlock),
            -7 => Ok(Self::NotFound),
            -8 => Ok(Self::AlreadyExists),
            -9 => Ok(Self::NotSupported),
            -10 => Ok(Self::PermissionDenied),

            -20 => Ok(Self::InvalidState),
            -21 => Ok(Self::Closed),

            -40 => Ok(Self::Io),
            -41 => Ok(Self::DeviceError),
            -42 => Ok(Self::ProtocolError),

            _ => Err(value),
        }
    }
}
