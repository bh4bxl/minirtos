use minirtos_abi::{EndpointHandle, ServiceId, SysError};

use crate::{synchronization::CriticalSectionLock, task::TaskId};

const MAX_SERVICES: usize = 16;

#[derive(Clone, Copy)]
struct ServiceEntry {
    id: ServiceId,
    owner: TaskId,
    endpoint: EndpointHandle,
}

pub(crate) struct ServiceRegistry {
    entries: [Option<ServiceEntry>; MAX_SERVICES],
}

impl ServiceRegistry {
    pub const fn new() -> Self {
        Self {
            entries: [const { None }; MAX_SERVICES],
        }
    }

    pub(crate) fn register(
        &mut self,
        owner: TaskId,
        id: ServiceId,
        endpoint: EndpointHandle,
    ) -> Result<(), SysError> {
        if self.entries.iter().flatten().any(|entry| entry.id == id) {
            return Err(SysError::AlreadyExists);
        }

        let slot = self
            .entries
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(SysError::NoResource)?;

        *slot = Some(ServiceEntry {
            id,
            owner,
            endpoint,
        });

        Ok(())
    }

    pub(crate) fn lookup(&self, id: ServiceId) -> Result<EndpointHandle, SysError> {
        self.entries
            .iter()
            .flatten()
            .find(|entry| entry.id == id)
            .map(|entry| entry.endpoint)
            .ok_or(SysError::NotFound)
    }

    pub(crate) fn unregister(&mut self, owner: TaskId, id: ServiceId) -> Result<(), SysError> {
        let slot = self
            .entries
            .iter_mut()
            .find(|slot| slot.as_ref().is_some_and(|entry| entry.id == id))
            .ok_or(SysError::NotFound)?;

        let entry = slot.as_ref().ok_or(SysError::NotFound)?;

        if entry.owner != owner {
            return Err(SysError::InvalidState);
        }

        *slot = None;

        Ok(())
    }

    pub(crate) fn unregister_all(&mut self, owner: TaskId) {
        for slot in &mut self.entries {
            if slot.as_ref().is_some_and(|entry| entry.owner == owner) {
                *slot = None;
            }
        }
    }
}

pub(crate) static SERVICE_REGISTRY: CriticalSectionLock<ServiceRegistry> =
    CriticalSectionLock::new(ServiceRegistry::new());
