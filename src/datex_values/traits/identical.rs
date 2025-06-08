pub trait Identical {
    /// Check if two values are strictly identical (same pointer, same value, same type, same permissions).
    fn identical(&self, other: &Self) -> bool;
}
