use super::serializable::Serializable;
use crate::values::core_values::endpoint::Endpoint;
use binrw::{BinRead, BinWrite};
use modular_bitfield::prelude::*;

// 2 bit
#[derive(Debug, PartialEq, Clone, Default, Specifier)]
#[bits = 2]
pub enum SignatureType {
    #[default]
    None = 0b00,
    Unencrypted = 0b10,
    Encrypted = 0b11,
}

// 1 bit
#[derive(Debug, PartialEq, Clone, Default, Specifier)]
pub enum EncryptionType {
    #[default]
    Unencrypted = 0b0,
    Encrypted = 0b1,
}

// 1 bit
#[derive(Debug, PartialEq, Clone, Default, Specifier)]
pub enum BlockSize {
    #[default]
    Default = 0b0,
    Large = 0b1,
}

// 2 bit + 1 bit + 1 bit + 4 bit = 1 byte
#[bitfield]
#[derive(BinWrite, BinRead, Clone, Default, Copy, Debug, PartialEq)]
#[bw(map = |&x| Self::into_bytes(x))]
#[br(map = Self::from_bytes)]
pub struct Flags {
    pub signature_type: SignatureType,
    pub encryption_type: EncryptionType,
    pub block_size: BlockSize,
    pub is_bounce_back: bool,

    #[allow(unused)]
    unused_0: bool,
    #[allow(unused)]
    unused_1: bool,
    #[allow(unused)]
    unused_2: bool,
}

// 1 byte
#[bitfield]
#[derive(BinWrite, BinRead, Clone, Default, Copy, Debug, PartialEq)]
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
#[derive(Debug, Clone, Default, BinWrite, BinRead, PartialEq)]
pub struct PointerId {
    pub pointer_type: u8,
    pub identifier: [u8; 18],
    pub instance: u16,
    pub timestamp: u32,
    pub counter: u8,
}

// <count>: 2 byte + (21 byte * count)
// min: 2 bytes
#[derive(Debug, Clone, Default, BinWrite, BinRead, PartialEq)]
pub struct ReceiverEndpoints {
    pub count: u16,
    #[br(count = count)]
    pub endpoints: Vec<Endpoint>,
}

impl ReceiverEndpoints {
    pub fn new(endpoints: Vec<Endpoint>) -> Self {
        let count = endpoints.len() as u16;
        ReceiverEndpoints { count, endpoints }
    }
}

// <count>: 2 byte + (21 byte * count) + (512 byte * count)
// min: 2 bytes
#[derive(Debug, Clone, Default, BinWrite, BinRead, PartialEq)]
pub struct ReceiverEndpointsWithKeys {
    pub count: u16,
    #[br(count = count)]
    pub endpoints_with_keys: Vec<(Endpoint, [u8; 512])>,
}

// min: 1 byte
#[derive(Debug, Clone, Default, BinWrite, BinRead, PartialEq)]
pub struct Receivers {
    pub flags: ReceiverFlags,

    #[brw(if(flags.has_pointer_id()))]
    pub pointer_id: Option<PointerId>,

    #[brw(if(flags.has_endpoints() && !flags.has_endpoint_keys()))]
    pub endpoints: Option<ReceiverEndpoints>,
    #[brw(if(flags.has_endpoints() && flags.has_endpoint_keys()))]
    pub endpoints_with_keys: Option<ReceiverEndpointsWithKeys>,
}

// min: 11 byte + 2 byte + 21 byte + 1 byte = 35 bytes
#[derive(Debug, Clone, BinWrite, BinRead, PartialEq)]
#[brw(little, magic = b"\x01\x64")]
pub struct RoutingHeader {
    pub version: u8,
    pub distance: i8,
    pub ttl: u8,
    pub flags: Flags,

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

    pub sender: Endpoint,
    // TODO #115: add custom match receiver queries
    pub receivers: Receivers,
}

impl Serializable for RoutingHeader {}

impl Default for RoutingHeader {
    fn default() -> Self {
        RoutingHeader {
            version: 1,
            distance: 0,
            ttl: 42,
            flags: Flags::new(),
            block_size_u16: Some(26),
            block_size_u32: None,
            sender: Endpoint::default(),
            receivers: Receivers::default(),
        }
    }
}
