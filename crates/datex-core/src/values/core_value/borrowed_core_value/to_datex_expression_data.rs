use crate::{
    ast::expressions::DatexExpressionData,
    traits::to_datex_expression_data::ToDatexExpressionData,
    values::core_value::borrowed_core_value::BorrowedCoreValue,
};

impl<'a> ToDatexExpressionData for BorrowedCoreValue<'a> {
    fn to_datex_expression_data(&self) -> DatexExpressionData {
        match &self {
            BorrowedCoreValue::Integer(integer) => {
                integer.to_datex_expression_data()
            }
            BorrowedCoreValue::TypedInteger(typed_integer) => {
                typed_integer.to_datex_expression_data()
            }
            BorrowedCoreValue::Decimal(decimal) => {
                decimal.to_datex_expression_data()
            }
            BorrowedCoreValue::TypedDecimal(typed_decimal) => {
                typed_decimal.to_datex_expression_data()
            }
            BorrowedCoreValue::Boolean(boolean) => {
                boolean.to_datex_expression_data()
            }
            BorrowedCoreValue::Text(text) => {
                text.as_ref().to_datex_expression_data()
            }
            BorrowedCoreValue::Range(range) => range.to_datex_expression_data(),
            BorrowedCoreValue::Endpoint(endpoint) => {
                endpoint.to_datex_expression_data()
            }
            BorrowedCoreValue::Null => DatexExpressionData::Null,
            BorrowedCoreValue::List(list) => list.to_datex_expression_data(),
            BorrowedCoreValue::Map(map) => map.to_datex_expression_data(),
            BorrowedCoreValue::Type(type_value) => {
                type_value.to_datex_expression_data()
            }
            BorrowedCoreValue::Callable(callable) => {
                callable.to_datex_expression_data()
            }
            BorrowedCoreValue::EntityTypeDefinition(entity_type_definition) => {
                entity_type_definition.to_datex_expression_data()
            }
            BorrowedCoreValue::Uninitialized => todo!(),
            BorrowedCoreValue::Box(inner) => inner.to_datex_expression_data(),
            BorrowedCoreValue::Instant(instant) => {
                instant.to_datex_expression_data()
            }
        }
    }
}
