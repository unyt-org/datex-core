use super::serializable::Serializable;
use crate::global::protocol_structures::instructions::RawFullPointerAddress;
use crate::stdlib::vec::Vec;
use crate::values::core_values::endpoint::Endpoint;
use binrw::{BinRead, BinWrite};
use core::fmt::Display;
use core::prelude::rust_2024::*;
use modular_bitfield::prelude::*;

// 2 bit
#[cfg_attr(feature = "debug", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Clone, Default, Specifier)]
#[bits = 2]
pub enum SignatureType {
    #[default]
    None = 0b00,
    Unencrypted = 0b10,
    Encrypted = 0b11,
}

// 1 bit
#[cfg_attr(feature = "debug", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Clone, Default, Specifier)]
pub enum EncryptionType {
    #[default]
    None = 0b0,
    Encrypted = 0b1,
}

// 2 bit + 1 bit + 2 bit + 1 bit + 1 bit + 1 bit = 1 byte
#[bitfield]
#[derive(BinWrite, BinRead, Clone, Default, Copy, Debug, PartialEq)]
#[bw(map = |&x| Self::into_bytes(x))]
#[br(map = Self::from_bytes)]
pub struct Flags {
    pub signature_type: SignatureType,   // 2 bit
    pub encryption_type: EncryptionType, // 1 bit
    pub receiver_type: ReceiverType,     // 2 bit
    pub is_bounce_back: bool,            // 1 bit
    pub has_checksum: bool,              // 1 bit

    #[allow(unused)]
    unused_2: bool,
}

#[cfg(feature = "debug")]
mod flags_serde {
    use super::*;
    use crate::global::protocol_structures::routing_header::Flags;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    #[derive(Serialize, Deserialize)]
    struct FlagsHelper {
        signature_type: SignatureType,
        encryption_type: EncryptionType,
        receiver_type: ReceiverType,
        is_bounce_back: bool,
        has_checksum: bool,
    }

    impl Serialize for Flags {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let helper = FlagsHelper {
                signature_type: self.signature_type(),
                encryption_type: self.encryption_type(),
                receiver_type: self.receiver_type(),
                is_bounce_back: self.is_bounce_back(),
                has_checksum: self.has_checksum(),
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
                .with_signature_type(helper.signature_type)
                .with_encryption_type(helper.encryption_type)
                .with_receiver_type(helper.receiver_type)
                .with_is_bounce_back(helper.is_bounce_back)
                .with_has_checksum(helper.has_checksum))
        }
    }
}

// 2 bit
#[cfg_attr(feature = "debug", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Clone, Default, Specifier)]
#[bits = 2]
pub enum ReceiverType {
    #[default]
    None = 0b00,
    Pointer = 0b01,
    Receivers = 0b10,
    ReceiversWithKeys = 0b11,
}

// <count>: 1 byte + (21 byte * count)
// min: 2 bytes
#[cfg_attr(feature = "debug", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, BinWrite, BinRead, PartialEq)]
pub struct ReceiverEndpoints {
    #[cfg_attr(feature = "debug", serde(rename = "number_of_receivers"))]
    pub count: u8,
    #[br(count = count)]
    #[cfg_attr(feature = "debug", serde(rename = "receivers"))]
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
#[cfg_attr(feature = "debug", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, BinWrite, BinRead, PartialEq)]
pub struct ReceiverEndpointsWithKeys {
    #[cfg_attr(feature = "debug", serde(rename = "number_of_receivers"))]
    count: u8,
    #[br(count = count)]
    #[cfg_attr(feature = "debug", serde(rename = "receivers_with_keys"))]
    pub endpoints_with_keys: Vec<(Endpoint, Key512)>,
}

#[cfg_attr(feature = "debug", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, BinWrite, BinRead, PartialEq)]
pub struct Key512(
    #[cfg_attr(feature = "debug", serde(with = "serde_big_array::BigArray"))]
    [u8; 512],
);
impl Default for Key512 {
    fn default() -> Self {
        Key512([0u8; 512])
    }
}
impl From<[u8; 512]> for Key512 {
    fn from(arr: [u8; 512]) -> Self {
        Key512(arr)
    }
}

impl ReceiverEndpointsWithKeys {
    pub fn new<T>(endpoints_with_keys: Vec<(Endpoint, T)>) -> Self
    where
        T: Into<Key512>,
    {
        let count = endpoints_with_keys.len() as u8;
        ReceiverEndpointsWithKeys {
            count,
            endpoints_with_keys: endpoints_with_keys
                .into_iter()
                .map(|(ep, key)| (ep, key.into()))
                .collect(),
        }
    }
}

// min: 11 byte + 2 byte + 21 byte + 1 byte = 35 bytes
#[cfg_attr(feature = "debug", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, BinWrite, BinRead, PartialEq)]
#[brw(little, magic = b"\x01\x64")]
pub struct RoutingHeader {
    pub version: u8,
    pub block_size: u16,
    pub flags: Flags,

    #[brw(if(flags.has_checksum()))]
    checksum: Option<u32>,

    pub distance: i8,
    pub ttl: u8,

    pub sender: Endpoint,

    // TODO #115: add custom match receiver queries
    #[brw(if(flags.receiver_type() == ReceiverType::Pointer))]
    receivers_pointer_id: Option<RawFullPointerAddress>,
    #[brw(if(flags.receiver_type() == ReceiverType::Receivers))]
    #[cfg_attr(feature = "debug", serde(flatten))]
    receivers_endpoints: Option<ReceiverEndpoints>,
    #[brw(if(flags.receiver_type() == ReceiverType::ReceiversWithKeys))]
    #[cfg_attr(feature = "debug", serde(flatten))]
    receivers_endpoints_with_keys: Option<ReceiverEndpointsWithKeys>,
}

impl Serializable for RoutingHeader {}

impl Default for RoutingHeader {
    fn default() -> Self {
        RoutingHeader {
            version: 1,
            distance: 0,
            ttl: 42,
            flags: Flags::new(),
            checksum: None,
            block_size: 0,
            sender: Endpoint::default(),
            receivers_pointer_id: None,
            receivers_endpoints: None,
            receivers_endpoints_with_keys: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Receivers {
    None,
    // TODO #431 rename to PointerAddress
    PointerId(RawFullPointerAddress),
    Endpoints(Vec<Endpoint>),
    EndpointsWithKeys(Vec<(Endpoint, Key512)>),
}
impl Display for Receivers {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Receivers::None => core::write!(f, "No receivers"),
            Receivers::PointerId(pid) => {
                core::write!(f, "Pointer ID: {:?}", pid)
            }
            Receivers::Endpoints(endpoints) => {
                core::write!(f, "Endpoints: {:?}", endpoints)
            }
            Receivers::EndpointsWithKeys(endpoints_with_keys) => {
                core::write!(
                    f,
                    "Endpoints with keys: {:?}",
                    endpoints_with_keys
                )
            }
        }
    }
}

impl<T> From<T> for Receivers
where
    T: Into<RawFullPointerAddress>,
{
    fn from(pid: T) -> Self {
        Receivers::PointerId(pid.into())
    }
}
impl From<Vec<Endpoint>> for Receivers {
    fn from(endpoints: Vec<Endpoint>) -> Self {
        Receivers::from(endpoints.as_slice())
    }
}
impl From<&Vec<Endpoint>> for Receivers {
    fn from(endpoints: &Vec<Endpoint>) -> Self {
        Receivers::from(endpoints.as_slice())
    }
}
impl From<&[Endpoint]> for Receivers {
    fn from(endpoints: &[Endpoint]) -> Self {
        if endpoints.is_empty() {
            Receivers::None
        } else {
            Receivers::Endpoints(endpoints.to_vec())
        }
    }
}
impl<T> From<Vec<(Endpoint, T)>> for Receivers
where
    T: Into<Key512>,
{
    fn from(endpoints_with_keys: Vec<(Endpoint, T)>) -> Self {
        if endpoints_with_keys.is_empty() {
            Receivers::None
        } else {
            Receivers::EndpointsWithKeys(
                endpoints_with_keys
                    .into_iter()
                    .map(|(ep, key)| (ep, key.into()))
                    .collect(),
            )
        }
    }
}

impl RoutingHeader {
    pub fn with_sender(&mut self, sender: Endpoint) -> &mut Self {
        self.sender = sender;
        self
    }
    pub fn with_receivers(&mut self, receivers: Receivers) -> &mut Self {
        self.set_receivers(receivers);
        self
    }
    pub fn with_ttl(&mut self, ttl: u8) -> &mut Self {
        self.ttl = ttl;
        self
    }
}

impl RoutingHeader {
    pub fn new(
        ttl: u8,
        flags: Flags,
        sender: Endpoint,
        receivers: Receivers,
    ) -> Self {
        let mut routing_header = RoutingHeader {
            sender,
            ttl,
            flags,
            ..RoutingHeader::default()
        };
        routing_header.set_receivers(receivers);
        routing_header
    }

    pub fn set_size(&mut self, size: u16) {
        self.block_size = size;
    }

    pub fn set_receivers(&mut self, receivers: Receivers) {
        self.receivers_endpoints = None;
        self.receivers_pointer_id = None;
        self.receivers_endpoints_with_keys = None;
        self.flags.set_receiver_type(ReceiverType::None);

        match receivers {
            Receivers::PointerId(pid) => {
                self.receivers_pointer_id = Some(pid);
                self.flags.set_receiver_type(ReceiverType::Pointer);
            }
            Receivers::Endpoints(endpoints) => {
                if !endpoints.is_empty() {
                    self.receivers_endpoints =
                        Some(ReceiverEndpoints::new(endpoints));
                    self.flags.set_receiver_type(ReceiverType::Receivers);
                }
            }
            Receivers::EndpointsWithKeys(endpoints_with_keys) => {
                if !endpoints_with_keys.is_empty() {
                    self.receivers_endpoints_with_keys = Some(
                        ReceiverEndpointsWithKeys::new(endpoints_with_keys),
                    );
                    self.flags
                        .set_receiver_type(ReceiverType::ReceiversWithKeys);
                }
            }
            Receivers::None => {}
        }
    }

    /// Get the receivers from the routing header
    pub fn receivers(&self) -> Receivers {
        if let Some(pid) = &self.receivers_pointer_id {
            Receivers::PointerId(pid.clone())
        } else if let Some(endpoints) = &self.receivers_endpoints
            && endpoints.count > 0
        {
            Receivers::Endpoints(endpoints.endpoints.clone())
        } else if let Some(endpoints_with_keys) =
            &self.receivers_endpoints_with_keys
            && endpoints_with_keys.count > 0
        {
            Receivers::EndpointsWithKeys(
                endpoints_with_keys.endpoints_with_keys.clone(),
            )
        } else {
            Receivers::None
        }
    }
}

#[cfg(test)]
mod tests {
    use core::str::FromStr;

    use super::*;
    #[test]
    fn single_receiver() {
        let routing_header = RoutingHeader::default()
            .with_sender(Endpoint::from_str("@jonas").unwrap())
            .with_ttl(64)
            .with_receivers(Receivers::Endpoints(vec![
                Endpoint::from_str("@alice").unwrap(),
            ]))
            .to_owned();
        assert_eq!(
            routing_header.sender,
            Endpoint::from_str("@jonas").unwrap()
        );
        assert_eq!(routing_header.ttl, 64);
        assert_eq!(
            routing_header.receivers(),
            Receivers::Endpoints(vec![Endpoint::from_str("@alice").unwrap()])
        );
        assert_eq!(
            routing_header.flags.receiver_type(),
            ReceiverType::Receivers
        );
    }

    #[test]
    fn multiple_receivers() {
        let routing_header = RoutingHeader::default()
            .with_receivers(Receivers::Endpoints(vec![
                Endpoint::from_str("@alice").unwrap(),
                Endpoint::from_str("@bob").unwrap(),
            ]))
            .to_owned();
        assert_eq!(
            routing_header.receivers(),
            Receivers::Endpoints(vec![
                Endpoint::from_str("@alice").unwrap(),
                Endpoint::from_str("@bob").unwrap(),
            ])
        );
        assert_eq!(
            routing_header.flags.receiver_type(),
            ReceiverType::Receivers
        );
    }

    #[test]
    fn no_receivers() {
        let routing_header = RoutingHeader::default()
            .with_receivers(Receivers::None)
            .to_owned();
        assert_eq!(routing_header.receivers(), Receivers::None);

        let routing_header = RoutingHeader::default()
            .with_receivers(Receivers::Endpoints(vec![]))
            .to_owned();
        assert_eq!(routing_header.receivers(), Receivers::None);
        assert_eq!(routing_header.flags.receiver_type(), ReceiverType::None);
    }
}
