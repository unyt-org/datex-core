use super::serializable::Serializable;
use crate::values::core_values::endpoint::Endpoint;
use binrw::{BinRead, BinWrite};
use core::prelude::rust_2024::*;
use modular_bitfield::{Specifier, bitfield, prelude::B43};
use strum_macros::Display;

// 4 bit
#[derive(Debug, Display, PartialEq, Clone, Copy, Default, Specifier)]
#[cfg_attr(feature = "debug", derive(serde::Serialize, serde::Deserialize))]
#[bits = 4]
pub enum BlockType {
    #[default]
    Request = 0,
    Response = 1,
    Hello = 2,
    Trace = 3,
    TraceBack = 4,
}

impl BlockType {
    pub fn is_response(&self) -> bool {
        core::matches!(self, BlockType::Response | BlockType::TraceBack)
    }
}

// 21 bit + 43 bit = 64 bit
/// has_side_effects: If set, the block can have side effects that change external state. Default is true
/// has_only_data: If set, the block does only contain data and no executable instructions. Default is false
#[bitfield]
#[derive(BinWrite, BinRead, Clone, Copy, Debug, PartialEq)]
#[bw(map = |&x| Self::into_bytes(x))]
#[br(map = Self::from_bytes)]
#[brw(little)]
pub struct FlagsAndTimestamp {
    pub block_type: BlockType,
    pub has_side_effects: bool,
    pub has_only_data: bool,
    pub is_end_of_section: bool,
    pub is_end_of_context: bool,
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

    pub creation_timestamp: B43,
}

#[cfg(feature = "debug")]
mod flags_and_timestamp_serde {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct FlagsHelper {
        block_type: BlockType,
        has_side_effects: bool,
        has_only_data: bool,
        is_end_of_section: bool,
        is_end_of_context: bool,
        has_lifetime: bool,
        has_represented_by: bool,
        has_iv: bool,
        is_compressed: bool,
        is_signature_in_last_subblock: bool,
        creation_timestamp: u64,
    }

    impl Serialize for FlagsAndTimestamp {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let helper = FlagsHelper {
                block_type: self.block_type(),
                has_side_effects: self.has_side_effects(),
                has_only_data: self.has_only_data(),
                is_end_of_section: self.is_end_of_section(),
                is_end_of_context: self.is_end_of_context(),
                has_lifetime: self.has_lifetime(),
                has_represented_by: self.has_represented_by(),
                has_iv: self.has_iv(),
                is_compressed: self.is_compressed(),
                is_signature_in_last_subblock: self
                    .is_signature_in_last_subblock(),
                creation_timestamp: self.creation_timestamp(),
            };
            helper.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for FlagsAndTimestamp {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let helper = FlagsHelper::deserialize(deserializer)?;
            Ok(FlagsAndTimestamp::new()
                .with_block_type(helper.block_type)
                .with_has_side_effects(helper.has_side_effects)
                .with_has_only_data(helper.has_only_data)
                .with_is_end_of_section(helper.is_end_of_section)
                .with_is_end_of_context(helper.is_end_of_context)
                .with_has_lifetime(helper.has_lifetime)
                .with_has_represented_by(helper.has_represented_by)
                .with_has_iv(helper.has_iv)
                .with_is_compressed(helper.is_compressed)
                .with_is_signature_in_last_subblock(
                    helper.is_signature_in_last_subblock,
                )
                .with_creation_timestamp(helper.creation_timestamp))
        }
    }
}

impl Default for FlagsAndTimestamp {
    fn default() -> Self {
        FlagsAndTimestamp::new()
            .with_block_type(BlockType::Request)
            .with_has_side_effects(true)
            .with_has_only_data(false)
            .with_is_end_of_section(true)
            .with_is_end_of_context(true)
            .with_has_lifetime(false)
            .with_has_represented_by(false)
            .with_has_iv(false)
            .with_is_compressed(false)
            .with_is_signature_in_last_subblock(false)
    }
}

// min: 16 byte
// max 8 + 8 byte + 4 byte + 21 byte + 16 byte = 57 byte
#[cfg_attr(feature = "debug", derive(serde::Serialize, serde::Deserialize))]
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
    pub iv: Option<[u8; 16]>,
}

impl Serializable for BlockHeader {}
