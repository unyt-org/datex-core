use binrw::io::Write;
use binrw::io::Seek;
use binrw::io::Read;
use binrw::{BinRead, BinResult, BinWrite, Endian};
use crate::instruction::instruction_data::TextData;
use crate::values::core_values::text::Text;
use crate::prelude::*;

impl BinRead for Text {
    type Args<'a> = <TextData as BinRead>::Args<'a>;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        args: Self::Args<'_>,
    ) -> BinResult<Self> {
        TextData::read_options(reader, endian, args).map(|t| Text::String(t.0))
    }
}

impl BinWrite for Text {
    type Args<'a> = <TextData as BinWrite>::Args<'a>;

    fn write_options<W: Write + Seek>(
        &self,
        writer: &mut W,
        endian: Endian,
        args: Self::Args<'_>,
    ) -> BinResult<()> {
        TextData(self.as_str().to_owned()).write_options(writer, endian, args)
    }
}