use crate::{
    core_compiler::into_regular_instruction::{
        ToRegularInstruction,
    },
    instruction::regular_instruction::RegularInstruction,
};
impl ToRegularInstruction for u8 {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::uint8(*self)
    }
}

impl ToRegularInstruction for u16 {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::uint16(*self)
    }
}
impl ToRegularInstruction for u32 {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::uint32(*self)
    }
}
impl ToRegularInstruction for u64 {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::uint64(*self)
    }
}
impl ToRegularInstruction for u128 {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::uint128(*self)
    }
}

impl ToRegularInstruction for i8 {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::int8(*self)
    }
}

impl ToRegularInstruction for i16 {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::int16(*self)
    }
}

impl ToRegularInstruction for i32 {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::int32(*self)
    }
}

impl ToRegularInstruction for i64 {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::int64(*self)
    }
}
impl ToRegularInstruction for i128 {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::int128(*self)
    }
}

#[cfg(target_pointer_width = "32")]
impl ToRegularInstruction for isize {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::int32(*self as i32)
    }
}

#[cfg(target_pointer_width = "32")]
impl ToRegularInstruction for usize {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::uint32(*self as u32)
    }
}

#[cfg(target_pointer_width = "64")]
impl ToRegularInstruction for isize {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::int64(*self as i64)
    }
}

#[cfg(target_pointer_width = "64")]
impl ToRegularInstruction for usize {
    fn to_regular_instruction(&self) -> RegularInstruction {
        RegularInstruction::uint64(*self as u64)
    }
}