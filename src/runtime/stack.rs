use log::info;
use crate::datex_values::value_container::ValueContainer;
use crate::runtime::execution::ExecutionError;

#[derive(Debug, Default, Clone)]
pub struct Stack {
    stack: Vec<ValueContainer>,
}

impl Stack {
    // custom stack operations

    pub fn print(&mut self) {
        info!("[CURRENT STACK]");
        for item in &self.stack {
            info!("{:?}", &item)
        }
    }

    pub fn size(&mut self) -> usize {
        self.stack.len()
    }

    pub fn push(&mut self, value: ValueContainer) {
        self.stack.push(value)
    }

    pub fn pop(&mut self) -> Result<ValueContainer, ExecutionError> {
        let value = self.stack.pop();
        if let Some(value) = value {
            Ok(value)
        }
        else {
            Err(ExecutionError::Unknown)
        }
    }

    pub fn pop_or_void(&mut self) -> ValueContainer {
        let value = self.stack.pop();
        if let Some(value) = value {
            value
        } else {
            ValueContainer::from(false) // TODO: return void
        }
    }
}
