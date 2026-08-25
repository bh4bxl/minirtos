#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceId(u32);

impl ServiceId {
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceOp {
    Register = 0,
    Lookup = 1,
    Unregister = 2,
}

impl TryFrom<u32> for ServiceOp {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Register),
            1 => Ok(Self::Lookup),
            2 => Ok(Self::Unregister),
            _ => Err(()),
        }
    }
}
