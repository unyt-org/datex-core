use crate::compiler::ast_parser::{
    parse, DatexExpression, DatexScriptParser, TupleEntry, VariableType,
};
use crate::compiler::CompilerError;
use crate::datex_values::core_value::CoreValue;
use crate::datex_values::core_values::decimal::decimal::Decimal;
use crate::datex_values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::datex_values::core_values::endpoint::Endpoint;
use crate::datex_values::core_values::integer::integer::Integer;
use crate::datex_values::core_values::integer::typed_integer::TypedInteger;
use crate::datex_values::core_values::integer::utils::smallest_fitting_signed;
use crate::datex_values::value::Value;
use crate::datex_values::value_container::ValueContainer;
use crate::global::binary_codes::InstructionCode;
use crate::utils::buffers::{
    append_f32, append_f64, append_i128, append_i16, append_i32, append_i64,
    append_i8, append_u128, append_u32, append_u8,
};
use binrw::BinWrite;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::Cursor;

#[derive(Debug, Clone, Default)]
pub struct Scope {
    /// List of variables, mapped by name to their slot address and type.
    variables: HashMap<String, (u32, VariableType)>,
    parent_scope: Option<Box<Scope>>,
    next_slot_address: u32,
}

impl Scope {
    pub fn register_variable_slot(
        &mut self,
        slot_address: u32,
        variable_type: VariableType,
        name: String,
    ) {
        self.variables
            .insert(name.clone(), (slot_address, variable_type));
    }

    pub fn get_next_variable_slot(&mut self) -> u32 {
        let slot_address = self.next_slot_address;
        self.next_slot_address += 1;
        slot_address
    }

    pub fn resolve_variable_slot(
        &self,
        name: &str,
    ) -> Option<(u32, VariableType)> {
        let mut variables = &self.variables;
        loop {
            if let Some(slot) = variables.get(name) {
                return Some(slot.clone());
            }
            if let Some(parent) = &self.parent_scope {
                variables = &parent.variables;
            } else {
                return None; // variable not found in this scope or any parent scope
            }
        }
    }

    /// Creates a new `CompileScope` that is a child of the current scope.
    pub fn push(self) -> Scope {
        Scope {
            next_slot_address: self.next_slot_address,
            parent_scope: Some(Box::new(self)),
            variables: HashMap::new(),
        }
    }

    /// Drops the current scope and returns to the parent scope and a list
    /// of all slot addresses that should be dropped.
    pub fn pop(self) -> Option<(Scope, Vec<u32>)> {
        if let Some(mut parent) = self.parent_scope {
            // update next_slot_address for parent scope
            parent.next_slot_address = self.next_slot_address;
            Some((
                *parent,
                self.variables.keys().map(|k| self.variables[k].0).collect(),
            ))
        } else {
            None
        }
    }
}
