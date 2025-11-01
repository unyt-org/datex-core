pub trait Identity {
    /// Check if two values are strictly identical (same pointer, same value, same type, same permissions).
    fn identical(&self, other: &Self) -> bool;
}

#[macro_export]
macro_rules! assert_identical {
    ($left_val:expr, $right_val:expr $(,)?) => {
        if !$left_val.identical(&$right_val) {
            core::panic!(
                "identity assertion failed: `(left is right)`\n  left: `{:?}`,\n right: `{:?}`",
                $left_val, $right_val
            );
        }
    };
}
