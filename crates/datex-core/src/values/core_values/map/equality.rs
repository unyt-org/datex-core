use crate::{
    traits::structural_eq::StructuralEq, values::core_values::map::Map,
};
use core::hash::{Hash, Hasher};

impl PartialEq for Map {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl Eq for Map {}

impl Hash for Map {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (k, v) in self.iter() {
            k.hash(state);
            v.hash(state);
        }
    }
}

impl StructuralEq for Map {
    fn structural_eq(&self, other: &Self) -> bool {
        if self.size() != other.size() {
            return false;
        }
        for ((key, value), (other_key, other_value)) in
            self.iter().zip(other.iter())
        {
            if !key.structural_eq(&other_key)
                || !value.structural_eq(other_value)
            {
                return false;
            }
        }
        true
    }
}
