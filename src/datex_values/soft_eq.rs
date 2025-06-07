pub trait SoftEq {
    /// Check if two values are equal, ignoring the type.
    fn soft_eq(&self, other: &Self) -> bool;
}

#[macro_export]
macro_rules! assert_soft_eq {
    ($a:expr, $b:expr $(,)?) => {
        if !$a.soft_eq(&$b) {
            panic!(
                "Soft equality assertion failed: {:?} is not soft equal to {:?}",
                $a, $b
            );
        }
    };
}