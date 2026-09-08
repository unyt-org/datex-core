#[cfg(feature = "ast")]
pub use crate::{
    ast,
    ast::{
        expressions::DatexExpressionData, expressions::Statements,
        spanned::Spanned,
    },
    traits::to_datex_expression_data::ToDatexExpressionData,
};
#[doc(hidden)]
pub use crate::{
    core_compiler::to_instructions::ToInstructions,
    core_compiler::value_visitor::ValueVisitor,
    datex_registry::{get_impls, get_impls_for},
    instruction::{Instruction, regular_instruction::RegularInstruction},
    libs::core::type_id::{CoreLibBaseTypeId, CoreLibTypeId},
    prelude::*,
    runtime::cache::shared_references_cache::{
        SharedReferencesCache, SharedTypeReservation,
    },
    serde_compat::{serde_to_value_container, try_serde_from_value_container},
    shared_values::{
        SelfOwnedPointerAddress,
        errors::{AccessError, KeyNotFoundError},
    },
    traits::classification::Classification,
    traits::convert_core_value::ConvertCoreValue,
    traits::convert_parts::FromParts,
    traits::convert_parts::IntoParts,
    traits::convert_value_container::ConvertValueContainer,
    traits::datex_hash::DatexHash,
    traits::datex_native_only_structural::DatexNativeOnlyStructural,
    traits::datex_native_structural::DatexNativeStructural,
    traits::get_core_lib_type_id::GetCoreLibTypeId,
    traits::get_datex_type::GetDatexType,
    traits::static_classification::StaticClassification,
    traits::value_access::ValueAccess,
    types::type_definition::callable::{CallableKind, CallableTypeDefinition},
    types::{
        entities::entity_impls::EntityImpl,
        entities::entity_impls::EntityImplMethod,
        entities::entity_type_definition::EntityTypeDefinition,
        literal_type_definition::LiteralTypeDefinition,
        r#type::Type,
        type_definition::{
            TypeDefinition, list::ListTypeDefinition, map::MapTypeDefinition,
            tagged_type::TaggedTypeDefinition, union::UnionTypeDefinition,
        },
    },
    utils::{goat::Goat, goat_mut::GoatMut},
    values::core_values::callable::{Callable, CallableBody},
    values::value::borrowed_value::{BorrowedCoreValue, BorrowedCoreValueMut},
    values::{
        borrowed_value_container::{
            AsBorrowed, AsBorrowedMut, BorrowedValueContainer,
            BorrowedValueContainerMut,
        },
        core_value::CoreValue,
        core_values::{
            list::List, map::Map, native::DatexNative, native::DatexNativeOps,
            text::Text,
        },
        value::Value,
        value_container::{ValueContainer, value_key::BorrowedValueKey},
    },
};
