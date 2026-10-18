use crate::{
    prelude::*,
    runtime::cache::shared_references_cache::SharedReferencesCache,
    shared_values::{SharedContainerMutability, SharedContainerOwnership},
    traits::get_datex_type::GetDatexType,
    types::{
        r#type::Type,
        type_definition::TypeDefinition,
        type_definition_with_metadata::{
            TypeDefinitionWithMetadata, TypeMetadata,
        },
    },
    values::core_values::native::DatexNative,
};
use crate::shared_values::ReferenceMutability;
use crate::shared_wrappers::shared::shared_ref::SharedRef;

impl<T> GetDatexType for SharedRef<T>
where
    T: DatexNative + GetDatexType,
{
    fn datex_type(context: &mut SharedReferencesCache) -> Type {
        Type::Definition(TypeDefinitionWithMetadata::new(
            TypeDefinition::Box(Box::new(T::datex_type(context))),
            TypeMetadata::Shared {
                mutability: SharedContainerMutability::Mutable,
                ownership: SharedContainerOwnership::Referenced(ReferenceMutability::Mutable), // TODO: distinguish between mutable and immutable references
            },
        ))
    }
}
