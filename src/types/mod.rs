use crate::values::value_container::ValueContainer;

pub struct TypeIdentifier {
    pub name: String,
    pub path: Option<String>
}

// New type implementation based on ValueContainer
// The TypeNew struct is only a helper struct that is used by the runtime or compiler.
// The actual type is fully represented by the ValueContainer definition.
pub struct TypeNew {
    /// Optional identifier for the type, if this is not a structural type, but a nominal type.
    pub identifier: Option<TypeIdentifier>,
    /// Value container that defines the type.
    pub definition: ValueContainer,
}

impl TryFrom<ValueContainer> for TypeNew {
    type Error = String;

    // TODO: for now, we accept any ValueContainer as a TypeNew. This might be restricted later.
    // For example, mutable references should not be allowed as types.
    fn try_from(value: ValueContainer) -> Result<Self, Self::Error> {
        Ok(TypeNew {
            identifier: None,
            definition: value,
        })
    }
}

impl TypeNew {
    /// converts a specific type (e.g. 42) to its base type (e.g. integer)
    pub fn get_base_type(&self) -> TypeNew {
        match &self.definition {
            _ => todo!(),
        }
    }
}