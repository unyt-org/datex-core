use crate::{
    core_compiler::to_instructions::ToInstructions,
    traits::{
        classification::Classification,
        convert_parts::{FromParts, IntoParts},
        convert_value_container::ConvertValueContainer,
        datex_hash::DatexHash,
        dyn_eq::DynEq,
        get_core_lib_type_id::GetCoreLibTypeId,
        get_datex_type::GetDatexType,
        try_clone::TryClone,
        value_access::ValueAccess,
    },
};
#[cfg(feature = "ast")]
use crate::{
    preludes::derive::DatexNativeOps,
    traits::to_datex_expression_data::ToDatexExpressionData,
};
use core::any::Any;

#[cfg(feature = "ast")]
pub trait DatexNativeBase:
    ConvertValueContainer
    + GetDatexType
    + IntoParts
    + FromParts
    + Classification
    + GetCoreLibTypeId
    + DatexHash
    + ToDatexExpressionData
    + ToInstructions
{
}
#[cfg(feature = "ast")]
impl<T> DatexNativeBase for T where
    T: ConvertValueContainer
        + GetDatexType
        + IntoParts
        + FromParts
        + Classification
        + GetCoreLibTypeId
        + DatexHash
        + ToDatexExpressionData
        + ToInstructions
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
{
}

// TODO: better solution than duplicate definition of trait for different feature flags?
#[cfg(feature = "ast")]
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
    + ToDatexExpressionData
    + DatexNativeOps
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
{
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
