use crate::{
    traits::structural_eq::StructuralEq, values::core_values::text::Text,
};

impl StructuralEq for Text {
    fn structural_eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}
