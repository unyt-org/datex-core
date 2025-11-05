use super::serializable::Serializable;
use crate::values::core_values::endpoint::Endpoint;
use binrw::{BinRead, BinWrite};
use core::prelude::rust_2024::*;
use modular_bitfield::{Specifier, bitfield};

// 4 bit
#[cfg_attr(feature = "debug", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Clone, Default, Specifier)]
pub enum UserAgent {
    #[default]
    Unknown = 0,
    Human = 1,
    Bot = 2,
    Service = 3,

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
    pub user_agent: UserAgent,
    pub has_on_behalf_of: bool,

    #[allow(unused)]
    unused_0: bool,
    #[allow(unused)]
    unused_1: bool,
    #[allow(unused)]
    unused_2: bool,
}

#[cfg(feature = "debug")]
mod flags_serde {
    use super::*;
    use crate::global::protocol_structures::encrypted_header::Flags;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct FlagsHelper {
        user_agent: UserAgent,
        has_on_behalf_of: bool,
    }

    impl Serialize for Flags {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let helper = FlagsHelper {
                user_agent: self.user_agent(),
                has_on_behalf_of: self.has_on_behalf_of(),
            };
            helper.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for Flags {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let helper = FlagsHelper::deserialize(deserializer)?;
            Ok(Flags::new()
                .with_user_agent(helper.user_agent)
                .with_has_on_behalf_of(helper.has_on_behalf_of))
        }
    }
}

// min: 1 byte
// max: 1 byte + 21 bytes = 22 bytes
#[cfg_attr(feature = "debug", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, BinWrite, BinRead, PartialEq)]
#[brw(little)]
pub struct EncryptedHeader {
    pub flags: Flags,

    #[brw(if (flags.has_on_behalf_of()))]
    pub on_behalf_of: Option<Endpoint>,
}
impl Serializable for EncryptedHeader {}
