use crate::ast::assignment_operation::AssignmentOperator;
use crate::ast::binary_operation::BinaryOperator;
use crate::ast::binding::VariableId;
use crate::ast::chain::ApplyOperation;
use crate::ast::comparison_operation::ComparisonOperator;
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

pub use chumsky::prelude::SimpleSpan;

pub trait Visitable {
    fn visit_children_with(&self, visitor: &mut impl Visit);
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
pub enum TypeExpression {
    Null,
    // a type name or variable, e.g. integer, string, User, MyType, T
    Literal(String),

    Variable(VariableId, String),
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
    StructuralList(Vec<TypeExpression>),

    // [text; 3], integer[10]
    // fixed size and known to compile time, only one type
    FixedSizeList(Box<TypeExpression>, usize),

    // text[], integer[]
    // size not known to compile time, only one type
    SliceList(Box<TypeExpression>),

    // text & "test"
    Intersection(Vec<TypeExpression>),

    // text | integer
    Union(Vec<TypeExpression>),

    // User<text, integer>
    Generic(String, Vec<TypeExpression>),

    // (x: text) -> text
    Function {
        parameters: Vec<(String, TypeExpression)>,
        return_type: Box<TypeExpression>,
    },

    // structurally typed map, e.g. { x: integer, y: text }
    StructuralMap(Vec<(TypeExpression, TypeExpression)>),

    // modifiers
    Ref(Box<TypeExpression>),
    RefMut(Box<TypeExpression>),
    RefFinal(Box<TypeExpression>),
}

#[derive(Clone, Debug)]
pub struct DatexExpression {
    pub data: DatexExpressionData,
    pub span: SimpleSpan,
    pub wrapped: Option<usize>, // number of wrapping parentheses
}

impl Visitable for DatexExpression {
    fn visit_children_with(&self, visitor: &mut impl Visit) {
        match &self.data {
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
                visitor.visit_boolean(*b, self.span)
            }
            DatexExpressionData::Endpoint(e) => {
                visitor.visit_endpoint(e, self.span)
            }
            DatexExpressionData::Null => visitor.visit_null(self.span),
            DatexExpressionData::List(list) => {
                visitor.visit_list(list, self.span)
            }
            DatexExpressionData::Map(map) => visitor.visit_map(map, self.span),
            _ => {}
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
    Conditional {
        condition: Box<DatexExpression>,
        then_branch: Box<DatexExpression>,
        else_branch: Option<Box<DatexExpression>>,
    },

    // TODO #465: Give information on type kind (nominal & structural)
    /// Variable declaration, e.g. const x = 1, const mut x = 1, or var y = 2. VariableId is always set to 0 by the ast parser.
    VariableDeclaration(VariableDeclaration),
    /// Variable assignment, e.g. x = 42 or y += 1
    VariableAssignment(VariableAssignment),
    /// Variable access - only generated by the precompiler, not by the parser
    VariableAccess(VariableAccess),

    // TODO #466: Shall we avoid hoisting for type aliases?
    // This would remove the ability to have recursive type
    // definitions.
    /// Type declaration, e.g. type MyType = { x: 42, y: "John" };
    TypeDeclaration {
        id: Option<VariableId>,
        name: String,
        value: TypeExpression, // Type
        hoisted: bool,
    },

    /// Type expression, e.g. { x: 42, y: "John" }
    TypeExpression(TypeExpression),

    /// Type keyword, e.g. type(...)
    Type(TypeExpression),

    FunctionDeclaration {
        name: String,
        parameters: Vec<(String, TypeExpression)>,
        return_type: Option<TypeExpression>,
        body: Box<DatexExpression>,
    },

    // TODO #467 combine
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
    SlotAssignment(Slot, Box<DatexExpression>),

    PointerAddress(PointerAddress),

    // TODO #468 struct instead of tuple
    BinaryOperation(
        BinaryOperator,
        Box<DatexExpression>,
        Box<DatexExpression>,
        Option<Type>,
    ),
    ComparisonOperation(
        ComparisonOperator,
        Box<DatexExpression>,
        Box<DatexExpression>,
    ),
    DerefAssignment {
        operator: AssignmentOperator,
        deref_count: usize,
        deref_expression: Box<DatexExpression>,
        assigned_expression: Box<DatexExpression>,
    },
    UnaryOperation(UnaryOperation),

    // apply (e.g. x (1)) or property access
    ApplyChain(Box<DatexExpression>, Vec<ApplyOperation>),

    // ?
    Placeholder,
    // @xy :: z
    RemoteExecution(Box<DatexExpression>, Box<DatexExpression>),
}

// Expressions with visit methods

#[derive(Clone, Debug, PartialEq)]
pub struct UnaryOperation {
    pub operator: UnaryOperator,
    pub expression: Box<DatexExpression>,
}
impl Visitable for UnaryOperation {
    fn visit_children_with(&self, visitor: &mut impl Visit) {
        visitor.visit_expression(&self.expression);
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
    fn visit_children_with(&self, visitor: &mut impl Visit) {
        for stmt in &self.statements {
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

// TODO #469: visitor for type expressions
impl Visitable for VariableDeclaration {
    fn visit_children_with(&self, visitor: &mut impl Visit) {
        visitor.visit_expression(&self.init_expression);
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
    fn visit_children_with(&self, visitor: &mut impl Visit) {
        visitor.visit_expression(&self.expression);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariableAccess {
    pub id: VariableId,
    pub name: String,
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
    fn visit_children_with(&self, visitor: &mut impl Visit) {
        for item in &self.items {
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
    fn visit_children_with(&self, visitor: &mut impl Visit) {
        for (key, value) in &self.entries {
            visitor.visit_expression(key);
            visitor.visit_expression(value);
        }
    }
}

// TODO #470: implement Visitable for all expressions with children

impl DatexExpressionData {
    pub(crate) fn with_span(self, span: SimpleSpan) -> DatexExpression {
        DatexExpression {
            data: self,
            span,
            wrapped: None,
        }
    }

    pub(crate) fn with_default_span(self) -> DatexExpression {
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

/// Visitor pattern for traversing the AST
/// Implement the `Visit` trait and override the methods for the nodes you want to visit.
/// The default implementation visits all child nodes and traverses the entire tree.
pub trait Visit: Sized {
    fn visit_expression(&mut self, expr: &DatexExpression) {
        expr.visit_children_with(self);
    }
    fn visit_statements(&mut self, stmts: &Statements, span: SimpleSpan) {
        stmts.visit_children_with(self);
    }
    fn visit_unary_operation(&mut self, op: &UnaryOperation, span: SimpleSpan) {
        op.visit_children_with(self);
    }
    fn visit_variable_declaration(
        &mut self,
        var_decl: &VariableDeclaration,
        span: SimpleSpan,
    ) {
        var_decl.visit_children_with(self);
    }
    fn visit_variable_assignment(
        &mut self,
        var_assign: &VariableAssignment,
        span: SimpleSpan,
    ) {
        var_assign.visit_children_with(self);
    }
    fn visit_variable_access(
        &mut self,
        var_access: &VariableAccess,
        span: SimpleSpan,
    ) {
    }
    fn visit_list(&mut self, list: &List, span: SimpleSpan) {
        list.visit_children_with(self);
    }
    fn visit_map(&mut self, map: &Map, span: SimpleSpan) {
        map.visit_children_with(self);
    }
    fn visit_integer(&mut self, value: &Integer, span: SimpleSpan) {}
    fn visit_typed_integer(&mut self, value: &TypedInteger, span: SimpleSpan) {}
    fn visit_decimal(&mut self, value: &Decimal, span: SimpleSpan) {}
    fn visit_typed_decimal(&mut self, value: &TypedDecimal, span: SimpleSpan) {}
    fn visit_text(&mut self, value: &String, span: SimpleSpan) {}
    fn visit_boolean(&mut self, value: bool, span: SimpleSpan) {}
    fn visit_endpoint(&mut self, value: &Endpoint, span: SimpleSpan) {}
    fn visit_null(&mut self, span: SimpleSpan) {}
}
