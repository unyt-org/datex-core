use crate::stdlib::vec;
use crate::stdlib::vec::Vec;

#[derive(Debug, Clone)]
pub enum NextScopeInstruction {
    /// number of regular instructions expected to follow
    Regular(u32),
    /// unknown number of regular instructions is expected to follow
    // this state must be explicitly ended when a specific end instruction is reached
    RegularUnbounded,
    /// number of type instructions expected to follow
    Type(u32),
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
    pub fn push_next_regular(&mut self, count: u32) {
        match self.0.last_mut() {
            Some(NextScopeInstruction::Regular(existing_count)) => {
                // if existing count + count overflows, push a new entry instead
                if let Some(new_count) = existing_count.checked_add(count) {
                    *existing_count = new_count;
                } else {
                    self.0.push(NextScopeInstruction::Regular(count));
                }
            }
            _ => {
                self.0.push(NextScopeInstruction::Regular(count));
            }
        }
    }
    
    pub fn push_next_regular_unbounded(&mut self) {
        self.0.push(NextScopeInstruction::RegularUnbounded);
    }

    pub fn push_next_type(&mut self, count: u32) {
        match self.0.last_mut() {
            Some(NextScopeInstruction::Type(existing_count)) => {
                // if existing count + count overflows, push a new entry instead
                if let Some(new_count) = existing_count.checked_add(count) {
                    *existing_count = new_count;
                } else {
                    self.0.push(NextScopeInstruction::Type(count));
                }
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
                NextScopeInstruction::RegularUnbounded => {
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

    /// Ends the current unbounded regular instruction scope.
    /// Returns Ok if successful, Err if the top of the stack is not an unbounded regular instruction scope.
    pub fn pop_unbounded_regular(&mut self) -> Result<(), ()> {
        let stack = &mut self.0;
        match stack.last() {
            Some(NextScopeInstruction::RegularUnbounded) => {
                stack.pop();
                Ok(())
            }
            _ => Err(())
        }
    }

    /// Returns true if there are no more instructions to process.
    pub fn is_end(&self) -> bool {
        self.0.is_empty()
    }
}