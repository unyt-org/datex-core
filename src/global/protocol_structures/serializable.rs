use binrw::{
    meta::{ReadEndian, WriteEndian},
    BinWrite,
};
use std::io::Cursor; // FIXME no-std

pub trait Serializable: BinWrite + ReadEndian + WriteEndian {
    fn to_bytes(&self) -> Result<Vec<u8>, binrw::Error>
    where
        for<'a> Self::Args<'a>: Default,
    {
        let mut writer = Cursor::new(Vec::new());
        self.write(&mut writer)?;
        let bytes = writer.into_inner();
        Ok(bytes)
    }
}
