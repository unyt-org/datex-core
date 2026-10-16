use crate::core_compiler::core_compilation_context::ByteCursor;
use binrw::{BinWrite, meta::WriteEndian};
// TBD move out, if needed by more than just compiler
pub trait BufferProvider {
    fn cursor_mut(&mut self) -> &mut ByteCursor;

    #[deprecated(
        note = "Use yield instructions, write is done on cursor directly"
    )]
    fn write<T: BinWrite + WriteEndian>(&mut self, value: T)
    where
        for<'a> <T as binrw::BinWrite>::Args<'a>: core::default::Default,
    {
        value
            .write(self.cursor_mut())
            .expect("Failed to write value to buffer");
    }
}
