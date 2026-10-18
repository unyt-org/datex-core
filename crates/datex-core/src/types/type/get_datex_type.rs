use crate::libs::core::type_id::CoreLibBaseTypeId;
use crate::runtime::cache::shared_references_cache::SharedReferencesCache;
use crate::traits::get_datex_type::GetDatexType;
use crate::types::r#type::Type;
use crate::types::type_definition::TypeDefinition;

impl GetDatexType for Type {
    fn datex_type(_cache: &mut SharedReferencesCache) -> Type {
        TypeDefinition::CoreType(CoreLibBaseTypeId::Type.into()).into()
    }
}