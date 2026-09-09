//! This module defines the [CoreValue] enum, which represents the fundamental value types in DATEX.
//! Each variant of CoreValue holds the underlying value of a specific type, such as [Integer], [Text], or [List].
//! CoreValues can be converted to and from native Rust types.

use crate::{
    libs::core::type_id::{
        CoreLibBaseTypeId, CoreLibTypeId, CoreLibVariantTypeId,
    },
    prelude::*,
    types::entities::entity_type_definition::EntityTypeDefinition,
    values::core_values::native::{DatexNative, NativeCoreValue},
};
pub mod serde_dif;
use crate::{
    preludes::derive::{ConvertCoreValue, DatexNativeStructural},
    types::r#type::Type,
    values::{
        core_values::{
            boolean::Boolean,
            callable::Callable,
            decimal::{
                Decimal,
                typed_decimal::{DecimalTypeVariant, TypedDecimal},
            },
            endpoint::Endpoint,
            integer::{
                Integer,
                typed_integer::{IntegerTypeVariant, TypedInteger},
            },
            list::List,
            map::Map,
            range::Range,
            text::Text,
        },
        value_container::ValueContainer,
    },
};
use binrw::error::CustomError;
use core::fmt::{Debug, Display, Formatter};

mod child_iterator;
mod datex_hash;
pub mod equality;
pub mod ops;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
mod to_instructions;
pub mod try_clone;

#[derive(Default, Clone, Debug)]
pub enum CoreValue {
    #[default]
    Uninitialized,
    Null,
    Boolean(Boolean),
    Integer(Integer),
    TypedInteger(TypedInteger),
    Decimal(Decimal),
    TypedDecimal(TypedDecimal),
    Text(Text),
    Endpoint(Endpoint),
    List(List),
    Map(Map),
    Type(Type),
    EntityTypeDefinition(EntityTypeDefinition),
    Callable(Callable),
    Range(Range),
    /// Used for nested values, e.g. #Tagged (shared 42)
    Box(Box<ValueContainer>),
    /// Native rust value with DATEX representation
    Native(NativeCoreValue),
}

/// Implementation that allows direct conversion from any type that implements the [DatexNativeStructural] trait into a [CoreValue].
impl<T: DatexNativeStructural> From<T> for CoreValue {
    fn from(value: T) -> Self {
        CoreValue::native(value)
    }
}

impl From<&str> for CoreValue {
    fn from(value: &str) -> Self {
        CoreValue::Text(value.into())
    }
}

impl<T> FromIterator<T> for CoreValue
where
    T: Into<ValueContainer>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        CoreValue::List(List::new(iter.into_iter().map(Into::into).collect()))
    }
}

impl From<&CoreValue> for CoreLibTypeId {
    fn from(value: &CoreValue) -> Self {
        match value {
            CoreValue::Map(_) => CoreLibTypeId::Base(CoreLibBaseTypeId::Map),
            CoreValue::List(_) => CoreLibTypeId::Base(CoreLibBaseTypeId::List),
            CoreValue::Text(_) => CoreLibTypeId::Base(CoreLibBaseTypeId::Text),
            CoreValue::Boolean(_) => {
                CoreLibTypeId::Base(CoreLibBaseTypeId::Boolean)
            }
            CoreValue::TypedInteger(i) => CoreLibTypeId::Variant(
                CoreLibVariantTypeId::Integer(i.variant()),
            ),
            CoreValue::TypedDecimal(d) => CoreLibTypeId::Variant(
                CoreLibVariantTypeId::Decimal(d.variant()),
            ),
            CoreValue::Integer(_) => {
                CoreLibTypeId::Base(CoreLibBaseTypeId::Integer)
            }
            CoreValue::Decimal(_) => {
                CoreLibTypeId::Base(CoreLibBaseTypeId::Decimal)
            }
            CoreValue::Endpoint(_) => {
                CoreLibTypeId::Base(CoreLibBaseTypeId::Endpoint)
            }
            CoreValue::Null => CoreLibTypeId::Base(CoreLibBaseTypeId::Null),
            CoreValue::Type(_) => CoreLibTypeId::Base(CoreLibBaseTypeId::Type),
            CoreValue::Callable(_) => {
                CoreLibTypeId::Base(CoreLibBaseTypeId::Callable)
            }
            CoreValue::Range(_) => {
                CoreLibTypeId::Base(CoreLibBaseTypeId::Range)
            }
            CoreValue::EntityTypeDefinition(_nominal_type) => {
                CoreLibTypeId::Base(CoreLibBaseTypeId::Never) // TODO: what is the type of nominal type? do we even need to handle this?
            }
            CoreValue::Uninitialized => {
                CoreLibTypeId::Base(CoreLibBaseTypeId::Never)
            }
            CoreValue::Box(_) => CoreLibTypeId::Base(CoreLibBaseTypeId::Box),
            CoreValue::Native(native) => native.core_lib_type_id(),
        }
    }
}

impl CoreValue {
    pub fn new<T>(value: T) -> CoreValue
    where
        CoreValue: From<T>,
    {
        value.into()
    }

    /// Creates a new CoreValue from a native value that implements the [DatexNative] trait.
    pub fn native(value: impl DatexNative) -> CoreValue {
        CoreValue::Native(NativeCoreValue::new(value))
    }
    pub fn native_boxed(value: Box<dyn DatexNative>) -> CoreValue {
        CoreValue::Native(NativeCoreValue { value })
    }

    /// Check if the CoreValue is a combined value type (List, Map)
    /// that contains inner ValueContainers.
    pub fn is_collection_value(&self) -> bool {
        core::matches!(self, CoreValue::List(_) | CoreValue::Map(_))
    }

    /// Get the default type of the CoreValue type definition.
    /// This method uses the CoreLibPointerId to retrieve the corresponding
    /// type reference from the core library.
    /// For example, a CoreValue::TypedInteger(i32) will return the type ref integer/i32
    pub fn default_core_type(&self) -> CoreLibTypeId {
        CoreLibTypeId::from(self)
    }

    /// Tries to get a borrow of the current value as the specified type.
    /// Does not perform any type conversion.
    pub fn try_as<T>(&self) -> Option<&T>
    where
        T: ConvertCoreValue,
    {
        T::try_borrow_from_core_value(self).ok()
    }

    pub fn try_as_mut<T>(&mut self) -> Option<&mut T>
    where
        T: ConvertCoreValue,
    {
        T::try_borrow_mut_from_core_value(self).ok()
    }

    /// Tries to convert the current value into the specific specified type.
    /// Does not perform any type conversion.
    pub fn try_into_value<T>(self) -> Result<T, CoreValue>
    where
        T: ConvertCoreValue,
    {
        T::try_from_core_value(self)
    }

    /// Casts the value to a [Text] value
    /// Note: in contrast to [try_cast_to], [Text] values are not wrapped in quotation marks.
    pub fn cast_to_text(&self) -> Text {
        match self {
            CoreValue::Text(text) => text.clone(),
            _ => Text(self.to_string()),
        }
    }

    /// Tries to downcast the CoreValue to a native value of type T.
    /// This method will return Some(Box<T>) if the CoreValue is a Native variant and the underlying value can be downcast to T.
    pub fn downcast_native<T: DatexNative>(self) -> Option<Box<T>> {
        if let CoreValue::Native(native_value) = self {
            native_value.into_any().downcast::<T>().ok()
        } else {
            None
        }
    }

    /// Tries to downcast the CoreValue to a reference of a native value of type T.
    /// This method will return Some(&T) if the CoreValue is a Native variant and the underlying value can be downcast to T.
    pub fn downcast_native_ref<T: DatexNative>(&self) -> Option<&T> {
        if let CoreValue::Native(native_value) = self {
            native_value.as_any().downcast_ref::<T>()
        } else {
            None
        }
    }

    /// Tries to downcast the CoreValue to a mutable reference of a native value of type T.
    /// This method will return Some(&mut T) if the CoreValue is a Native variant and the underlying value can be downcast to T.
    pub fn downcast_native_mut<T: DatexNative>(&mut self) -> Option<&mut T> {
        if let CoreValue::Native(native_value) = self {
            native_value.as_any_mut().downcast_mut::<T>()
        } else {
            None
        }
    }

    pub fn cast_to_bool(&self) -> Option<Boolean> {
        match self {
            CoreValue::Text(text) => Some(Boolean(!text.0.is_empty())),
            CoreValue::Boolean(bool) => Some(bool.clone()),
            CoreValue::TypedInteger(int) => Some(Boolean(int.as_i128()? != 0)),
            CoreValue::Null => Some(Boolean(false)),
            _ => None,
        }
    }

    pub fn cast_to_decimal(&self) -> Option<Decimal> {
        match self {
            CoreValue::Text(text) => {
                text.to_string().parse::<f64>().ok().map(Decimal::from)
            }
            CoreValue::TypedInteger(int) => {
                Some(Decimal::from(int.as_i128()? as f64))
            }
            CoreValue::TypedDecimal(decimal) => {
                Some(Decimal::from(decimal.clone()))
            }
            CoreValue::Integer(int) => {
                Some(Decimal::from(int.as_i128()? as f64))
            }
            CoreValue::Decimal(decimal) => Some(decimal.clone()),
            _ => None,
        }
    }

    pub fn cast_to_typed_decimal(
        &self,
        variant: DecimalTypeVariant,
    ) -> Option<TypedDecimal> {
        match self {
            CoreValue::Text(text) => TypedDecimal::try_from_string_and_variant(
                text.as_str(),
                variant,
            )
            .ok(),
            CoreValue::TypedInteger(int) => Some(
                TypedDecimal::try_from_string_and_variant(
                    &int.to_string(),
                    variant,
                )
                .ok()?,
            ),
            CoreValue::TypedDecimal(decimal) => Some(
                TypedDecimal::try_from_string_and_variant(
                    &decimal.to_string(),
                    variant,
                )
                .ok()?,
            ),
            CoreValue::Integer(int) => Some(
                TypedDecimal::try_from_string_and_variant(
                    &int.to_string(),
                    variant,
                )
                .ok()?,
            ),
            CoreValue::Decimal(decimal) => Some(
                TypedDecimal::try_from_string_and_variant(
                    &decimal.to_string(),
                    variant,
                )
                .ok()?,
            ),
            _ => None,
        }
    }

    // FIXME #314 discuss here - shall we fit the integer in the minimum viable type?
    pub fn _cast_to_integer_internal(&self) -> Option<TypedInteger> {
        match self {
            CoreValue::Text(text) => {
                Integer::try_from_string(&text.to_string())
                    .map(|x| Some(x.to_smallest_fitting()))
                    .unwrap_or(None)
            }
            CoreValue::TypedInteger(int) => {
                Some(int.to_smallest_fitting().clone())
            }
            CoreValue::Integer(int) => {
                Some(TypedInteger::IBig(int.clone()).to_smallest_fitting())
            }
            CoreValue::Decimal(decimal) => Some(
                TypedInteger::from(decimal.into_f64() as i128)
                    .to_smallest_fitting(),
            ),
            CoreValue::TypedDecimal(decimal) => Some(
                TypedInteger::from(decimal.as_f64() as i64)
                    .to_smallest_fitting(),
            ),
            _ => None,
        }
    }

    // TODO #315 improve conversion logic
    pub fn cast_to_integer(&self) -> Option<Integer> {
        match self {
            CoreValue::Text(text) => {
                Integer::try_from_string(&text.to_string()).ok()
            }
            CoreValue::TypedInteger(int) => Some(int.as_integer()),
            CoreValue::Integer(int) => Some(int.clone()),
            CoreValue::Decimal(decimal) => {
                // FIXME #316 currently bad as f64 can be infinity or nan
                // convert decimal directly to integer into_f64 is wrong here
                Some(Integer::from(decimal.into_f64() as i128))
            }
            CoreValue::TypedDecimal(decimal) => {
                decimal.as_integer().map(Integer::from)
            }
            _ => None,
        }
    }

    pub fn cast_to_typed_integer(
        &self,
        variant: IntegerTypeVariant,
    ) -> Option<TypedInteger> {
        match self {
            CoreValue::Text(text) => TypedInteger::try_from_string_and_variant(
                text.as_str(),
                variant,
            )
            .ok(),
            CoreValue::TypedInteger(int) => {
                TypedInteger::try_from_string_and_variant(
                    &int.to_string(),
                    variant,
                )
                .ok()
            }
            CoreValue::Integer(int) => {
                TypedInteger::try_from_string_and_variant(
                    int.to_string().as_str(),
                    variant,
                )
                .ok()
            }
            CoreValue::Decimal(decimal) => {
                Some(TypedInteger::from(decimal.into_f64() as i128))
            }
            CoreValue::TypedDecimal(decimal) => {
                decimal.as_integer().map(TypedInteger::from)
            }
            _ => None,
        }
    }

    pub fn cast_to_endpoint(&self) -> Option<Endpoint> {
        match self {
            CoreValue::Text(text) => Endpoint::try_from(text.as_str()).ok(),
            CoreValue::Endpoint(endpoint) => Some(endpoint.clone()),
            _ => None,
        }
    }
}

impl Display for CoreValue {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        match self {
            CoreValue::Type(ty) => write!(f, "{ty}"),
            CoreValue::Boolean(bool) => write!(f, "{bool}"),
            CoreValue::TypedInteger(int) => write!(f, "{int}"),
            CoreValue::TypedDecimal(decimal) => write!(f, "{decimal}"),
            CoreValue::Text(text) => write!(f, "{text}"),
            CoreValue::Null => write!(f, "null"),
            CoreValue::Endpoint(endpoint) => write!(f, "{endpoint}"),
            CoreValue::Map(map) => write!(f, "{map}"),
            CoreValue::Range(range) => {
                write!(f, "{}..{}", range.start, range.end)
            }
            CoreValue::Integer(integer) => write!(f, "{integer}"),
            CoreValue::Decimal(decimal) => write!(f, "{decimal}"),
            CoreValue::List(list) => write!(f, "{list}"),
            CoreValue::Callable(_callable) => write!(f, "[[ callable ]]"),
            CoreValue::EntityTypeDefinition(container) => {
                write!(f, "{container}")
            }
            CoreValue::Uninitialized => write!(f, "[[ uninitialized ]]"),
            CoreValue::Box(inner) => write!(f, "({})", inner),
            CoreValue::Native(native) => {
                write!(f, "{native}")
            }
        }
    }
}

#[cfg(test)]
/// This module contains tests for the CoreValue struct.
/// Each CoreValue is a representation of an underlying native value.
/// The tests cover addition, casting, and type conversions.
mod tests {
    use log::{debug, info};

    use super::*;

    #[test]
    fn type_construct() {
        let a = CoreValue::from(42i32);
        assert_eq!(a.default_core_type().to_string(), "integer/i32");
    }

    #[test]
    fn addition() {
        let a = CoreValue::from(42i32);
        let b = CoreValue::from(11i32);

        let a_plus_b = (a.clone() + b.clone()).unwrap();
        assert_eq!(a_plus_b.clone(), CoreValue::from(53));
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b.clone());
    }

    #[test]
    fn endpoint() {
        let endpoint: Endpoint =
            CoreValue::from("@test").cast_to_endpoint().unwrap();
        debug!("Endpoint: {endpoint}");
        assert_eq!(endpoint.to_string(), "@test");
    }

    #[test]
    pub fn range_from_core() {
        assert_eq!(
            CoreValue::Range(Range::new(
                CoreValue::Integer(Integer::from(11)).into(),
                CoreValue::Integer(Integer::from(13)).into()
            ))
            .to_string(),
            "11..13"
        );
    }

    #[test]
    pub fn native_values() {
        let native_string = CoreValue::native("Hello DATEX".to_string());
    }
}
