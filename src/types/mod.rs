use crate::values::value_container::ValueContainer;

pub struct TypeName {
    pub tag: String,
    pub path: Option<String>
}

// New type implementation based on ValueContainer
// The TypeNew struct is only a helper struct that is used by the runtime or compiler.
// The actual type is fully represented by the ValueContainer definition.
pub struct TypeNew {
    pub name: Option<TypeName>,
    pub definition: ValueContainer,
}

impl TryFrom<ValueContainer> for TypeNew {
    type Error = String;

    fn try_from(value: ValueContainer) -> Result<Self, Self::Error> {
        // for now, we accept any ValueContainer as a TypeNew. This might be restricted later.
        Ok(TypeNew {
            name: None,
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