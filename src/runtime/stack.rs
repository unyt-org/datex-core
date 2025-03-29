use log::info;

use crate::datex_values::{Error, PrimitiveValue, Value, ValueResult};

pub struct Stack {
    stack: Vec<Box<dyn Value>>,
}

impl Stack {
    pub fn new() -> Stack {
        Stack { stack: Vec::new() }
    }

    // custom stack operations

    pub fn print(&mut self) {
        info!("[CURRENT STACK]");
        for item in &self.stack {
            info!("{}", &item.to_string())
        }
    }

    pub fn size(&mut self) -> usize {
        self.stack.len()
    }

    pub fn push(&mut self, value: Box<dyn Value>) {
        self.stack.push(value)
    }

    pub fn pop(&mut self) -> ValueResult {
        let value = self.stack.pop();
        if value.is_some() {
            Ok(value.unwrap())
        } else {
            Err(Error {
                message: "stack error".to_string(),
            })
        }
    }

    pub fn pop_or_void(&mut self) -> Box<dyn Value> {
        let value = self.stack.pop();
        if value.is_some() {
            value.unwrap()
        } else {
            Box::new(PrimitiveValue::Void)
        }
    }
}
