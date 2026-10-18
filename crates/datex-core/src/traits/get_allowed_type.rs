use crate::runtime::cache::shared_references_cache::SharedReferencesCache;
use crate::traits::get_datex_type::GetDatexType;
use crate::types::type_definition::TypeDefinition;

/// A trait for types that can provide their allowed type definition.
pub trait GetAllowedType {
    fn allowed_type(&self, cache: &mut SharedReferencesCache) -> TypeDefinition;
}

/// Auto implementation of [GetAllowedType] for all types that implement [GetDatexType].
impl<T: GetDatexType> GetAllowedType for T {
    fn allowed_type(&self, cache: &mut SharedReferencesCache) -> TypeDefinition {
        TypeDefinition::from(T::datex_type(cache))
    }
}