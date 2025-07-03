pub trait ValueEq {
    /// Check if two values are equal, ignoring the type.
    fn value_eq(&self, other: &Self) -> bool;
}

#[macro_export]
macro_rules! assert_value_eq {
    ($left_val:expr, $right_val:expr $(,)?) => {
        if !$left_val.value_eq(&$right_val) {
            panic!(
                "value equality assertion failed: `(left === right)`\n  left: `{:?}`,\n right: `{:?}`",
                $left_val, $right_val
            );
        }
    };
}
