use binrw::{BinRead, BinWrite};
use modular_bitfield::prelude::*;

use super::{addressing::{Endpoint, Sender}, serializable::Serializable};

#[derive(Debug, PartialEq, Clone, Default)]
#[derive(BitfieldSpecifier)]
pub enum SignatureType {
    #[default]
    None = 0b00,
    Invalid = 0b01,
    Unencrypted = 0b10,
    Encrypted = 0b11,
}

#[derive(Debug, PartialEq, Clone, Default)]
#[derive(BitfieldSpecifier)]
pub enum EncryptionType {
    #[default]
    Unencrypted = 0b0,
    Encrypted = 0b1,
}

#[derive(Debug, PartialEq, Clone, Default)]
#[derive(BitfieldSpecifier)]
pub enum BlockSize {
    #[default]
    Default = 0b0,
    Large = 0b1,
}

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


#[derive(Debug, Clone, Default)]
#[derive(BinWrite, BinRead)]
pub struct PointerId {
    pub pointer_type: u8,
    pub identifier: [u8; 18],
    pub instance: u16,
    pub timestamp: u32,
    pub counter: u8,
}


#[derive(Debug, Clone, Default)]
#[derive(BinWrite, BinRead)]
pub struct ReceiverEndpoints {
    pub count: u16,
    #[br(count = count)]
    pub endpoints: Vec<Endpoint>,
}

#[derive(Debug, Clone, Default)]
#[derive(BinWrite, BinRead)]
pub struct ReceiverEndpointsWithKeys {
    pub count: u16,
    #[br(count = count)]
    pub endpoints_with_keys: Vec<(Endpoint, [u8; 512])>,
}


#[derive(Debug, Clone, Default)]
#[derive(BinWrite, BinRead)]
pub struct Receivers {
    pub flags: ReceiverFlags,

    #[br(if(flags.has_pointer_id()))]
    pub pointer_id: Option<PointerId>,

    #[br(if(flags.has_endpoints() && !flags.has_endpoint_keys()))]
    pub endpoints: Option<ReceiverEndpoints>,
    #[br(if(flags.has_endpoints() && flags.has_endpoint_keys()))]
    pub endpoints_with_keys: Option<ReceiverEndpointsWithKeys>,
}


#[derive(Debug, Clone)]
#[derive(BinWrite, BinRead)]
#[brw(little, magic = b"\x01\x64")]
pub struct RoutingHeader {
	pub version: u8,
	pub ttl: u8,
	pub flags: Flags,

    pub scope_id: u32,
    pub block_index: u16,
    pub block_increment: u16,

	#[br(
        try,
        if(flags.block_size() == BlockSize::Default)
    )]
    pub block_size_u16: Option<u16>,

    #[br(
        try,
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
            block_size_u16: Some(37),
            block_size_u32: None,
            sender: Sender::default(),
            receivers: Receivers::default(),
        }
    }
}