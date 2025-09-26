use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PointerAddress {
    // pointer with the local endpoint as origin
    // the full pointer id consists of the local endpoint id + this local id
    Local([u8; 5]),
    // pointer with a remote endpoint as origin, contains the full pointers address
    Remote([u8; 26]),
    // globally unique internal pointer, e.g. for #core, #std
    Internal([u8; 3]),
}
impl From<String> for PointerAddress {
    fn from(s: String) -> Self {
        PointerAddress::from(s.as_str())
    }
}
impl From<&str> for PointerAddress {
    fn from(s: &str) -> Self {
        if !s.starts_with('$') {
            panic!("PointerAddress must start with '$'");
        }
        let hex_str = &s[1..];
        let bytes =
            hex::decode(hex_str).expect("PointerAddress must be valid hex");
        match bytes.len() {
            5 => {
                let mut arr = [0u8; 5];
                arr.copy_from_slice(&bytes);
                PointerAddress::Local(arr)
            }
            26 => {
                let mut arr = [0u8; 26];
                arr.copy_from_slice(&bytes);
                PointerAddress::Remote(arr)
            }
            3 => {
                let mut arr = [0u8; 3];
                arr.copy_from_slice(&bytes);
                PointerAddress::Internal(arr)
            }
            _ => panic!("PointerAddress must be 5, 26 or 3 bytes long"),
        }
    }
}

impl Display for PointerAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "$")?;
        match self {
            PointerAddress::Local(bytes) => {
                write!(f, "{}", hex::encode(bytes))
            }
            PointerAddress::Remote(bytes) => {
                write!(f, "{}", hex::encode(bytes))
            }
            PointerAddress::Internal(bytes) => {
                write!(f, "{}", hex::encode(bytes))
            }
        }
    }
}
impl Serialize for PointerAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for PointerAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(PointerAddress::from(s.as_str()))
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
