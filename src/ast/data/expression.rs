use chumsky::span::SimpleSpan;

use crate::ast::assignment_operation::AssignmentOperator;
use crate::ast::binary_operation::BinaryOperator;
use crate::ast::binding::VariableId;
use crate::ast::chain::ApplyOperation;
use crate::ast::comparison_operation::ComparisonOperator;
use crate::ast::data::spanned::Spanned;
use crate::ast::data::r#type::TypeExpression;
use crate::ast::data::visitor::{Visit, Visitable};
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

#[derive(Clone, Debug)]
/// An expression in the AST
pub struct DatexExpression {
    pub data: DatexExpressionData,
    pub span: SimpleSpan,
    pub wrapped: Option<usize>, // number of wrapping parentheses
}

impl Visitable for DatexExpression {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        match &mut self.data {
            DatexExpressionData::UnaryOperation(op) => {
                visitor.visit_unary_operation(op, self.span)
            }
            DatexExpressionData::Statements(stmts) => {
                visitor.visit_statements(stmts, self.span)
            }
            DatexExpressionData::VariableDeclaration(var_decl) => {
                visitor.visit_variable_declaration(var_decl, self.span)
            }
            DatexExpressionData::VariableAssignment(var_assign) => {
                visitor.visit_variable_assignment(var_assign, self.span)
            }
            DatexExpressionData::VariableAccess(var_access) => {
                visitor.visit_variable_access(var_access, self.span)
            }
            DatexExpressionData::Integer(i) => {
                visitor.visit_integer(i, self.span)
            }
            DatexExpressionData::TypedInteger(ti) => {
                visitor.visit_typed_integer(ti, self.span)
            }
            DatexExpressionData::Decimal(d) => {
                visitor.visit_decimal(d, self.span)
            }
            DatexExpressionData::TypedDecimal(td) => {
                visitor.visit_typed_decimal(td, self.span)
            }
            DatexExpressionData::Text(s) => visitor.visit_text(s, self.span),
            DatexExpressionData::Boolean(b) => {
                visitor.visit_boolean(b, self.span)
            }
            DatexExpressionData::Endpoint(e) => {
                visitor.visit_endpoint(e, self.span)
            }
            DatexExpressionData::Null => visitor.visit_null(self.span),
            DatexExpressionData::List(list) => {
                visitor.visit_list(list, self.span)
            }
            DatexExpressionData::Map(map) => visitor.visit_map(map, self.span),
            DatexExpressionData::GetReference(pointer_address) => {
                visitor.visit_get_reference(pointer_address, self.span)
            }
            DatexExpressionData::Conditional(conditional) => {
                visitor.visit_conditional(conditional, self.span)
            }
            DatexExpressionData::TypeDeclaration(type_declaration) => {
                visitor.visit_type_declaration(type_declaration, self.span)
            }
            DatexExpressionData::TypeExpression(type_expression) => {
                visitor.visit_type_expression(type_expression)
            }
            DatexExpressionData::Type(type_expression) => {
                visitor.visit_type_expression(type_expression)
            }
            DatexExpressionData::FunctionDeclaration(function_declaration) => {
                visitor
                    .visit_function_declaration(function_declaration, self.span)
            }
            DatexExpressionData::CreateRef(datex_expression) => {
                visitor.visit_create_ref(datex_expression, self.span)
            }
            DatexExpressionData::CreateRefMut(datex_expression) => {
                visitor.visit_create_mut(datex_expression, self.span)
            }
            DatexExpressionData::Deref(deref) => {
                visitor.visit_deref(deref, self.span)
            }
            DatexExpressionData::Slot(slot) => {
                visitor.visit_slot(slot, self.span)
            }
            DatexExpressionData::SlotAssignment(slot_assignment) => {
                visitor.visit_slot_assignment(slot_assignment, self.span)
            }
            DatexExpressionData::PointerAddress(pointer_address) => {
                visitor.visit_pointer_address(pointer_address, self.span)
            }
            DatexExpressionData::BinaryOperation(binary_operation) => {
                visitor.visit_binary_operation(binary_operation, self.span)
            }
            DatexExpressionData::ComparisonOperation(comparison_operation) => {
                visitor
                    .visit_comparison_operation(comparison_operation, self.span)
            }
            DatexExpressionData::DerefAssignment(deref_assignment) => {
                visitor.visit_deref_assignment(deref_assignment, self.span)
            }
            DatexExpressionData::ApplyChain(apply_chain) => {
                visitor.visit_apply_chain(apply_chain, self.span)
            }
            DatexExpressionData::RemoteExecution(remote_execution) => {
                visitor.visit_remote_execution(remote_execution, self.span)
            }
            DatexExpressionData::CreateRefFinal(datex_expression) => {
                unimplemented!("CreateRefFinal is going to be deprecated")
            }
            DatexExpressionData::Placeholder
            | DatexExpressionData::Recover
            | DatexExpressionData::Identifier(_) => {}
        }
    }
}

// PartialEquality for DatexExpression ignores the span (allows for easier testing)
impl PartialEq for DatexExpression {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

#[derive(Clone, Debug, PartialEq)]
/// The different kinds of type expressions in the AST
pub enum DatexExpressionData {
    /// This is a marker for recovery from parse errors.
    /// We should never use this manually.
    Recover,

    /// null
    Null,
    /// Boolean (true or false)
    Boolean(bool),
    /// Text, e.g "Hello, world!"
    Text(String),
    /// Decimal, e.g 123.456789123456
    Decimal(Decimal),

    /// Typed Decimal, e.g. 123.456i8
    TypedDecimal(TypedDecimal),

    /// Integer, e.g 123456789123456789
    Integer(Integer),

    /// Typed Integer, e.g. 123i8
    TypedInteger(TypedInteger),

    /// Identifier (variable / core type usage)
    Identifier(String),

    /// Endpoint, e.g. @test_a or @test_b
    Endpoint(Endpoint),
    /// List, e.g  `[1, 2, 3, "text"]`
    List(List),
    /// Map, e.g {"xy": 2, (3): 4, xy: "xy"}
    Map(Map),
    /// One or more statements, e.g (1; 2; 3)
    Statements(Statements),
    /// reference access, e.g. &<ABCDEF>
    GetReference(PointerAddress),

    /// Conditional expression, e.g. if (true) { 1 } else { 2 }
    Conditional(Conditional),

    // TODO: Give information on type kind (nominal & structural)
    /// Variable declaration, e.g. const x = 1, const mut x = 1, or var y = 2. VariableId is always set to 0 by the ast parser.
    VariableDeclaration(VariableDeclaration),
    /// Variable assignment, e.g. x = 42 or y += 1
    VariableAssignment(VariableAssignment),
    /// Variable access - only generated by the precompiler, not by the parser
    VariableAccess(VariableAccess),

    // TODO: Shall we avoid hoisting for type aliases?
    // This would remove the ability to have recursive type
    // definitions.
    /// Type declaration, e.g. type MyType = { x: 42, y: "John" };
    TypeDeclaration(TypeDeclaration),

    /// Type expression, e.g. { x: 42, y: "John" }
    TypeExpression(TypeExpression),

    /// Type keyword, e.g. type(...)
    Type(TypeExpression),

    /// Function declaration, e.g. fn my_function() -> type ( ... )
    FunctionDeclaration(FunctionDeclaration),

    // TODO combine
    /// Reference, e.g. &x
    CreateRef(Box<DatexExpression>),
    /// Mutable reference, e.g. &mut x
    CreateRefMut(Box<DatexExpression>),
    /// Final reference, e.g. &final x
    CreateRefFinal(Box<DatexExpression>),

    /// Deref
    Deref(Box<DatexExpression>),

    /// Slot, e.g. #1, #endpoint
    Slot(Slot),

    /// Slot assignment
    SlotAssignment(SlotAssignment),

    /// Pointer address $<identifier>
    PointerAddress(PointerAddress),

    /// Binary operation, e.g. x + y
    BinaryOperation(BinaryOperation),

    /// Comparison operation, e.g. x < y
    ComparisonOperation(ComparisonOperation),

    /// Deref assignment, e.g. *x = y, **x += y
    DerefAssignment(DerefAssignment),

    /// Unary operation, e.g. -x, !x
    UnaryOperation(UnaryOperation),

    /// apply (e.g. x (1)) or property access
    ApplyChain(ApplyChain),

    /// The '?' placeholder expression
    Placeholder,

    /// Remote execution, e.g. @example :: 41 + 1
    RemoteExecution(RemoteExecution),
}

impl Spanned for DatexExpressionData {
    type Output = DatexExpression;

    fn with_span(self, span: SimpleSpan) -> Self::Output {
        DatexExpression {
            data: self,
            span,
            wrapped: None,
        }
    }

    fn with_default_span(self) -> Self::Output {
        DatexExpression {
            data: self,
            span: SimpleSpan::from(0..0),
            wrapped: None,
        }
    }
}

// directly convert DatexExpression to a ValueContainer
impl TryFrom<&DatexExpressionData> for ValueContainer {
    type Error = ();

    fn try_from(expr: &DatexExpressionData) -> Result<Self, Self::Error> {
        Ok(match expr {
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator,
                expression,
            }) => {
                let value = ValueContainer::try_from(&expression.data)?;
                match value {
                    ValueContainer::Value(Value {
                        inner: CoreValue::Integer(_) | CoreValue::Decimal(_),
                        ..
                    }) => match operator {
                        UnaryOperator::Arithmetic(
                            ArithmeticUnaryOperator::Plus,
                        ) => value,
                        UnaryOperator::Arithmetic(
                            ArithmeticUnaryOperator::Minus,
                        ) => value.neg().map_err(|_| ())?,
                        _ => Err(())?,
                    },
                    _ => Err(())?,
                }
            }
            DatexExpressionData::Null => ValueContainer::Value(Value::null()),
            DatexExpressionData::Boolean(b) => ValueContainer::from(*b),
            DatexExpressionData::Text(s) => ValueContainer::from(s.clone()),
            DatexExpressionData::Decimal(d) => ValueContainer::from(d.clone()),
            DatexExpressionData::Integer(i) => ValueContainer::from(i.clone()),
            DatexExpressionData::Endpoint(e) => ValueContainer::from(e.clone()),
            DatexExpressionData::List(list) => {
                let entries = list
                    .items
                    .iter()
                    .map(|e| ValueContainer::try_from(&e.data))
                    .collect::<Result<Vec<ValueContainer>, ()>>()?;
                ValueContainer::from(
                    datex_core::values::core_values::list::List::from(entries),
                )
            }
            DatexExpressionData::Map(pairs) => {
                let entries = pairs
                    .entries
                    .iter()
                    .map(|(k, v)| {
                        let key = ValueContainer::try_from(&k.data)?;
                        let value = ValueContainer::try_from(&v.data)?;
                        Ok((key, value))
                    })
                    .collect::<Result<Vec<(ValueContainer, ValueContainer)>, ()>>()?;
                ValueContainer::from(
                    crate::values::core_values::map::Map::from(entries),
                )
            }
            _ => Err(())?,
        })
    }
}

// Expressions with visit methods

#[derive(Clone, Debug, PartialEq)]
pub struct BinaryOperation {
    pub operator: BinaryOperator,
    pub left: Box<DatexExpression>,
    pub right: Box<DatexExpression>,
    pub r#type: Option<Type>,
}

impl Visitable for BinaryOperation {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        visitor.visit_expression(&mut self.left);
        visitor.visit_expression(&mut self.right);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComparisonOperation {
    pub operator: ComparisonOperator,
    pub left: Box<DatexExpression>,
    pub right: Box<DatexExpression>,
}

impl Visitable for ComparisonOperation {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        visitor.visit_expression(&mut self.left);
        visitor.visit_expression(&mut self.right);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DerefAssignment {
    pub operator: AssignmentOperator,
    pub deref_count: usize,
    pub deref_expression: Box<DatexExpression>,
    pub assigned_expression: Box<DatexExpression>,
}

impl Visitable for DerefAssignment {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        visitor.visit_expression(&mut self.deref_expression);
        visitor.visit_expression(&mut self.assigned_expression);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Conditional {
    pub condition: Box<DatexExpression>,
    pub then_branch: Box<DatexExpression>,
    pub else_branch: Option<Box<DatexExpression>>,
}
impl Visitable for Conditional {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        visitor.visit_expression(&mut self.condition);
        visitor.visit_expression(&mut self.then_branch);
        if let Some(else_branch) = &mut self.else_branch {
            visitor.visit_expression(else_branch);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeDeclaration {
    pub id: Option<VariableId>,
    pub name: String,
    pub value: TypeExpression,
    pub hoisted: bool,
}
impl Visitable for TypeDeclaration {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        visitor.visit_type_expression(&mut self.value);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnaryOperation {
    pub operator: UnaryOperator,
    pub expression: Box<DatexExpression>,
}
impl Visitable for UnaryOperation {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        visitor.visit_expression(&mut self.expression);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApplyChain {
    pub base: Box<DatexExpression>,
    pub operations: Vec<ApplyOperation>,
}
impl Visitable for ApplyChain {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        visitor.visit_expression(&mut self.base);
        for op in &mut self.operations {
            match op {
                ApplyOperation::FunctionCall(expression) => {
                    visitor.visit_expression(expression);
                }
                ApplyOperation::PropertyAccess(property) => {
                    visitor.visit_expression(property);
                }
                ApplyOperation::GenericAccess(access) => {
                    visitor.visit_expression(access);
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RemoteExecution {
    pub left: Box<DatexExpression>,
    pub right: Box<DatexExpression>,
}
impl Visitable for RemoteExecution {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        visitor.visit_expression(&mut self.left);
        visitor.visit_expression(&mut self.right);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Statements {
    pub statements: Vec<DatexExpression>,
    pub is_terminated: bool,
}
impl Statements {
    pub fn empty() -> Self {
        Statements {
            statements: Vec::new(),
            is_terminated: true,
        }
    }
    pub fn new_terminated(statements: Vec<DatexExpression>) -> Self {
        Statements {
            statements,
            is_terminated: true,
        }
    }
    pub fn new_unterminated(statements: Vec<DatexExpression>) -> Self {
        Statements {
            statements,
            is_terminated: false,
        }
    }
}
impl Visitable for Statements {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        for stmt in &mut self.statements {
            visitor.visit_expression(stmt);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariableDeclaration {
    pub id: Option<VariableId>,
    pub kind: VariableKind,
    pub name: String,
    pub type_annotation: Option<TypeExpression>,
    pub init_expression: Box<DatexExpression>,
}

impl Visitable for VariableDeclaration {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        visitor.visit_expression(&mut self.init_expression);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariableAssignment {
    pub id: Option<VariableId>,
    pub name: String,
    pub operator: AssignmentOperator,
    pub expression: Box<DatexExpression>,
}

impl Visitable for VariableAssignment {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        visitor.visit_expression(&mut self.expression);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariableAccess {
    pub id: VariableId,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionDeclaration {
    pub name: String,
    pub parameters: Vec<(String, TypeExpression)>,
    pub return_type: Option<TypeExpression>,
    pub body: Box<DatexExpression>,
}

impl Visitable for FunctionDeclaration {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        visitor.visit_expression(&mut self.body);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct List {
    pub items: Vec<DatexExpression>,
}

impl List {
    pub fn new(items: Vec<DatexExpression>) -> Self {
        List { items }
    }
}

impl Visitable for List {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        for item in &mut self.items {
            visitor.visit_expression(item);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    pub entries: Vec<(DatexExpression, DatexExpression)>,
}

impl Map {
    pub fn new(entries: Vec<(DatexExpression, DatexExpression)>) -> Self {
        Map { entries }
    }
}

impl Visitable for Map {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        for (key, value) in &mut self.entries {
            visitor.visit_expression(key);
            visitor.visit_expression(value);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VariableKind {
    Const,
    Var,
}

impl Display for VariableKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariableKind::Const => write!(f, "const"),
            VariableKind::Var => write!(f, "var"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Slot {
    Addressed(u32),
    Named(String),
}
impl Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#")?;
        match self {
            Slot::Addressed(addr) => write!(f, "{}", addr),
            Slot::Named(name) => write!(f, "{}", name),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SlotAssignment {
    pub slot: Slot,
    pub expression: Box<DatexExpression>,
}
impl Visitable for SlotAssignment {
    fn visit_children_with(&mut self, visitor: &mut impl Visit) {
        visitor.visit_expression(&mut self.expression);
    }
}
