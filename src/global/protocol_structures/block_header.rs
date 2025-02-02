use binrw::{BinRead, BinWrite};


#[derive(Debug, Clone, Default)]
#[derive(BinWrite, BinRead)]
#[brw(little,)]
pub struct BlockHeader {

}