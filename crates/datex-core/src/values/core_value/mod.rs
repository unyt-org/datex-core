//! This module defines the [CoreValue] enum, which represents the fundamental value types in DATEX.
//! Each variant of CoreValue holds the underlying value of a specific type, such as [Integer], [Text], or [List].
//! CoreValues can be converted to and from native Rust types.

use crate::{
    libs::core::type_id::{
        CoreLibBaseTypeId, CoreLibTypeId, CoreLibVariantTypeId,
    },
    prelude::*,
    types::entities::entity_type_definition::EntityTypeDefinition,
    values::core_values::instant::Instant,
};
pub mod serde_dif;
use crate::{
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
use core::fmt::{Debug, Display, Formatter};

pub mod borrowed_core_value;
pub mod convert_core_value;
mod convert_parts;
mod datex_hash;
mod equality;
mod iter_parts;
mod ops;
mod to_instructions;
mod try_clone;

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
    Instant(Instant),
    /// Used for nested values, e.g. #Tagged (shared 42)
    Box(Box<ValueContainer>),
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
            CoreValue::Instant(_) => {
                CoreLibTypeId::Base(CoreLibBaseTypeId::Instant)
            }
            CoreValue::Uninitialized => {
                CoreLibTypeId::Base(CoreLibBaseTypeId::Never)
            }
            CoreValue::Box(_) => CoreLibTypeId::Base(CoreLibBaseTypeId::Box),
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

    /// Check if the CoreValue is a combined value type (List, Map)
    /// that contains inner ValueContainers.
    pub fn is_collection_value(&self) -> bool {
        core::matches!(self, CoreValue::List(_) | CoreValue::Map(_))
    }

    /// Get the default type of the CoreValue type definition.
    /// This method uses the CoreLibPointerId to retrieve the corresponding
    /// type reference from the core library.
    /// For example, a CoreValue::TypedInteger(i32) will return the type ref integer/i32
    pub fn core_lib_type_id(&self) -> CoreLibTypeId {
        CoreLibTypeId::from(self)
    }

    /// Casts the value to a [Text] value
    /// Note: in contrast to [try_cast_to], [Text] values are not wrapped in quotation marks.
    pub fn cast_to_text(&self) -> Text {
        match self {
            CoreValue::Text(text) => text.clone(),
            _ => Text(self.to_string()),
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
            CoreValue::Instant(instant) => write!(f, "{instant}"),
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
        }
    }
}

impl From<Text> for CoreValue {
    fn from(value: Text) -> Self {
        CoreValue::Text(value)
    }
}

impl From<Endpoint> for CoreValue {
    fn from(value: Endpoint) -> Self {
        CoreValue::Endpoint(value)
    }
}

impl From<Range> for CoreValue {
    fn from(value: Range) -> Self {
        CoreValue::Range(value)
    }
}

impl From<Callable> for CoreValue {
    fn from(value: Callable) -> Self {
        CoreValue::Callable(value)
    }
}

impl From<Type> for CoreValue {
    fn from(value: Type) -> Self {
        CoreValue::Type(value)
    }
}

impl From<EntityTypeDefinition> for CoreValue {
    fn from(value: EntityTypeDefinition) -> Self {
        CoreValue::EntityTypeDefinition(value)
    }
}

impl From<Map> for CoreValue {
    fn from(value: Map) -> Self {
        CoreValue::Map(value)
    }
}

impl From<List> for CoreValue {
    fn from(value: List) -> Self {
        CoreValue::List(value)
    }
}

impl From<Decimal> for CoreValue {
    fn from(value: Decimal) -> Self {
        CoreValue::Decimal(value)
    }
}

impl From<TypedDecimal> for CoreValue {
    fn from(value: TypedDecimal) -> Self {
        CoreValue::TypedDecimal(value)
    }
}

impl From<Integer> for CoreValue {
    fn from(value: Integer) -> Self {
        CoreValue::Integer(value)
    }
}

impl From<TypedInteger> for CoreValue {
    fn from(value: TypedInteger) -> Self {
        CoreValue::TypedInteger(value)
    }
}

impl From<Boolean> for CoreValue {
    fn from(value: Boolean) -> Self {
        CoreValue::Boolean(value)
    }
}

impl From<Instant> for CoreValue {
    fn from(value: Instant) -> Self {
        CoreValue::Instant(value)
    }
}

#[cfg(test)]
/// This module contains tests for the CoreValue struct.
/// Each CoreValue is a representation of an underlying native value.
/// The tests cover addition, casting, and type conversions.
mod tests {
    use log::{debug, info};

    use super::*;
    use crate::traits::{
        convert_value::ConvertValue, get_core_lib_type_id::GetCoreLibTypeId,
    };

    #[test]
    fn type_construct() {
        let a = 42i32.to_value();
        assert_eq!(a.core_lib_type_id().to_string(), "integer/i32");
    }

    #[test]
    fn addition() {
        let a = 42i32.to_value();
        let b = 11i32.to_value();

        let a_plus_b = (a.clone() + b.clone()).unwrap();
        assert_eq!(a_plus_b.clone(), 53i32.to_value());
        info!("{} + {} = {}", a.clone(), b.clone(), a_plus_b.clone());
    }

    #[test]
    fn endpoint() {
        let endpoint: Endpoint = "@test"
            .to_string()
            .to_value()
            .try_into_core_value()
            .unwrap()
            .cast_to_endpoint()
            .unwrap();
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
}
