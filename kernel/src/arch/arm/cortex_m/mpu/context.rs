const MAX_REGIONS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    ReadExecute,
    ReadOnly,
    ReadWrite,
    DeviceReadWrite,
}

#[derive(Clone, Copy, Debug)]
pub struct Region {
    pub base: usize,
    pub size: usize,
    pub access: Access,
}

impl Region {
    pub const fn new(base: usize, size: usize, access: Access) -> Self {
        Self { base, size, access }
    }

    pub const fn code(base: usize, size: usize) -> Self {
        Self::new(base, size, Access::ReadExecute)
    }

    pub const fn read_only(base: usize, size: usize) -> Self {
        Self::new(base, size, Access::ReadOnly)
    }

    pub const fn read_write(base: usize, size: usize) -> Self {
        Self::new(base, size, Access::ReadWrite)
    }

    pub const fn device(base: usize, size: usize) -> Self {
        Self::new(base, size, Access::DeviceReadWrite)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtectionError {
    InvalidRegion,
    NoRegion,
}

#[derive(Clone, Copy)]
pub struct ProtectionContext {
    pub(super) regions: [Option<Region>; MAX_REGIONS],
}

impl ProtectionContext {
    pub const fn new() -> Self {
        Self {
            regions: [None; MAX_REGIONS],
        }
    }

    pub fn add_region(&mut self, region: Region) -> Result<(), ProtectionError> {
        validate_region(region)?;

        let Some(slot) = self.regions.iter_mut().find(|slot| slot.is_none()) else {
            return Err(ProtectionError::NoRegion);
        };

        *slot = Some(region);
        Ok(())
    }

    pub fn remove_region(&mut self, base: usize, size: usize) -> Result<(), ProtectionError> {
        let slot = self
            .regions
            .iter_mut()
            .find(|region| {
                matches!(
                    region,
                    Some(region)
                        if region.base == base
                            && region.size == size
                )
            })
            .ok_or(ProtectionError::InvalidRegion)?;

        *slot = None;

        Ok(())
    }
}

impl Default for ProtectionContext {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_region(region: Region) -> Result<(), ProtectionError> {
    const ALIGNMENT: usize = 32;

    if region.size == 0
        || !region.base.is_multiple_of(ALIGNMENT)
        || !region.size.is_multiple_of(ALIGNMENT)
        || region.base.checked_add(region.size).is_none()
    {
        return Err(ProtectionError::InvalidRegion);
    }

    Ok(())
}
