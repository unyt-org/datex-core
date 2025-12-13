use crate::stdlib::vec;
use crate::stdlib::vec::Vec;

#[derive(Debug, Clone)]
pub enum NextScopeInstruction {
    Regular(u64),
    Type(u64),
}

pub enum NextInstructionType {
    Regular,
    Type,
    End,
}

#[derive(Debug, Clone)]
pub struct NextInstructionsStack(Vec<NextScopeInstruction>);

impl Default for NextInstructionsStack {
    fn default() -> Self {
        NextInstructionsStack(vec![NextScopeInstruction::Regular(1)])
    }
}

impl NextInstructionsStack {

    /// Indicate that the next `count` instructions are regular instructions.
    pub fn push_next_regular(&mut self, count: u64) {
        match self.0.last_mut() {
            Some(NextScopeInstruction::Regular(existing_count)) => {
                *existing_count += count as u64;
            }
            _ => {
                self.0.push(NextScopeInstruction::Regular(count));
            }
        }
    }

    pub fn push_next_type(&mut self, count: u64) {
        match self.0.last_mut() {
            Some(NextScopeInstruction::Type(existing_count)) => {
                *existing_count += count as u64;
            }
            _ => {
                self.0.push(NextScopeInstruction::Type(count));
            }
        }
    }

    /// Returns the type of the next instruction to be processed, or None if there are no more instructions.
    pub fn pop(&mut self) -> NextInstructionType {
        let stack = &mut self.0;
        if let Some(top) = stack.last_mut() {
            match top {
                NextScopeInstruction::Regular(count) => {
                    if *count > 1 {
                        *count -= 1;
                    }
                    else {
                        stack.pop();
                    }
                    NextInstructionType::Regular
                }
                NextScopeInstruction::Type(count) => {
                    if *count > 1 {
                        *count -= 1;
                    } else {
                        stack.pop();
                    }
                    NextInstructionType::Type
                }
            }
        } else {
            NextInstructionType::End
        }
    }
}