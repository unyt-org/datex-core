pub trait StructuralEq {
    /// Check if two values are equal, ignoring the type.
    fn structural_eq(&self, other: &Self) -> bool;
}

#[macro_export]
macro_rules! assert_structural_eq {
    ($left_val:expr_2021, $right_val:expr_2021 $(,)?) => {
        if !$left_val.structural_eq(&$right_val) {
            panic!(
                "structural equality assertion failed: `(left == right)`\n  left: `{:?}`,\n right: `{:?}`",
                $left_val, $right_val
            );
        }
    };
}
