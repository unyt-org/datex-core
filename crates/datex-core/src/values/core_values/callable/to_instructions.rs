use core::ops::Deref;
use crate::{
    core_compiler::{
        to_instructions::ToInstructions, value_visitor::ValueVisitor,
    },
    instruction::{
        Instruction,
        instruction_data::{
            CallableData, CallableDataBody, CallableSignatureData,
            ShortTextData,
        },
        regular_instruction::RegularInstruction,
    },
    prelude::*,
    values::core_values::callable::{Callable, CallableBody},
};
impl ToInstructions for Callable {
    fn to_instructions<'a>(
        &'a self,
        ctx: &'a mut dyn ValueVisitor,
    ) -> Box<dyn Iterator<Item = Instruction> + 'a>
    {
        Box::new(gen move {
            let (body, injected_values) = match &self.body {
                CallableBody::DatexBytecode(datex_bytecode) => (
                    CallableDataBody {
                        injected_value_count: datex_bytecode
                            .injected_values
                            .len()
                            as u32,
                        length: datex_bytecode.body.len() as u32,
                        body: datex_bytecode.body.clone(),
                    },
                    datex_bytecode.injected_values.clone(), // FIXME avoid clone!
                ),
                _ => (
                    CallableDataBody {
                        injected_value_count: 0,
                        length: 0,
                        body: vec![],
                    },
                    vec![],
                ),
            };

            yield RegularInstruction::Callable(CallableData {
                signature: CallableSignatureData {
                    name: ShortTextData(self.name.clone().unwrap_or_default()),
                    kind: self.signature.kind,
                    requires_async: self.signature.requires_async,
                    parameter_count: self.signature.parameters.len() as u8,
                    has_rest_parameter: self.signature.rest_parameter.is_some(),
                    has_return_type: self.signature.return_type.is_some(),
                    has_yeet_type: self.signature.yeet_type.is_some(),
                    parameter_names: self
                        .signature
                        .parameters
                        .iter()
                        .map(|(name, _)| {
                            ShortTextData(name.clone().unwrap_or_default())
                        })
                        .collect(),
                    rest_parameter_name: self
                        .signature
                        .rest_parameter
                        .as_ref()
                        .map(|(name, _)| {
                            ShortTextData(name.clone().unwrap_or_default())
                        }),
                },
                body,
            })
            .into();

            // add parameter types
            for (_, param) in &self.signature.parameters {
                for instruction in param.to_instructions(ctx) {
                    yield instruction;
                }
            }
            // add rest parameter type
            if let Some((_, param)) = &self.signature.rest_parameter {
                for instruction in param.deref().to_instructions(ctx) {
                    yield instruction;
                }
            }
            // add return type
            if let Some(ty) = &self.signature.return_type {
                for instruction in ty.deref().to_instructions(ctx) {
                    yield instruction;
                }
            }
            // add yield type
            if let Some(ty) = &self.signature.yeet_type {
                for instruction in ty.deref().to_instructions(ctx) {
                    yield instruction;
                }
            }

            for value in injected_values {
                let instructions: Vec<Instruction> =
                    value.to_instructions(ctx).collect();

                for instruction in instructions {
                    yield instruction;
                }
            }
        })
    }
}
