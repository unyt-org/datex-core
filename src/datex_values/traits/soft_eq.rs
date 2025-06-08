pub trait SoftEq {
    /// Check if two values are equal, ignoring the type.
    fn soft_eq(&self, other: &Self) -> bool;
}

#[macro_export]
macro_rules! assert_soft_eq {
    ($left_val:expr, $right_val:expr $(,)?) => {
        if !$left_val.soft_eq(&$right_val) {
            panic!(
                "soft assertion failed: `(left == right)`\n  left: `{:?}`,\n right: `{:?}`",
                $left_val, $right_val
            );
        }
    };
}
