pub trait Identity {
    /// Check if two values are strictly identical (same pointer, same value, same type, same permissions).
    fn identical(&self, other: &Self) -> bool;
}

#[macro_export]
macro_rules! assert_identical {
    ($left_val:expr_2021, $right_val:expr_2021 $(,)?) => {
        if !$left_val.identical(&$right_val) {
            panic!(
                "identity assertion failed: `(left is right)`\n  left: `{:?}`,\n right: `{:?}`",
                $left_val, $right_val
            );
        }
    };
}
