#[cfg(feature = "ast")]
use crate::traits::to_datex_expression_data::ToDatexExpressionData;
use crate::{
    core_compiler::to_instructions::ToInstructions,
    dif::{
        deserialize_with_serde_context::DeserializeWithSerdeContextDyn,
        serialize_with_serde_context::SerializeWithSerdeContextDyn,
    },
    traits::{
        classification::Classification,
        convert_parts::{FromParts, IntoParts},
        convert_value_container::ConvertValueContainer,
        datex_hash::DatexHash,
        dyn_eq::DynEq,
        get_core_lib_type_id::GetCoreLibTypeId,
        get_datex_type::GetDatexType,
        iter_parts::IterParts,
        local_child_path_resolver::LocalChildPathResolver,
        try_clone::TryClone,
        value_access::ValueAccess,
    },
    value_updates::update_handler::UpdateHandlerImpl,
    values::core_values::native::DatexNativeOps,
};
use core::any::Any;

#[cfg(feature = "ast")]
pub trait DatexNativeBase:
    ToDatexExpressionData
    + ConvertValueContainer
    + GetDatexType
    + IntoParts
    + FromParts
    + Classification
    + GetCoreLibTypeId
    + DatexHash
    + ToInstructions
    + SerializeWithSerdeContextDyn
    + DeserializeWithSerdeContextDyn
{
}

#[cfg(feature = "ast")]
impl<T> DatexNativeBase for T where
    T: ToDatexExpressionData
        + ConvertValueContainer
        + GetDatexType
        + IntoParts
        + FromParts
        + Classification
        + GetCoreLibTypeId
        + DatexHash
        + ToInstructions
        + SerializeWithSerdeContextDyn
        + DeserializeWithSerdeContextDyn
{
}

#[cfg(not(feature = "ast"))]
pub trait DatexNativeBase:
    ConvertValueContainer
    + GetDatexType
    + IntoParts
    + FromParts
    + Classification
    + GetCoreLibTypeId
    + DatexHash
    + ToInstructions
    + SerializeWithSerdeContextDyn
    + DeserializeWithSerdeContextDyn
{
}
#[cfg(not(feature = "ast"))]
impl<T> DatexNativeBase for T where
    T: ConvertValueContainer
        + GetDatexType
        + IntoParts
        + FromParts
        + Classification
        + GetCoreLibTypeId
        + DatexHash
        + ToInstructions
        + SerializeWithSerdeContextDyn
        + DeserializeWithSerdeContextDyn
{
}

// TODO: better solution than duplicate definition of trait for different feature flags?
#[cfg(feature = "ast")]
pub trait DatexNative:
    ToDatexExpressionData
    + Any
    + DynEq
    + DatexHash
    + FromParts
    + IntoParts
    + GetCoreLibTypeId
    + ValueAccess
    + TryClone
    + ConvertValueContainer
    + Classification
    + ToInstructions
    + DatexNativeOps
    + UpdateHandlerImpl
    + LocalChildPathResolver
    + SerializeWithSerdeContextDyn
    + DeserializeWithSerdeContextDyn
    + IterParts
{
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn type_name(&self) -> &'static str {
        core::any::type_name::<Self>()
    }
}

#[cfg(not(feature = "ast"))]
pub trait DatexNative:
    Any
    + DynEq
    + DatexHash
    + FromParts
    + IntoParts
    + GetCoreLibTypeId
    + ValueAccess
    + TryClone
    + ConvertValueContainer
    + Classification
    + ToInstructions
    + DatexNativeOps
    + UpdateHandlerImpl
    + LocalChildPathResolver
    + SerializeWithSerdeContextDyn
    + DeserializeWithSerdeContextDyn
    + IterParts
{
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn type_name(&self) -> &'static str {
        core::any::type_name::<Self>()
    }
}
