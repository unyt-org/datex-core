use std::collections::HashMap;

use crate::values::core_values::r#type::{path::TypePath, r#type::Type};

#[derive(Debug, Default)]
pub struct TypeRegistry {
    types: HashMap<TypePath, Type>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRegistryError {
    TypeAlreadyExists(TypePath),
    TypeNotFound(TypePath),
    BaseTypeNotFound(TypePath),
}

impl TypeRegistry {
    pub fn insert(&mut self, ty: Type) -> Result<(), TypeRegistryError> {
        if ty.base_type.is_some() {
            // If the type has a base type, ensure that the base type exists in the registry
            if let Some(base) = &ty.base_type
                && !self.types.contains_key(base)
            {
                return Err(TypeRegistryError::BaseTypeNotFound(base.clone()));
            }
        }

        // Check that the descriptor not a reference
        // if ty.descriptor.is_reference() {
        // 	return Err(TypeRegistryError::TypeAlreadyExists(ty.name.clone()));
        // }

        // Check if the type already exists in the registry
        if self.types.contains_key(&ty.name) {
            return Err(TypeRegistryError::TypeAlreadyExists(ty.name.clone()));
        }
        self.types.insert(ty.name.clone(), ty);
        Ok(())
    }

    pub fn get(&self, name: &TypePath) -> Result<&Type, TypeRegistryError> {
        self.types
            .get(name)
            .ok_or(TypeRegistryError::TypeNotFound(name.clone()))
    }
}

#[cfg(test)]
mod tests {
    use crate::values::{
        core_values::r#type::{
            descriptor::TypeDescriptor, path::TypePath, r#type::Type,
        },
        datex_type::CoreValueType,
    };

    #[test]
    fn test_register() {
        let mut registry = super::TypeRegistry::default();

        // variant
        let type_path = TypePath::parse("std:integer");
        let type_descriptor = TypeDescriptor::Core(CoreValueType::Integer);
        let ty = Type::new(type_path.clone(), type_descriptor);
        registry.insert(ty.clone()).unwrap();
        assert_eq!(registry.get(&ty.name).unwrap(), &ty);

        // sub variant
        let type_path_sub = TypePath::parse("std:integer/u8");
        let type_descriptor_sub = TypeDescriptor::Core(CoreValueType::Integer);
        let ty_sub = Type::new_with_base(
            type_path_sub,
            type_descriptor_sub,
            type_path.clone(),
        );
        registry.insert(ty_sub.clone()).unwrap();
        assert_eq!(registry.get(&ty_sub.name).unwrap(), &ty_sub);

        // parent type
        assert!(ty.is_parent_type_of(&ty_sub));
        assert!(!ty_sub.is_parent_type_of(&ty));
        assert!(ty_sub.is_subtype_of(&ty));
        assert!(!ty.is_subtype_of(&ty_sub));

        // duplicate insert
        let result = registry.insert(ty.clone());
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            super::TypeRegistryError::TypeAlreadyExists(type_path.clone())
        );

        // get non-existing type
        let non_existing_type_path = TypePath::parse("std:non_existing");
        let result = registry.get(&non_existing_type_path);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            super::TypeRegistryError::TypeNotFound(non_existing_type_path)
        );
    }

    #[test]
    fn test_base_type_not_found() {
        let mut registry = super::TypeRegistry::default();
        let base_type_path = TypePath::parse("my:non_existing_base");
        let type_path = TypePath::parse("my:my_new_type/variant");
        let type_descriptor_base = TypeDescriptor::Core(CoreValueType::Integer);
        let ty_base = Type::new_with_base(
            type_path.clone(),
            type_descriptor_base,
            base_type_path.clone(),
        );
        let result = registry.insert(ty_base.clone());
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            super::TypeRegistryError::BaseTypeNotFound(base_type_path)
        );
    }
}
