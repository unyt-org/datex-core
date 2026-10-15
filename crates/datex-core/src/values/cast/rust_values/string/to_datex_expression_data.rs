use crate::{
    ast::expressions::DatexExpressionData, prelude::*,
    traits::to_datex_expression_data::ToDatexExpressionData,
    values::core_values::text::Text,
};

impl ToDatexExpressionData for &str {
    fn to_datex_expression_data(&self) -> DatexExpressionData {
        DatexExpressionData::Text(Text::String(self.to_string()))
    }
}

impl ToDatexExpressionData for String {
    fn to_datex_expression_data(&self) -> DatexExpressionData {
        DatexExpressionData::Text(Text::String(self.clone()))
    }
}
