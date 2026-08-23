use crate::{
    synchronization::{CriticalSectionLock, Event, Mutex, Semaphore},
    task::TaskId,
};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SyncHandle(pub(super) u32);

impl SyncHandle {
    pub(super) const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }
}

enum SyncObject {
    Semaphore(Semaphore),
    Mutex(Mutex),
    Event(Event),
}

pub(super) struct SyncEntry {
    owner: TaskId,
    object: SyncObject,
}

const MAX_SYNC_OBJECTS: usize = 16;
pub(super) struct SyncRegistry {
    entries: [Option<SyncEntry>; MAX_SYNC_OBJECTS],
}

impl SyncRegistry {
    pub const fn new() -> Self {
        Self {
            entries: [const { None }; MAX_SYNC_OBJECTS],
        }
    }

    pub(super) fn create_semaphore(&mut self, owner: TaskId, initial: isize) -> Option<SyncHandle> {
        self.insert(owner, SyncObject::Semaphore(Semaphore::new(initial)))
    }

    pub(super) fn semaphore(&self, handle: SyncHandle) -> Option<&Semaphore> {
        let entry = self.entries.get(handle.0 as usize)?.as_ref()?;

        match &entry.object {
            SyncObject::Semaphore(sem) => Some(sem),
            _ => None,
        }
    }

    pub(crate) fn create_mutex(&mut self, owner: TaskId) -> Option<SyncHandle> {
        self.insert(owner, SyncObject::Mutex(Mutex::new()))
    }

    pub(crate) fn mutex(&self, handle: SyncHandle) -> Option<&Mutex> {
        let entry = self.entries.get(handle.0 as usize)?.as_ref()?;

        match &entry.object {
            SyncObject::Mutex(mutex) => Some(mutex),
            _ => None,
        }
    }

    pub(super) fn create_event(&mut self, owner: TaskId, signaled: bool) -> Option<SyncHandle> {
        self.insert(owner, SyncObject::Event(Event::new(signaled)))
    }

    pub(super) fn event(&self, handle: SyncHandle) -> Option<&Event> {
        let entry = self.entries.get(handle.0 as usize)?.as_ref()?;

        match &entry.object {
            SyncObject::Event(e) => Some(e),
            _ => None,
        }
    }

    pub(super) fn destroy(&mut self, owner: TaskId, handle: SyncHandle) -> bool {
        let Some(entry) = self.entries.get_mut(handle.0 as usize) else {
            return false;
        };

        let Some(obj) = entry.as_ref() else {
            return false;
        };

        if obj.owner != owner {
            return false;
        }

        *entry = None;
        true
    }

    fn insert(&mut self, owner: TaskId, object: SyncObject) -> Option<SyncHandle> {
        for (index, entry) in self.entries.iter_mut().enumerate() {
            if entry.is_none() {
                *entry = Some(SyncEntry { owner, object });
                return Some(SyncHandle::from_raw(index as u32));
            }
        }

        None
    }
}

pub(crate) static SYNC_REGISTRY: CriticalSectionLock<SyncRegistry> =
    CriticalSectionLock::new(SyncRegistry::new());
