use crate::types::{
    traits::type_match::TypeSuperset, type_definition::impl_type::ImplMarkers,
};

impl TypeSuperset<ImplMarkers> for ImplMarkers {
    fn is_superset_of(&self, other: &ImplMarkers) -> bool {
        // other must include all impls that self includes
        let all_impls_in_self_are_in_other = self
            .impl_markers
            .iter()
            .all(|self_impl| other.impl_markers.contains(self_impl));

        if !all_impls_in_self_are_in_other {
            return false;
        }
        true
    }
}
