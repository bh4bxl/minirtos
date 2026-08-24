use minirtos_abi::EndpointHandle;

use crate::{SysError, synchronization::CriticalSectionLock, task::TaskId};

use super::Endpoint;

pub(crate) const MAX_ENDPOINTS: usize = 16;

struct EndpointEntry {
    owner: TaskId,
    endpoint: Endpoint,
}

pub(crate) struct EndpointRegistry {
    entries: [Option<EndpointEntry>; MAX_ENDPOINTS],
}

impl EndpointRegistry {
    pub const fn new() -> Self {
        Self {
            entries: [const { None }; MAX_ENDPOINTS],
        }
    }

    pub(crate) fn create(&mut self, owner: TaskId) -> Result<EndpointHandle, SysError> {
        for (index, entry) in self.entries.iter_mut().enumerate() {
            if entry.is_none() {
                *entry = Some(EndpointEntry {
                    owner,
                    endpoint: Endpoint::new(),
                });

                return Ok(EndpointHandle::from_raw(index as u32));
            }
        }

        Err(SysError::NoResource)
    }

    pub(crate) fn endpoint(&self, handle: EndpointHandle) -> Result<&Endpoint, SysError> {
        let entry = self
            .entries
            .get(handle.raw() as usize)
            .ok_or(SysError::NotFound)?
            .as_ref()
            .ok_or(SysError::NotFound)?;

        Ok(&entry.endpoint)
    }

    pub(crate) fn owner(&self, handle: EndpointHandle) -> Result<TaskId, SysError> {
        let entry = self
            .entries
            .get(handle.raw() as usize)
            .ok_or(SysError::NotFound)?
            .as_ref()
            .ok_or(SysError::NotFound)?;

        Ok(entry.owner)
    }

    pub(crate) fn destroy(
        &mut self,
        owner: TaskId,
        handle: EndpointHandle,
    ) -> Result<(), SysError> {
        let slot = self
            .entries
            .get_mut(handle.raw() as usize)
            .ok_or(SysError::NotFound)?;

        let entry = slot.as_ref().ok_or(SysError::NotFound)?;

        if entry.owner != owner {
            return Err(SysError::InvalidState);
        }

        *slot = None;

        Ok(())
    }
}

pub(crate) static IPC_REGISTRY: CriticalSectionLock<EndpointRegistry> =
    CriticalSectionLock::new(EndpointRegistry::new());
