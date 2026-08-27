#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskId(usize);

impl TaskId {
    pub fn from_raw(id: usize) -> Self {
        Self(id)
    }

    pub fn raw(self) -> usize {
        self.0
    }
}
