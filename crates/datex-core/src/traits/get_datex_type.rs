use crate::{
    libs::core::type_id::CoreLibBaseTypeId,
    runtime::cache::shared_references_cache::SharedReferencesCache,
    types::r#type::Type,
};

// Returns the DATEX [Type] for the target
pub trait GetDatexType {
    fn datex_type(_cache: &mut SharedReferencesCache) -> Type {
        Type::core(CoreLibBaseTypeId::Any)
    }
}
