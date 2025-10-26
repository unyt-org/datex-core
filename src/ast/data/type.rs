use chumsky::span::SimpleSpan;

use crate::ast::assignment_operation::AssignmentOperator;
use crate::ast::binary_operation::BinaryOperator;
use crate::ast::binding::VariableId;
use crate::ast::chain::ApplyOperation;
use crate::ast::comparison_operation::ComparisonOperator;
use crate::ast::data::expression::VariableAccess;
use crate::ast::data::spanned::Spanned;
use crate::ast::data::visitable::{Visit, Visitable};
use crate::ast::unary_operation::{ArithmeticUnaryOperator, UnaryOperator};
use crate::values::core_value::CoreValue;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use std::fmt::Display;
use std::ops::Neg;

#[derive(Clone, Debug, PartialEq)]
pub enum TypeExpressionData {
    Null,
    // a type name or variable, e.g. integer, string, User, MyType, T
    Literal(String),

    VariableAccess(VariableAccess),
    GetReference(PointerAddress),

    // literals
    Integer(Integer),
    TypedInteger(TypedInteger),
    Decimal(Decimal),
    TypedDecimal(TypedDecimal),
    Boolean(bool),
    Text(String),
    Endpoint(Endpoint),

    // [integer, text, endpoint]
    // size known to compile time, arbitrary types
    StructuralList(StructuralList),

    // [text; 3], integer[10]
    // fixed size and known to compile time, only one type
    FixedSizeList(FixedSizeList),

    // text[], integer[]
    // size not known to compile time, only one type
    SliceList(SliceList),

    // text & "test"
    Intersection(Intersection),

    // text | integer
    Union(Union),

    // User<text, integer>
    GenericAccess(GenericAccess),

    // (x: text) -> text
    Function(FunctionType),

    // structurally typed map, e.g. { x: integer, y: text }
    StructuralMap(StructuralMap),

    // modifiers
    Ref(Box<TypeExpressionData>),
    RefMut(Box<TypeExpressionData>),
    RefFinal(Box<TypeExpressionData>),
}

impl Spanned for TypeExpressionData {
    type Output = TypeExpression;

    fn with_span(self, span: SimpleSpan) -> Self::Output {
        TypeExpression {
            data: self,
            span,
            wrapped: None,
        }
    }

    fn with_default_span(self) -> Self::Output {
        TypeExpression {
            data: self,
            span: SimpleSpan::from(0..0),
            wrapped: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TypeExpression {
    pub data: TypeExpressionData,
    pub span: SimpleSpan,
    pub wrapped: Option<usize>, // number of wrapping parentheses
}

impl Visitable for TypeExpression {
    fn visit_children_with(&self, visitor: &mut impl Visit) {
        match &self.data {
            TypeExpressionData::GetReference(pointer_address) => {
                visitor.visit_get_reference(pointer_address, self.span)
            }
            TypeExpressionData::Null => visitor.visit_null(self.span),
            TypeExpressionData::Literal(_) => todo!(),
            TypeExpressionData::VariableAccess(variable_access) => {
                visitor.visit_variable_access(variable_access, self.span)
            }
            TypeExpressionData::Integer(integer) => {
                visitor.visit_integer(integer, self.span)
            }
            TypeExpressionData::TypedInteger(typed_integer) => {
                visitor.visit_typed_integer(typed_integer, self.span)
            }
            TypeExpressionData::Decimal(decimal) => {
                visitor.visit_decimal(decimal, self.span)
            }
            TypeExpressionData::TypedDecimal(typed_decimal) => {
                visitor.visit_typed_decimal(typed_decimal, self.span)
            }
            TypeExpressionData::Boolean(boolean) => {
                visitor.visit_boolean(*boolean, self.span)
            }
            TypeExpressionData::Text(text) => {
                visitor.visit_text(text, self.span)
            }
            TypeExpressionData::Endpoint(endpoint) => {
                visitor.visit_endpoint(endpoint, self.span)
            }
            TypeExpressionData::StructuralList(type_expression_datas) => {
                todo!()
            }
            TypeExpressionData::FixedSizeList(fixed_size_list) => {
                todo!()
            }
            TypeExpressionData::SliceList(type_expression_data) => todo!(),
            TypeExpressionData::Intersection(type_expression_datas) => todo!(),
            TypeExpressionData::Union(type_expression_datas) => todo!(),
            TypeExpressionData::GenericAccess(generic_access) => {
                todo!()
            }
            TypeExpressionData::Function(function) => todo!(),
            TypeExpressionData::StructuralMap(items) => todo!(),
            TypeExpressionData::Ref(type_expression_data) => todo!(),
            TypeExpressionData::RefMut(type_expression_data) => todo!(),
            TypeExpressionData::RefFinal(type_expression_data) => todo!(),
        }
    }
}

impl PartialEq for TypeExpression {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructuralList(pub Vec<TypeExpression>);

impl Visitable for StructuralList {
    fn visit_children_with(&self, visitor: &mut impl Visit) {
        for item in &self.0 {
            visitor.visit_type_expression(item);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FixedSizeList {
    pub r#type: Box<TypeExpression>,
    pub size: usize,
}
impl Visitable for FixedSizeList {
    fn visit_children_with(&self, visitor: &mut impl Visit) {
        visitor.visit_type_expression(&self.r#type);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SliceList(Box<TypeExpression>);

impl Visitable for SliceList {
    fn visit_children_with(&self, visitor: &mut impl Visit) {
        visitor.visit_type_expression(&self.0);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Intersection(pub Vec<TypeExpression>);

impl Visitable for Intersection {
    fn visit_children_with(&self, visitor: &mut impl Visit) {
        for item in &self.0 {
            visitor.visit_type_expression(item);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Union(pub Vec<TypeExpression>);
impl Visitable for Union {
    fn visit_children_with(&self, visitor: &mut impl Visit) {
        for item in &self.0 {
            visitor.visit_type_expression(item);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenericAccess {
    pub base: String,
    pub access: Box<TypeExpression>,
}
impl Visitable for GenericAccess {
    fn visit_children_with(&self, visitor: &mut impl Visit) {
        visitor.visit_type_expression(&self.access);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionType {
    pub parameters: Vec<(String, TypeExpression)>,
    pub return_type: Box<TypeExpression>,
}
impl Visitable for FunctionType {
    fn visit_children_with(&self, visitor: &mut impl Visit) {
        for (_, param_type) in &self.parameters {
            visitor.visit_type_expression(param_type);
        }
        visitor.visit_type_expression(&self.return_type);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructuralMap(pub Vec<(TypeExpression, TypeExpression)>);

impl Visitable for StructuralMap {
    fn visit_children_with(&self, visitor: &mut impl Visit) {
        for (key, value) in &self.0 {
            visitor.visit_type_expression(key);
            visitor.visit_type_expression(value);
        }
    }
}
