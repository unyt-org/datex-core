use binrw::{
  meta::{ReadEndian, WriteEndian},
  BinWrite,
};
use std::io::Cursor;

pub trait Serializable: BinWrite + ReadEndian + WriteEndian {
  fn to_bytes(&self) -> anyhow::Result<Vec<u8>>
  where
    for<'a> Self::Args<'a>: Default,
  {
    let mut writer = Cursor::new(Vec::new());
    self.write(&mut writer)?;
    let bytes = writer.into_inner();
    return Ok(bytes);
  }
}
