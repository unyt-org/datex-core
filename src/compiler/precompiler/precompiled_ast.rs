use crate::stdlib::{cell::RefCell, rc::Rc};
use crate::{
    ast::structs::expression::{DatexExpression, VariableKind},
    types::type_container::TypeContainer,
};
use core::fmt::Display;

#[derive(Clone, Debug)]
pub struct VariableMetadata {
    pub original_realm_index: usize,
    pub is_cross_realm: bool,
    pub shape: VariableShape,
    pub var_type: Option<TypeContainer>,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableShape {
    Type,
    Value(VariableKind),
}

impl From<VariableKind> for VariableShape {
    fn from(value: VariableKind) -> Self {
        VariableShape::Value(value)
    }
}

impl Display for VariableShape {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VariableShape::Type => core::write!(f, "type"),
            VariableShape::Value(kind) => core::write!(f, "{kind}"),
        }
    }
}

#[derive(Default, Debug)]
pub struct AstMetadata {
    pub variables: Vec<VariableMetadata>,
}

impl AstMetadata {
    pub fn variable_metadata(&self, id: usize) -> Option<&VariableMetadata> {
        self.variables.get(id)
    }

    pub fn variable_metadata_mut(
        &mut self,
        id: usize,
    ) -> Option<&mut VariableMetadata> {
        self.variables.get_mut(id)
    }
}

#[derive(Debug, Clone, Default)]
pub struct RichAst {
    pub ast: Option<DatexExpression>,
    pub metadata: Rc<RefCell<AstMetadata>>,
}

impl RichAst {
    pub fn new(
        ast: DatexExpression,
        metadata: &Rc<RefCell<AstMetadata>>,
    ) -> Self {
        RichAst {
            ast: Some(ast),
            metadata: metadata.clone(),
        }
    }

    pub fn new_without_metadata(ast: DatexExpression) -> Self {
        RichAst {
            ast: Some(ast),
            metadata: Rc::new(RefCell::new(AstMetadata::default())),
        }
    }
}
