
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flags {
    pub value: u8
}

impl Flags {
    pub fn new(value: u8) -> Self {
        Self { value }
    }

    pub fn zero() -> Self {
        Self { value: 0 }
    }

    pub fn get(&self, index: usize) -> bool {
        (self.value & (1 << index)) != 0
    }

    pub fn set(&mut self, index: usize) -> &mut Self {
        self.value |= 1 << index;
        self
    }
}
