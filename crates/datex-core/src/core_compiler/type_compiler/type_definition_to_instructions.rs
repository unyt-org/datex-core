use crate::{
    core_compiler::{
        to_instructions::{ToInstructions, yield_all},
        value_visitor::ValueVisitor,
    },
    instruction::{
        Instruction,
        instruction_data::{
            CallableSignatureData, ImplMarkersData, IntersectionData, ListData,
            ListSliceCollectionData, MapData, ShortTextData, TaggedTypeData,
            UnionData,
        },
        type_instruction::TypeInstruction,
    },
    prelude::*,
    types::{
        type_definition::{
            TypeDefinition,
            callable::CallableTypeDefinition,
            collection::{
                CollectionTypeDefinition,
                type_definition::{
                    list::ListCollectionTypeDefinition,
                    list_slice::ListSliceCollectionTypeDefinition,
                    map::MapCollectionTypeDefinition,
                },
            },
            impl_type::ImplMarkers,
            intersection::IntersectionTypeDefinition,
            list::ListTypeDefinition,
            map::MapTypeDefinition,
            range::RangeTypeDefinition,
            tagged_type::TaggedTypeDefinition,
            union::UnionTypeDefinition,
        },
        type_definition_with_metadata::TypeDefinitionWithMetadata,
    },
};
use core::ops::Deref;

impl ToInstructions for TypeDefinition {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            match self {
                TypeDefinition::ImplMarkers(impl_type_def) => {
                    yield_all!(impl_type_def.to_bytecode_item(visitor));
                }
                TypeDefinition::Literal(literal_type_definition) => {
                    yield TypeInstruction::Literal(
                        literal_type_definition.clone(),
                    )
                    .into();
                }
                TypeDefinition::List(list_type_definition) => {
                    yield_all!(list_type_definition.to_bytecode_item(visitor));
                }
                TypeDefinition::Map(map_type_definition) => {
                    yield_all!(map_type_definition.to_bytecode_item(visitor));
                }
                TypeDefinition::Range(range_type_definition) => {
                    yield_all!(range_type_definition.to_bytecode_item(visitor));
                }
                TypeDefinition::Collection(collection_type_definition) => {
                    yield_all!(
                        collection_type_definition.to_bytecode_item(visitor)
                    );
                }
                TypeDefinition::Shared(shared_container_containing_type) => {
                    yield_all!(
                        shared_container_containing_type
                            .to_bytecode_item(visitor)
                    );
                }
                TypeDefinition::Box(nested) => {
                    yield TypeInstruction::Boxed.into();
                    yield_all!(nested.deref().to_bytecode_item(visitor));
                }
                TypeDefinition::Callable(callable_type_definition) => {
                    yield_all!(
                        callable_type_definition.to_bytecode_item(visitor)
                    );
                }
                TypeDefinition::Intersection(intersection_type_definition) => {
                    yield_all!(
                        intersection_type_definition.to_bytecode_item(visitor)
                    );
                }
                TypeDefinition::Union(union_type_definition) => {
                    yield_all!(union_type_definition.to_bytecode_item(visitor));
                }
                TypeDefinition::TaggedType(tagged_type_definition) => {
                    yield_all!(tagged_type_definition.to_bytecode_item(visitor));
                }
                TypeDefinition::CoreType(core_lib_type_id) => {
                    yield TypeInstruction::CoreType(*core_lib_type_id).into();
                }
            }
        })
    }
}

impl ToInstructions for TypeDefinitionWithMetadata {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen {
            yield TypeInstruction::DefinitionWithMetadata(self.metadata).into();
            yield_all!(self.definition.to_bytecode_item(visitor))
        })
    }
}
impl ToInstructions for ImplMarkers {
    fn to_bytecode_item<'a>(
        &'a self,
        _visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen {
            yield TypeInstruction::ImplMarkers(ImplMarkersData {
                impl_count: self.impl_markers.len() as u8,
                impls: self.impl_markers.to_vec(),
            })
            .into();
        })
    }
}

impl ToInstructions for ListTypeDefinition {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            yield TypeInstruction::List(ListData {
                element_count: self.len() as u32,
            })
            .into();
            for ty in self.iter() {
                yield_all!(ty.to_bytecode_item(visitor));
            }
        })
    }
}

impl ToInstructions for MapTypeDefinition {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            yield TypeInstruction::Map(MapData {
                element_count: self.len() as u32,
            })
            .into();
            for (key_ty, value_ty) in self.iter() {
                yield_all!(key_ty.to_bytecode_item(visitor));
                yield_all!(value_ty.to_bytecode_item(visitor));
            }
        })
    }
}

impl ToInstructions for RangeTypeDefinition {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            yield TypeInstruction::Range.into();
            yield_all!(self.start.deref().to_bytecode_item(visitor));
            yield_all!(self.end.deref().to_bytecode_item(visitor));
        })
    }
}

impl ToInstructions for CollectionTypeDefinition {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            match self {
                CollectionTypeDefinition::List(list) => {
                    yield_all!(list.to_bytecode_item(visitor));
                }
                CollectionTypeDefinition::Map(map) => {
                    yield_all!(map.to_bytecode_item(visitor));
                }
                CollectionTypeDefinition::ListSlice(list) => {
                    yield_all!(list.to_bytecode_item(visitor));
                }
                CollectionTypeDefinition::Range(range) => {
                    yield_all!(range.to_bytecode_item(visitor));
                }
            }
        })
    }
}

impl ToInstructions for ListCollectionTypeDefinition {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen {
            yield TypeInstruction::ListCollection.into();
            yield_all!(self.0.deref().to_bytecode_item(visitor))
        })
    }
}
impl ToInstructions for MapCollectionTypeDefinition {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            yield TypeInstruction::MapCollection.into();
            for instruction in self
                .key_type
                .deref()
                .to_bytecode_item(visitor)
                .collect::<Vec<_>>()
            {
                yield instruction;
            }
            for instruction in self
                .value_type
                .deref()
                .to_bytecode_item(visitor)
                .collect::<Vec<_>>()
            {
                yield instruction;
            }
        })
    }
}

impl ToInstructions for ListSliceCollectionTypeDefinition {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            yield TypeInstruction::ListSliceCollection(
                ListSliceCollectionData {
                    element_count: self.size as u32,
                },
            )
            .into();
            for instruction in self
                .item_type
                .deref()
                .to_bytecode_item(visitor)
                .collect::<Vec<_>>()
            {
                yield instruction;
            }
        })
    }
}

impl ToInstructions for IntersectionTypeDefinition {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            yield TypeInstruction::Intersection(IntersectionData {
                element_count: self.len() as u32,
            })
            .into();
            for ty in self.iter() {
                for instruction in
                    ty.to_bytecode_item(visitor).collect::<Vec<_>>()
                {
                    yield instruction;
                }
            }
        })
    }
}

impl ToInstructions for CallableTypeDefinition {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            yield TypeInstruction::Callable(CallableSignatureData {
                name: ShortTextData::new("".to_string()),
                kind: self.kind,
                parameter_count: self.parameters.len() as u8,
                requires_async: self.requires_async,
                has_rest_parameter: self.rest_parameter.is_some(),
                has_return_type: self.return_type.is_some(),
                has_yeet_type: self.yeet_type.is_some(),
                parameter_names: self
                    .parameters
                    .iter()
                    .map(|(name, _)| {
                        ShortTextData::new(name.clone().unwrap_or_default())
                    })
                    .collect(),
                rest_parameter_name: self.rest_parameter.as_ref().map(
                    |(name, _)| {
                        ShortTextData::new(name.clone().unwrap_or_default())
                    },
                ),
            })
            .into();
            for (_, ty) in &self.parameters {
                for instruction in
                    ty.to_bytecode_item(visitor).collect::<Vec<_>>()
                {
                    yield instruction;
                }
            }
            if let Some((_, rest_type)) = &self.rest_parameter {
                for instruction in rest_type
                    .deref()
                    .to_bytecode_item(visitor)
                    .collect::<Vec<_>>()
                {
                    yield instruction;
                }
            }
            if let Some(return_type) = &self.return_type {
                for instruction in return_type
                    .deref()
                    .to_bytecode_item(visitor)
                    .collect::<Vec<_>>()
                {
                    yield instruction;
                }
            }
            if let Some(yeet_type) = &self.yeet_type {
                for instruction in yeet_type
                    .deref()
                    .to_bytecode_item(visitor)
                    .collect::<Vec<_>>()
                {
                    yield instruction;
                }
            }
        })
    }
}

impl ToInstructions for UnionTypeDefinition {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            yield TypeInstruction::Union(UnionData {
                element_count: self.len() as u32,
            })
            .into();
            for ty in self.iter() {
                for instruction in
                    ty.to_bytecode_item(visitor).collect::<Vec<_>>()
                {
                    yield instruction;
                }
            }
        })
    }
}

impl ToInstructions for TaggedTypeDefinition {
    fn to_bytecode_item<'a>(
        &'a self,
        visitor: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = crate::instruction::BytecodeItem> + 'a> {
        Box::new(gen move {
            yield TypeInstruction::TaggedType(TaggedTypeData::new(
                self.tag.clone(),
                self.ty.is_some(),
            ))
            .into();
            if let Some(ty) = &self.ty {
                for instruction in
                    ty.deref().to_bytecode_item(visitor).collect::<Vec<_>>()
                {
                    yield instruction;
                }
            }
        })
    }
}
