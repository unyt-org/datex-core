#[macro_export]
macro_rules! derive_prelude {
    () => {
        #[allow(unused_imports)]
        use $crate::{
            core_compiler::{
                to_instructions::ToInstructions, value_visitor::ValueVisitor,
            },
            datex_registry::get_impls_for,
            dif::{
                deserialize_serde_context::DeserializeSerdeContext,
                deserialize_with_serde_context::DeserializeWithSerdeContext,
                serde_context::SerdeContext,
                serialize_with_serde_context::SerializeWithSerdeContext,
                value_with_serde_context::ValueWithSerdeContext,
            },
            instruction::{
                Instruction, regular_instruction::RegularInstruction,
            },
            libs::core::type_id::{CoreLibBaseTypeId, CoreLibTypeId},
            libs::core::{
                core_lib_id::{CoreLibId, CoreLibIdIndex},
                value_id::CoreLibValueId,
            },
            prelude::*,
            runtime::cache::shared_references_cache::{
                SharedReferencesCache, SharedTypeReservation,
            },
            serde,
            serde_compat::{
                serde_to_value_container, try_serde_from_value_container,
            },
            shared_values::{
                PointerAddress, SelfOwnedPointerAddress,
                errors::{AccessError, KeyNotFoundError},
            },
            traits::{
                classification::Classification,
                convert_parts::{
                    FromParts, HasPartsKind, IntoParts, PartsKind
                },
                convert_value::ConvertValue,
                convert_value_container::ConvertValueContainer,
                datex_hash::DatexHash,
                datex_native_only_structural::DatexNativeOnlyStructural,
                datex_native_structural::DatexNativeStructural,
                get_core_lib_type_id::GetCoreLibTypeId,
                get_datex_type::GetDatexType,
                iter_parts::IterParts,
                local_child_path_resolver::LocalChildPathResolver,
                value_access::ValueAccess,
            },
            types::{
                entities::{
                    entity_impls::{EntityImpl, EntityImplMethod},
                    entity_type_definition::EntityTypeDefinition,
                },
                entity_type::EntityType,
                literal_type_definition::LiteralTypeDefinition,
                r#type::Type,
                type_definition::{
                    TypeDefinition,
                    callable::{CallableKind, CallableTypeDefinition},
                    list::ListTypeDefinition,
                    map::MapTypeDefinition,
                    tagged_type::TaggedTypeDefinition,
                    union::UnionTypeDefinition,
                },
            },
            utils::{goat::Goat, goat_mut::GoatMut},
            value_updates::update_handler::{
                UpdateCallbackDataAccess, UpdateHandlerImpl,
            },
            values::{
                core_value::CoreValue,
                core_value::borrowed_core_value::{
                    BorrowedCoreValue, BorrowedCoreValueMut,
                },
                core_values::{
                    callable::{Callable, CallableBody},
                    list::List,
                    map::Map,
                    native::{DatexNative, DatexNativeOps},
                    text::Text,
                },
                value::{
                    Value,
                    value_classification::{ValueClassification, ValueTag},
                },
                value_container::borrowed_value_container::{
                    BorrowedValueContainer, BorrowedValueContainerMut,
                },
                value_container::{
                    ValueContainer,
                },
            },
            types::entities::entity_type_definition::dyn_deserialize::{DeFn, de_as}
        };
    };
}

#[macro_export]
macro_rules! derive_prelude_ast {
    () => {
        #[allow(unused_imports)]
        use $crate::{
            ast,
            ast::{
                expressions::DatexExpressionData, expressions::Statements,
                spanned::Spanned,
            },
            traits::to_datex_expression_data::ToDatexExpressionData,
        };
    };
}

pub(crate) use derive_prelude;
pub(crate) use derive_prelude_ast;
