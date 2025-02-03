use binrw::{BinRead, BinWrite};
use modular_bitfield::prelude::*;

use super::{
    addressing::{Endpoint, Sender},
    serializable::Serializable,
};

// 2 bit
#[derive(Debug, PartialEq, Clone, Default, BitfieldSpecifier)]
pub enum SignatureType {
    #[default]
    None = 0b00,
    Invalid = 0b01,
    Unencrypted = 0b10,
    Encrypted = 0b11,
}

// 1 bit
#[derive(Debug, PartialEq, Clone, Default, BitfieldSpecifier)]
pub enum EncryptionType {
    #[default]
    Unencrypted = 0b0,
    Encrypted = 0b1,
}

// 1 bit
#[derive(Debug, PartialEq, Clone, Default, BitfieldSpecifier)]
pub enum BlockSize {
    #[default]
    Default = 0b0,
    Large = 0b1,
}

// 2 bit + 1 bit + 1 bit + 4 bit = 1 byte
#[bitfield]
#[derive(BinWrite, BinRead, Clone, Default, Copy, Debug)]
#[bw(map = |&x| Self::into_bytes(x))]
#[br(map = Self::from_bytes)]
pub struct Flags {
    pub signature_type: SignatureType,
    pub encryption_type: EncryptionType,
    pub block_size: BlockSize,

    #[allow(unused)]
    unused_0: bool,
    #[allow(unused)]
    unused_1: bool,
    #[allow(unused)]
    unused_2: bool,
    #[allow(unused)]
    unused_3: bool,
}

// 1 byte
#[bitfield]
#[derive(BinWrite, BinRead, Clone, Default, Copy, Debug)]
#[bw(map = |&x| Self::into_bytes(x))]
#[br(map = Self::from_bytes)]
pub struct ReceiverFlags {
    pub has_pointer_id: bool,
    pub has_endpoints: bool,
    pub has_endpoint_keys: bool,

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
}

// 1 byte + 18 byte + 2 byte + 4 byte + 1 byte = 26 bytes
#[derive(Debug, Clone, Default, BinWrite, BinRead)]
pub struct PointerId {
    pub pointer_type: u8,
    pub identifier: [u8; 18],
    pub instance: u16,
    pub timestamp: u32,
    pub counter: u8,
}

// <count>: 2 byte + (21 byte * count)
// min: 2 bytes
#[derive(Debug, Clone, Default, BinWrite, BinRead)]
pub struct ReceiverEndpoints {
    pub count: u16,
    #[br(count = count)]
    pub endpoints: Vec<Endpoint>,
}

// <count>: 2 byte + (21 byte * count) + (512 byte * count)
// min: 2 bytes
#[derive(Debug, Clone, Default, BinWrite, BinRead)]
pub struct ReceiverEndpointsWithKeys {
    pub count: u16,
    #[br(count = count)]
    pub endpoints_with_keys: Vec<(Endpoint, [u8; 512])>,
}

// min: 1 byte
#[derive(Debug, Clone, Default, BinWrite, BinRead)]
pub struct Receivers {
    pub flags: ReceiverFlags,

    #[brw(if(flags.has_pointer_id()))]
    pub pointer_id: Option<PointerId>,

    #[brw(if(flags.has_endpoints() && !flags.has_endpoint_keys()))]
    pub endpoints: Option<ReceiverEndpoints>,
    #[brw(if(flags.has_endpoints() && flags.has_endpoint_keys()))]
    pub endpoints_with_keys: Option<ReceiverEndpointsWithKeys>,
}

// min: 11 byte + 2 byte + 1 byte + 1 byte = 15 bytes
#[derive(Debug, Clone, BinWrite, BinRead)]
#[brw(little, magic = b"\x01\x64")]
pub struct RoutingHeader {
    pub version: u8,
    pub ttl: u8,
    pub flags: Flags,

    pub scope_id: u32,
    pub block_index: u16,
    pub block_increment: u16,

    #[brw(
        if(flags.block_size() == BlockSize::Default)
    )]
    pub block_size_u16: Option<u16>,

    #[brw(
        if(flags.block_size() == BlockSize::Large),
        assert(
            match flags.block_size() {
                BlockSize::Large => block_size_u32.is_some(),
                BlockSize::Default => block_size_u16.is_some(),
            },
            "No valid block size found"
        ),
    )]
    pub block_size_u32: Option<u32>,

    pub sender: Sender,
    pub receivers: Receivers,
}

impl Serializable for RoutingHeader {}

impl Default for RoutingHeader {
    fn default() -> Self {
        RoutingHeader {
            version: 1,
            ttl: 0,
            flags: Flags::new(),
            scope_id: 0,
            block_index: 0,
            block_increment: 0,
            block_size_u16: Some(26),
            block_size_u32: None,
            sender: Sender::default(),
            receivers: Receivers::default(),
        }
    }
}
