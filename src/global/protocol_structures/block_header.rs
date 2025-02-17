
use binrw::{BinRead, BinWrite};
use modular_bitfield::{bitfield, prelude::B43, BitfieldSpecifier};

use super::{addressing::Endpoint, serializable::Serializable};

// 4 bit
#[derive(Debug, PartialEq, Clone, Copy, Default, BitfieldSpecifier)]
pub enum BlockType {
    #[default]
    Request = 0,
    Response = 1,
    #[allow(unused)]
    Unused0,
    #[allow(unused)]
    Unused1,
    #[allow(unused)]
    Unused2,
    #[allow(unused)]
    Unused3,
    #[allow(unused)]
    Unused4,
    #[allow(unused)]
    Unused5,
    #[allow(unused)]
    Unused6,
    #[allow(unused)]
    Unused7,
    #[allow(unused)]
    Unused8,
    #[allow(unused)]
    Unused9,
    #[allow(unused)]
    Unused10,
    #[allow(unused)]
    Unused11,
    #[allow(unused)]
    Unused12,
    #[allow(unused)]
    Unused13,
}

// 21 bit + 43 bit = 64 bit
#[bitfield]
#[derive(BinWrite, BinRead, Clone, Default, Copy, Debug, PartialEq)]
#[bw(map = |&x| Self::into_bytes(x))]
#[br(map = Self::from_bytes)]
pub struct FlagsAndTimestamp {
    pub block_type: BlockType,
    pub allow_execution: bool,
    pub is_end_of_block: bool,
    pub is_end_of_scope: bool,
    pub has_lifetime: bool,
    pub has_represented_by: bool,
    pub has_iv: bool,
    pub is_compressed: bool,
    pub is_signature_in_last_subblock: bool,

    #[allow(unused)]
    unused_0: bool,
    #[allow(unused)]
    unused_1: bool,
    #[allow(unused)]
    unused_2: bool,
    #[allow(unused)]
    unused_3: bool,
    #[allow(unused)]
    unused_4: bool,
    #[allow(unused)]
    unused_5: bool,
    #[allow(unused)]
    unused_6: bool,
    #[allow(unused)]
    unused_7: bool,
    #[allow(unused)]
    unused_8: bool,

    pub creation_timestamp: B43,
}

// min: 8 byte
// max 8 byte + 4 byte + 21 byte + 16 byte = 49 byte
#[derive(Debug, Clone, Default, BinWrite, BinRead, PartialEq)]
#[brw(little)]
pub struct BlockHeader {
    pub flags_and_timestamp: FlagsAndTimestamp,

    #[brw(if(flags_and_timestamp.has_lifetime()))]
    pub lifetime: Option<u32>,

    #[brw(if(flags_and_timestamp.has_represented_by()))]
    pub represented_by: Option<Endpoint>,

    #[brw(if(flags_and_timestamp.has_iv()))]
    pub iv: [u8; 16],
}

impl Serializable for BlockHeader {}
