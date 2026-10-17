/// Auto implements the Display trait for a given DATEX compatible type $ty (must implement [ToDatexExpressionData]),
/// with a fallback implementation provided in $fallback_impl that is used when the "value_display" feature is not enabled.
pub macro impl_display_for_datex_value($ty:ty, $fallback_impl:item) {

    #[cfg(feature = "value_display")]
    /// This implementation of Display for $ty is only available when the "value_display" feature is enabled.
    /// It converts the native value to [ToDatexExpressionData] and then formats it as source code.
    impl core::fmt::Display for $ty {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            use crate::decompiler::ast_to_source_code::value_to_source_code_default;

            core::write!(f, "{}", value_to_source_code_default(self))
        }
    }

    // TODO: do we need this fallback impl or just always use the value_display if needed?
    #[cfg(not(feature = "value_display"))]
    $fallback_impl

}
