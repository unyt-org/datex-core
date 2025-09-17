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


// 2 bit + 1 bit + 1 bit + 4 bit = 1 byte
#[bitfield]
#[derive(BinWrite, BinRead, Clone, Default, Copy, Debug, PartialEq)]
#[bw(map = |&x| Self::into_bytes(x))]
#[br(map = Self::from_bytes)]
pub struct Flags {
    pub signature_type: SignatureType,
    pub encryption_type: EncryptionType,
    pub receiver_type: ReceiverType,
    pub is_bounce_back: bool,
    pub has_checksum: bool,

    #[allow(unused)]
    unused_2: bool,
}

// 2 bit
#[derive(Debug, PartialEq, Clone, Default, Specifier)]
#[bits = 2]
pub enum ReceiverType {
    #[default]
    None = 0b00,
    Pointer = 0b01,
    Receivers = 0b10,
    ReceiversWithKeys = 0b11,
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

// <count>: 1 byte + (21 byte * count)
// min: 2 bytes
#[derive(Debug, Clone, Default, BinWrite, BinRead, PartialEq)]
pub struct ReceiverEndpoints {
    pub count: u8,
    #[br(count = count)]
    pub endpoints: Vec<Endpoint>,
}

impl ReceiverEndpoints {
    pub fn new(endpoints: Vec<Endpoint>) -> Self {
        let count = endpoints.len() as u8;
        ReceiverEndpoints { count, endpoints }
    }
}

// <count>: 1 byte + (21 byte * count) + (512 byte * count)
// min: 2 bytes
#[derive(Debug, Clone, Default, BinWrite, BinRead, PartialEq)]
pub struct ReceiverEndpointsWithKeys {
    pub count: u8,
    #[br(count = count)]
    pub endpoints_with_keys: Vec<(Endpoint, [u8; 512])>,
}


// min: 11 byte + 2 byte + 21 byte + 1 byte = 35 bytes
#[derive(Debug, Clone, BinWrite, BinRead, PartialEq)]
#[brw(little, magic = b"\x01\x64")]
pub struct RoutingHeader {
    pub version: u8,
    pub block_size: u16,
    pub flags: Flags,

    #[brw(if(flags.has_checksum()))]
    pub checksum: u32,

    pub distance: i8,
    pub ttl: u8,

    pub sender: Endpoint,

    // TODO #115: add custom match receiver queries
    #[brw(if(flags.receiver_type() == ReceiverType::Pointer))]
    pub receivers_pointer_id: Option<PointerId>,

    #[brw(if(flags.receiver_type() == ReceiverType::Receivers))]
    pub receivers_endpoints: Option<ReceiverEndpoints>,
    #[brw(if(flags.receiver_type() == ReceiverType::ReceiversWithKeys))]
    pub receivers_endpoints_with_keys: Option<ReceiverEndpointsWithKeys>,
}

impl Serializable for RoutingHeader {}

impl Default for RoutingHeader {
    fn default() -> Self {
        RoutingHeader {
            version: 1,
            distance: 0,
            ttl: 42,
            flags: Flags::new(),
            checksum: 0,
            block_size: 0,
            sender: Endpoint::default(),
            receivers_pointer_id: None,
            receivers_endpoints: None,
            receivers_endpoints_with_keys: None,
        }
    }
}
