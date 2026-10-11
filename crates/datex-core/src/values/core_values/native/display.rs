use crate::values::core_values::native::NativeCoreValue;
use core::fmt::{Display, Formatter};

#[cfg(feature = "value_display")]
impl Display for NativeCoreValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        use crate::decompiler::ast_to_source_code::value_to_source_code_default;

        write!(f, "{}", value_to_source_code_default(&self.value))
    }
}

// TODO: do we need this fallback impl or just always use the value_display if needed?
#[cfg(not(feature = "value_display"))]
impl Display for NativeCoreValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "[[native value]]")
    }
}
