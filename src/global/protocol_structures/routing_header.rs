use std::fmt::Display;

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

impl Display for PointerId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "PointerId({}, {:?}, {}, {}, {})",
            self.pointer_type,
            self.identifier,
            self.instance,
            self.timestamp,
            self.counter
        )
    }
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
    count: u8,
    #[br(count = count)]
    pub endpoints_with_keys: Vec<(Endpoint, [u8; 512])>,
}
impl ReceiverEndpointsWithKeys {
    pub fn new(endpoints_with_keys: Vec<(Endpoint, [u8; 512])>) -> Self {
        let count = endpoints_with_keys.len() as u8;
        ReceiverEndpointsWithKeys {
            count,
            endpoints_with_keys,
        }
    }
}

// min: 11 byte + 2 byte + 21 byte + 1 byte = 35 bytes
#[derive(Debug, Clone, BinWrite, BinRead, PartialEq)]
#[brw(little, magic = b"\x01\x64")]
pub struct RoutingHeader {
    pub version: u8,
    pub block_size: u16,
    pub flags: Flags,

    #[brw(if(flags.has_checksum()))]
    checksum: u32,

    pub distance: i8,
    pub ttl: u8,

    pub sender: Endpoint,

    // TODO #115: add custom match receiver queries
    #[brw(if(flags.receiver_type() == ReceiverType::Pointer))]
    receivers_pointer_id: Option<PointerId>,

    #[brw(if(flags.receiver_type() == ReceiverType::Receivers))]
    receivers_endpoints: Option<ReceiverEndpoints>,
    #[brw(if(flags.receiver_type() == ReceiverType::ReceiversWithKeys))]
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
            checksum: 0,
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
    PointerId(PointerId),
    Endpoints(Vec<Endpoint>),
    EndpointsWithKeys(Vec<(Endpoint, [u8; 512])>),
}
impl Display for Receivers {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Receivers::None => write!(f, "No receivers"),
            Receivers::PointerId(pid) => write!(f, "PointerId: {}", pid),
            Receivers::Endpoints(endpoints) => {
                write!(f, "Endpoints: {:?}", endpoints)
            }
            Receivers::EndpointsWithKeys(endpoints_with_keys) => {
                write!(f, "Endpoints with keys: {:?}", endpoints_with_keys)
            }
        }
    }
}

impl From<PointerId> for Receivers {
    fn from(pid: PointerId) -> Self {
        Receivers::PointerId(pid)
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
        if endpoints.len() == 0 {
            Receivers::None
        } else {
            Receivers::Endpoints(endpoints.to_vec())
        }
    }
}
impl From<Vec<(Endpoint, [u8; 512])>> for Receivers {
    fn from(endpoints_with_keys: Vec<(Endpoint, [u8; 512])>) -> Self {
        if endpoints_with_keys.len() == 0 {
            Receivers::None
        } else {
            Receivers::EndpointsWithKeys(endpoints_with_keys)
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
            Receivers::PointerId(pid) => self.receivers_pointer_id = Some(pid),
            Receivers::Endpoints(endpoints) => {
                if endpoints.len() > 0 {
                    self.receivers_endpoints =
                        Some(ReceiverEndpoints::new(endpoints));
                    self.flags.set_receiver_type(ReceiverType::Receivers);
                }
            }
            Receivers::EndpointsWithKeys(endpoints_with_keys) => {
                if endpoints_with_keys.len() > 0 {
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
    use std::str::FromStr;

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
