use super::serializable::Serializable;
use crate::values::core_values::endpoint::Endpoint;
use binrw::{BinRead, BinWrite};
use modular_bitfield::{bitfield, prelude::B43, BitfieldSpecifier};
use strum_macros::Display;

// 4 bit
#[derive(
    Debug, Display, PartialEq, Clone, Copy, Default, BitfieldSpecifier,
)]
pub enum BlockType {
    #[default]
    Request = 0,
    Response = 1,
    Hello = 2,
    Trace = 3,
    TraceBack = 4,
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

impl BlockType {
    pub fn is_response(&self) -> bool {
        match self {
            BlockType::Response | BlockType::TraceBack => true,
            _ => false,
        }
    }
}

// 21 bit + 43 bit = 64 bit
#[bitfield]
#[derive(BinWrite, BinRead, Clone, Copy, Debug, PartialEq)]
#[bw(map = |&x| Self::into_bytes(x))]
#[br(map = Self::from_bytes)]
pub struct FlagsAndTimestamp {
    pub block_type: BlockType,
    pub allow_execution: bool,
    pub is_end_of_section: bool,
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

impl Default for FlagsAndTimestamp {
    fn default() -> Self {
        FlagsAndTimestamp::new()
            .with_block_type(BlockType::Request)
            .with_allow_execution(false)
            .with_is_end_of_section(true)
            .with_is_end_of_scope(true)
            .with_has_lifetime(false)
            .with_has_represented_by(false)
            .with_has_iv(false)
            .with_is_compressed(false)
            .with_is_signature_in_last_subblock(false)
    }
}

// min: 16 byte
// max 8 + 8 byte + 4 byte + 21 byte + 16 byte = 57 byte
#[derive(Debug, Clone, Default, BinWrite, BinRead, PartialEq)]
#[brw(little)]
pub struct BlockHeader {
    /// A unique id that defines the context in which this block lives
    /// A context has a persistent state that can e.g. contain DATEX variables
    pub context_id: u32,
    /// A section is a collection of multiple sequential blocks inside the same context
    /// (each with an incrementing block number)
    /// When a new section starts, the block number is not reset but continues to increment
    pub section_index: u16,
    /// A unique number that identifies a block inside a block context
    /// The context_id combined with the block_number define a unique block from a specific endpoint
    /// the block id (endpoint, context_id, block_number) defines a globally unique block
    /// Note: blocks ids are not completely unique, when the block_number or section_index overflows,
    /// it starts from 0 again, leading to duplicate block ids after a while
    pub block_number: u16,

    pub flags_and_timestamp: FlagsAndTimestamp,

    #[brw(if(flags_and_timestamp.has_lifetime()))]
    pub lifetime: Option<u32>,

    #[brw(if(flags_and_timestamp.has_represented_by()))]
    pub represented_by: Option<Endpoint>,

    #[brw(if(flags_and_timestamp.has_iv()))]
    pub iv: [u8; 16],
}

impl Serializable for BlockHeader {}
