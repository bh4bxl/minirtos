#[repr(u8)]
#[derive(Clone, Copy)]
pub enum SyscallId {
    StartFirst = 0,
    Task = 1,
    Sync = 2,
    Ipc = 3,
}

impl TryFrom<u8> for SyscallId {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::StartFirst),
            1 => Ok(Self::Task),
            2 => Ok(Self::Sync),
            3 => Ok(Self::Ipc),
            _ => Err(()),
        }
    }
}
