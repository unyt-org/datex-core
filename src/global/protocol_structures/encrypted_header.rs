use super::serializable::Serializable;
use crate::values::core_values::endpoint::Endpoint;
use binrw::{BinRead, BinWrite};
use modular_bitfield::{bitfield, BitfieldSpecifier};

// 4 bit
#[derive(Debug, PartialEq, Clone, Default, BitfieldSpecifier)]
pub enum DeviceType {
    #[default]
    Unknown = 0,
    Mobile = 1,
    Desktop = 2,
    Bot = 3,

    #[allow(unused)]
    Unused0 = 4,
    #[allow(unused)]
    Unused1 = 5,
    #[allow(unused)]
    Unused2 = 6,
    #[allow(unused)]
    Unused3 = 7,
    #[allow(unused)]
    Unused4 = 8,
    #[allow(unused)]
    Unused5 = 9,
    #[allow(unused)]
    Unused6 = 10,
    #[allow(unused)]
    Unused7 = 11,
    #[allow(unused)]
    Unused8 = 12,
    #[allow(unused)]
    Unused9 = 13,
    #[allow(unused)]
    Unused10 = 14,
    #[allow(unused)]
    Unused11 = 15,
}

// 4 bit + 4 bit = 8 bit
#[bitfield]
#[derive(BinWrite, BinRead, Clone, Default, Copy, Debug, PartialEq)]
#[bw(map = |&x| Self::into_bytes(x))]
#[br(map = Self::from_bytes)]
pub struct Flags {
    pub device_type: DeviceType,
    pub has_on_behalf_of: bool,
    unused_0: bool,
    unused_1: bool,
    unused_2: bool,
}

// min: 1 byte
// max: 1 byte + 21 bytes = 22 bytes
#[derive(Debug, Clone, Default, BinWrite, BinRead, PartialEq)]
#[brw(little)]
pub struct EncryptedHeader {
    pub flags: Flags,

    #[brw(if (flags.has_on_behalf_of()))]
    pub on_behalf_of: Option<Endpoint>,
}
impl Serializable for EncryptedHeader {}
