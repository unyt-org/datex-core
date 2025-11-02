use core::prelude::rust_2024::*;
use core::result::Result;
use serde::{Deserialize, Serialize};
use core::fmt::Display;
use crate::stdlib::string::String;
use crate::stdlib::format;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PointerAddress {
    // pointer with the local endpoint as origin
    // the full pointer id consists of the local endpoint id + this local id
    Local([u8; 5]),
    // pointer with a remote endpoint as origin, contains the full pointers address
    Remote([u8; 26]),
    // globally unique internal pointer, e.g. for #core, #std
    Internal([u8; 3]), // TODO #312 shrink down to 2 bytes?
}
impl TryFrom<String> for PointerAddress {
    type Error = &'static str;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        PointerAddress::try_from(s.as_str())
    }
}
impl TryFrom<&str> for PointerAddress {
    type Error = &'static str;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let hex_str = if let Some(stripped) = s.strip_prefix('$') {
            stripped
        } else {
            s
        };
        let bytes = hex::decode(hex_str).map_err(|_| "Invalid hex string")?;
        match bytes.len() {
            5 => {
                let mut arr = [0u8; 5];
                arr.copy_from_slice(&bytes);
                Ok(PointerAddress::Local(arr))
            }
            26 => {
                let mut arr = [0u8; 26];
                arr.copy_from_slice(&bytes);
                Ok(PointerAddress::Remote(arr))
            }
            3 => {
                let mut arr = [0u8; 3];
                arr.copy_from_slice(&bytes);
                Ok(PointerAddress::Internal(arr))
            }
            _ => Err("PointerAddress must be 5, 26 or 3 bytes long"),
        }
    }
}

impl PointerAddress {
    pub fn to_address_string(&self) -> String {
        match self {
            PointerAddress::Local(bytes) => hex::encode(bytes),
            PointerAddress::Remote(bytes) => hex::encode(bytes),
            PointerAddress::Internal(bytes) => hex::encode(bytes),
        }
    }
}

impl Display for PointerAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::write!(f, "$")?;
        core::write!(f, "{}", self.to_address_string())
    }
}
impl Serialize for PointerAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let addr_str = self.to_address_string();
        serializer.serialize_str(&addr_str)
    }
}
impl<'de> Deserialize<'de> for PointerAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        PointerAddress::try_from(s.as_str()).map_err(|e| {
            serde::de::Error::custom(format!(
                "Failed to parse PointerAddress: {}",
                e
            ))
        })
    }
}

impl PointerAddress {
    pub fn bytes(&self) -> &[u8] {
        match self {
            PointerAddress::Local(bytes) => bytes,
            PointerAddress::Remote(bytes) => bytes,
            PointerAddress::Internal(bytes) => bytes,
        }
    }
}
