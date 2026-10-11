use crate::{
    traits::structural_eq::StructuralEq, values::core_values::instant::Instant,
};

impl StructuralEq for Instant {
    fn structural_eq(&self, other: &Self) -> bool {
        self == other
    }
}
