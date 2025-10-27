use std::ops::Range;


use crate::ast::data::expression::VariableAccess;
use crate::ast::data::spanned::Spanned;
use crate::ast::data::visitor::{Visit, Visitable};
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::core_values::integer::Integer;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::pointer::PointerAddress;

#[derive(Clone, Debug, PartialEq)]
/// The different kinds of type expressions in the AST
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
    Ref(Box<TypeExpression>),
    RefMut(Box<TypeExpression>),
    RefFinal(Box<TypeExpression>),
}

impl Spanned for TypeExpressionData {
    type Output = TypeExpression;

    fn with_span<T: Into<Range<usize>>>(self, span: T) -> Self::Output {
        TypeExpression {
            data: self,
            span: span.into(),
            wrapped: None,
        }
    }

    fn with_default_span(self) -> Self::Output {
        TypeExpression {
            data: self,
            span: 0..0,
            wrapped: None,
        }
    }
}

#[derive(Clone, Debug)]
/// A type expression in the AST
pub struct TypeExpression {
    pub data: TypeExpressionData,
    pub span: Range<usize>,
    pub wrapped: Option<usize>, // number of wrapping parentheses
}

impl Visitable for TypeExpression {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        match &mut self.data {
            TypeExpressionData::GetReference(pointer_address) => {
                visitor.visit_get_reference(pointer_address, &self.span)
            }
            TypeExpressionData::Null => visitor.visit_null(&self.span),
            TypeExpressionData::VariableAccess(variable_access) => {
                visitor.visit_variable_access(variable_access, &self.span)
            }
            TypeExpressionData::Integer(integer) => {
                visitor.visit_integer(integer, &self.span)
            }
            TypeExpressionData::TypedInteger(typed_integer) => {
                visitor.visit_typed_integer(typed_integer, &self.span)
            }
            TypeExpressionData::Decimal(decimal) => {
                visitor.visit_decimal(decimal, &self.span)
            }
            TypeExpressionData::TypedDecimal(typed_decimal) => {
                visitor.visit_typed_decimal(typed_decimal, &self.span)
            }
            TypeExpressionData::Boolean(boolean) => {
                visitor.visit_boolean(boolean, &self.span)
            }
            TypeExpressionData::Text(text) => {
                visitor.visit_text(text, &self.span)
            }
            TypeExpressionData::Endpoint(endpoint) => {
                visitor.visit_endpoint(endpoint, &self.span)
            }
            TypeExpressionData::StructuralList(structual_list) => {
                visitor.visit_structural_list(structual_list, &self.span)
            }
            TypeExpressionData::FixedSizeList(fixed_size_list) => {
                visitor.visit_fixed_size_list(fixed_size_list, &self.span)
            }
            TypeExpressionData::SliceList(slice_list) => {
                visitor.visit_slice_list(slice_list, &self.span)
            }
            TypeExpressionData::Intersection(intersection) => {
                visitor.visit_intersection(intersection, &self.span)
            }
            TypeExpressionData::Union(union) => {
                visitor.visit_union(union, &self.span)
            }
            TypeExpressionData::GenericAccess(generic_access) => {
                visitor.visit_generic_access(generic_access, &self.span)
            }
            TypeExpressionData::Function(function) => {
                visitor.visit_function_type(function, &self.span)
            }
            TypeExpressionData::StructuralMap(structural_map) => {
                visitor.visit_structural_map(structural_map, &self.span)
            }
            TypeExpressionData::Ref(type_ref) => {
                visitor.visit_type_ref(type_ref, &self.span)
            }
            TypeExpressionData::RefMut(type_ref_mut) => {
                visitor.visit_type_ref_mut(type_ref_mut, &self.span)
            }
            TypeExpressionData::Literal(literal) => {
                visitor.visit_literal_type(literal, &self.span)
            }
            TypeExpressionData::RefFinal(type_ref_final) => {
                unimplemented!("RefFinal is going to be deprecated")
            }
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
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        for item in &mut self.0 {
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
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        visitor.visit_type_expression(&mut self.r#type);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SliceList(pub Box<TypeExpression>);

impl Visitable for SliceList {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        visitor.visit_type_expression(&mut self.0);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Intersection(pub Vec<TypeExpression>);

impl Visitable for Intersection {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        for item in &mut self.0 {
            visitor.visit_type_expression(item);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Union(pub Vec<TypeExpression>);
impl Visitable for Union {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        for item in &mut self.0 {
            visitor.visit_type_expression(item);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenericAccess {
    pub base: String,
    pub access: Vec<TypeExpression>,
}
impl Visitable for GenericAccess {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        for arg in &mut self.access {
            visitor.visit_type_expression(arg);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionType {
    pub parameters: Vec<(String, TypeExpression)>,
    pub return_type: Box<TypeExpression>,
}
impl Visitable for FunctionType {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        for (_, param_type) in &mut self.parameters {
            visitor.visit_type_expression(param_type);
        }
        visitor.visit_type_expression(&mut self.return_type);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructuralMap(pub Vec<(TypeExpression, TypeExpression)>);

impl Visitable for StructuralMap {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        for (key, value) in &mut self.0 {
            visitor.visit_type_expression(key);
            visitor.visit_type_expression(value);
        }
    }
}
