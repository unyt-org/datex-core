pub trait StructuralEq {
    /// Check if two values are equal, ignoring the type.
    fn structural_eq(&self, other: &Self) -> bool;
}

#[macro_export]
macro_rules! assert_structural_eq {
    ($left_val:expr, $right_val:expr $(,)?) => {
        if !$left_val.structural_eq(&$right_val) {
            panic!(
                "structural equality assertion failed: `(left == right)`\n  left: `{:?}`,\n right: `{:?}`",
                $left_val, $right_val
            );
        }
    };
}
impl<T: StructuralEq> StructuralEq for Option<T> {
    fn structural_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => a.structural_eq(b),
            (None, None) => {
                todo!("decide if None is structurally equal to None")
            }
            _ => false,
        }
    }
}
