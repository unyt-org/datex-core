use crate::{
    libs::core::{
        core_lib_id::CoreLibIdIndex,
        type_id::{CoreLibBaseTypeId, CoreLibTypeId, CoreLibVariantTypeId},
    },
    prelude::*,
    types::literal_type_definition::LiteralTypeDefinition,
    values::core_values::{
        decimal::typed_decimal::TypedDecimal,
        integer::typed_integer::TypedInteger,
    },
};
use serde::{Serialize, Serializer, de::SeqAccess, ser::SerializeSeq};
impl Serialize for LiteralTypeDefinition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            LiteralTypeDefinition::Boolean(value) => {
                serializer.serialize_bool(value.0)
            }

            LiteralTypeDefinition::Text(value) => {
                serializer.serialize_str(value.as_str())
            }

            other => {
                let mut seq = serializer.serialize_seq(Some(2))?;
                let id = self.core_lib_type_id();
                seq.serialize_element(&id.index().0)?;
                match other {
                    LiteralTypeDefinition::Integer(value) => {
                        seq.serialize_element(value)?;
                    }

                    LiteralTypeDefinition::Decimal(value) => {
                        seq.serialize_element(value)?;
                    }

                    LiteralTypeDefinition::TypedInteger(value) => {
                        seq.serialize_element(value)?;
                    }

                    LiteralTypeDefinition::TypedDecimal(value) => {
                        seq.serialize_element(value)?;
                    }

                    LiteralTypeDefinition::Endpoint(value) => {
                        seq.serialize_element(value)?;
                    }
                    _ => unreachable!(),
                }
                seq.end()
            }
        }
    }
}
use core::fmt;
use serde::{
    Deserialize,
    de::{self, Deserializer, Visitor},
};
use crate::values::core_values::text::Text;

impl<'de> Deserialize<'de> for LiteralTypeDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(LiteralTypeDefinitionVisitor)
    }
}

struct LiteralTypeDefinitionVisitor;

impl<'de> Visitor<'de> for LiteralTypeDefinitionVisitor {
    type Value = LiteralTypeDefinition;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a valid LiteralTypeDefinition")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(LiteralTypeDefinition::Boolean(value.into()))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(LiteralTypeDefinition::Text(Text::String(value.to_string())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(LiteralTypeDefinition::Text(value.into()))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let index: u16 = seq
            .next_element()?
            .ok_or_else(|| de::Error::custom("expected literal type id"))?;
        let type_id =
            CoreLibTypeId::try_from(CoreLibIdIndex(index)).map_err(|err| {
                de::Error::custom(format!(
                    "invalid literal type id: {:?}, error: {:?}",
                    index, err
                ))
            })?;

        deserialize_literal_for_type_id(type_id, &mut seq)
    }
}

fn deserialize_literal_for_type_id<'de, A>(
    type_id: CoreLibTypeId,
    seq: &mut A,
) -> Result<LiteralTypeDefinition, A::Error>
where
    A: SeqAccess<'de>,
{
    match type_id {
        CoreLibTypeId::Base(CoreLibBaseTypeId::Integer) => {
            let value = seq.next_element()?.ok_or_else(|| {
                de::Error::custom("expected integer literal value")
            })?;

            Ok(LiteralTypeDefinition::Integer(value))
        }

        CoreLibTypeId::Base(CoreLibBaseTypeId::Decimal) => {
            let value = seq.next_element()?.ok_or_else(|| {
                de::Error::custom("expected decimal literal value")
            })?;

            Ok(LiteralTypeDefinition::Decimal(value))
        }

        CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(variant)) => {
            let value: String = seq.next_element()?.ok_or_else(|| {
                de::Error::custom("expected typed integer literal value")
            })?;

            let value = TypedInteger::try_from_string_and_variant(
                &value, variant,
            )
            .map_err(|err| {
                de::Error::custom(format!(
                    "invalid typed integer literal for variant {:?}: {:?}",
                    variant, err
                ))
            })?;

            Ok(LiteralTypeDefinition::TypedInteger(value))
        }

        CoreLibTypeId::Variant(CoreLibVariantTypeId::Decimal(variant)) => {
            let value: String = seq.next_element()?.ok_or_else(|| {
                de::Error::custom("expected typed decimal literal value")
            })?;

            let value = TypedDecimal::try_from_string_and_variant(
                &value, variant,
            )
            .map_err(|err| {
                de::Error::custom(format!(
                    "invalid typed decimal literal for variant {:?}: {:?}",
                    variant, err
                ))
            })?;

            Ok(LiteralTypeDefinition::TypedDecimal(value))
        }

        other => Err(de::Error::custom(format!(
            "invalid LiteralTypeDefinition type id: {:?}",
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use serde_json::{self, Value, json};
    use test_case::test_case;
    fn assert_json_string_roundtrip(value: LiteralTypeDefinition) {
        let serialized = serde_json::to_string(&value).unwrap();
        let deserialized: LiteralTypeDefinition =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, value);
    }
    fn assert_json_roundtrip(value: LiteralTypeDefinition, expected: Value) {
        let serialized = serde_json::to_value(&value).unwrap();
        assert_eq!(serialized, expected);
        let deserialized: LiteralTypeDefinition =
            serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized, value);
    }

    #[test_case(LiteralTypeDefinition::Boolean(true.into()), json!(true); "boolean_true")]
    #[test_case(LiteralTypeDefinition::Boolean(false.into()), json!(false); "boolean_false")]
    #[test_case(LiteralTypeDefinition::Text("hello".into()), json!("hello"); "text")]
    #[test_case(LiteralTypeDefinition::Integer(123.into()), json!([
        CoreLibTypeId::Base(CoreLibBaseTypeId::Integer).index().0, "123"
    ]); "integer_positive")]
    #[test_case(LiteralTypeDefinition::Integer((-123).into()), json!([
        CoreLibTypeId::Base(CoreLibBaseTypeId::Integer).index().0, "-123"
    ]); "integer_negative")]
    #[test_case(LiteralTypeDefinition::Integer(0.into()), json!([
        CoreLibTypeId::Base(CoreLibBaseTypeId::Integer).index().0, "0"
    ]); "integer_zero")]
    // FIXME implement serialize for decimals correct
    // #[test_case(LiteralTypeDefinition::Decimal(123.0.into()), json!({
    //     CoreLibTypeId::Base(CoreLibBaseTypeId::Decimal).to_string(): "123.0"
    // }); "decimal_positive")]
    // #[test_case(LiteralTypeDefinition::Decimal((-123.0).into()), json!({
    //     CoreLibTypeId::Base(CoreLibBaseTypeId::Decimal).to_string(): "-123.0"
    // }); "decimal_negative")]
    // #[test_case(LiteralTypeDefinition::Decimal(0.0.into()), json!({
    //     CoreLibTypeId::Base(CoreLibBaseTypeId::Decimal).to_string(): "0.0"
    // }); "decimal_zero")]
    fn literal_type_definition_roundtrips(
        value: LiteralTypeDefinition,
        expected: Value,
    ) {
        assert_json_roundtrip(value, expected);
    }
}
