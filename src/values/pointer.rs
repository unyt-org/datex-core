#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pointer {
    pointer_id: u64,
}

impl Pointer {
    pub fn create() -> Self {
        Self { pointer_id: 42 } // FIXME
    }
    pub fn new(pointer_id: u64) -> Self {
        Self { pointer_id }
    }

    pub fn pointer_id(&self) -> u64 {
        self.pointer_id
    }
}
