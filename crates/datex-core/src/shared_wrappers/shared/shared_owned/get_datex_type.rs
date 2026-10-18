use crate::{
    prelude::*,
    runtime::cache::shared_references_cache::SharedReferencesCache,
    shared_values::{SharedContainerMutability, SharedContainerOwnership},
    shared_wrappers::shared::shared_owned::Shared,
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

impl<T> GetDatexType for Shared<T>
where
    T: DatexNative + GetDatexType,
{
    fn datex_type(context: &mut SharedReferencesCache) -> Type {
        Type::Definition(TypeDefinitionWithMetadata::new(
            TypeDefinition::Box(Box::new(T::datex_type(context))),
            TypeMetadata::Shared {
                mutability: SharedContainerMutability::Mutable,
                ownership: SharedContainerOwnership::Owned,
            },
        ))
    }
}
