use crate::runtime::execution::execution_loop::state::RuntimeExecutionSlots;
use crate::stdlib::vec::Vec;
use crate::values::value_container::ValueContainer;
use core::fmt::Display;
use itertools::Itertools;

pub struct MemoryDump {
    pub slots: Vec<(u32, Option<ValueContainer>)>,
}

#[cfg(feature = "compiler")]
impl Display for MemoryDump {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for (address, value) in &self.slots {
            match value {
                Some(vc) => {
                    let decompiled = crate::decompiler::decompile_value(
                        vc,
                        crate::decompiler::DecompileOptions::colorized(),
                    );
                    writeln!(f, "#{address}: {decompiled}")?
                }
                None => writeln!(f, "#{address}: <uninitialized>")?,
            }
        }
        if self.slots.is_empty() {
            writeln!(f, "<no slots allocated>")?;
        }
        Ok(())
    }
}

impl RuntimeExecutionSlots {
    /// Returns a memory dump of the current slots and their values.
    pub fn memory_dump(&self) -> MemoryDump {
        MemoryDump {
            slots: self
                .slots
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .sorted_by_key(|(k, _)| *k)
                .collect(),
        }
    }
}
