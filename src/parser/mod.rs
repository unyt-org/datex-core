use std::io::Cursor;
use binrw::{BinRead, BinWrite};
use crate::parser::instruction::Instruction;

pub mod instruction;

/// Reads a slice of raw byte code and returns an iterator over the instructions.
pub fn iterate_instructions<'a>(
    dxb_body: &'a [u8]
) -> impl Iterator<Item = Result<Instruction, binrw::Error>> + 'a {
    let mut reader = Cursor::new(dxb_body);

    std::iter::from_coroutine(
        #[coroutine]
        move || {
            while reader.position() < dxb_body.len() as u64 {
                yield Instruction::read(&mut reader)
            }
        }
    )
}

/// Converts a slice of instructions into a byte code array.
pub fn instructions_to_bytes(
    instructions: &[Instruction]
) -> Result<Vec<u8>, binrw::Error> {
    let mut writer = Cursor::new(Vec::new());
    for instruction in instructions {
        instruction.write(&mut writer)?;
    }
    Ok(writer.into_inner())
}



#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_int8() {
        let dxb_body: &[u8] = &[0xC1, 0x01];
        let mut iter = iterate_instructions(dxb_body);
        assert_eq!(iter.next().unwrap().unwrap(), Instruction::Int8(1));
    }

    #[test]
    fn parse_int16() {
        let dxb_body: &[u8] = &[0xC2, 0xff, 0x01];
        let mut iter = iterate_instructions(dxb_body);
        assert_eq!(iter.next().unwrap().unwrap(), Instruction::Int16(0x01ff));
    }

    #[test]
    fn parse_multiple() {
        let dxb_body: &[u8] = &[0xC1, 0x01, 0xC2, 0x02, 0x00];
        let mut iter = iterate_instructions(dxb_body);
        assert_eq!(iter.next().unwrap().unwrap(), Instruction::Int8(1));
        assert_eq!(iter.next().unwrap().unwrap(), Instruction::Int16(2));
    }

    #[test]
    fn parse_empty() {
        let dxb_body: &[u8] = &[];
        let mut iter = iterate_instructions(dxb_body);
        if iter.next().is_some() {
            panic!("Expected None, but got Some");
        }
    }

    #[test]
    fn parse_end_of_file() {
        let dxb_body: &[u8] = &[0xC1, 0x01];
        let mut iter = iterate_instructions(dxb_body);
        assert_eq!(iter.next().unwrap().unwrap(), Instruction::Int8(1));
        if iter.next().is_some() {
            panic!("Expected None, but got Some");
        }
    }

    #[test]
    fn parse_invalid() {
        let dxb_body: &[u8] = &[0xC1];
        let mut iter = iterate_instructions(dxb_body);

        // should return an error
        if !iter.next().unwrap().is_err() {
            panic!("Expected an error, but got Ok(_)");
        }
    }

}