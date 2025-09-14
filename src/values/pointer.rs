use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PointerAddress {
    // pointer with the local endpoint as origin
    // the full pointer id consists of the local endpoint id + this local id
    Local([u8; 5]),
    // pointer with a remote endpoint as origin, contains the full pointers address
    Remote([u8; 26]),
    // globally unique internal pointer, e.g. for #core, #std
    Internal([u8; 3]),
}
