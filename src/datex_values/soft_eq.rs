pub trait SoftEq {
    /// Check if two values are equal, ignoring the type.
    fn soft_eq(&self, other: &Self) -> bool;
}
