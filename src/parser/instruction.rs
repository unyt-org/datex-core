use binrw::{BinRead, BinWrite};

#[derive(BinRead, BinWrite, Clone, Copy, Debug, PartialEq)]
#[brw(little)]
pub enum Instruction {
    /// unsigned 8 bit integer
    #[brw(magic(0xC1u8))]
    Int8(i8),
    /// unsigned 16 bit integer
    #[brw(magic(0xC2u8))]
    Int16(i16),
}
