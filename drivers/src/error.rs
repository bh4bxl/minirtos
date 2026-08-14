#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum DevError {
    // Device state
    Busy = -1,
    NoSuchDevice = -2,
    Unsupported = -3,
    WouldBlock = -4,
    Timeout = -5,

    // Request / I/O
    InvalidArg = -6,
    Io = -7,

    // Driver framework
    AlreadyInitialized = -8,
    NoFreeDriverSlot = -9,

    // Resource
    NoMem = -10,
    NotInitialized = -11,
    InitFailed = -12,
}

impl TryFrom<i32> for DevError {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            -1 => Ok(Self::Busy),
            -2 => Ok(Self::NoSuchDevice),
            -3 => Ok(Self::Unsupported),
            -4 => Ok(Self::WouldBlock),
            -5 => Ok(Self::Timeout),
            -6 => Ok(Self::InvalidArg),
            -7 => Ok(Self::Io),
            -8 => Ok(Self::AlreadyInitialized),
            -9 => Ok(Self::NoFreeDriverSlot),
            -10 => Ok(Self::NoMem),
            -11 => Ok(Self::NotInitialized),
            -12 => Ok(Self::InitFailed),
            other => Err(other),
        }
    }
}
