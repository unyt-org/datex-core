use num_enum::TryFromPrimitive;

use crate::datex_values::Endpoint;

#[derive(Debug)]
pub struct DXBBlock {
    pub header: DXBHeader,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DXBHeader {
    pub version: u8,

    pub size: u16,

    pub signed: bool,
    pub encrypted: bool,

    pub timestamp: u64,

    pub scope_id: u32,
    pub block_index: u16,
    pub block_increment: u16,
    pub block_type: DXBBlockType,
    pub flags: HeaderFlags,

    pub routing: RoutingInfo,

    pub body_start_offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum DXBBlockType {
    REQUEST = 0,  // default datex request
    RESPONSE = 1, // response to a request (can be empty)

    DATA = 2,      // data only (limited execution permission)
    TMP_SCOPE = 3, // resettable scope

    LOCAL = 4, // default datex request, but don't want a response (use for <Function> code blocks, ....), must be sent and executed on same endpoint

    HELLO = 5,      // info message that endpoint is online
    DEBUGGER = 6,   // get a debugger for a scope
    SOURCE_MAP = 7, // send a source map for a scope
    UPDATE = 8, // like normal request, but don't propgate updated pointer values back to sender (prevent recursive loop)
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeaderFlags {
    pub allow_execute: bool,
    pub end_of_scope: bool,
    pub device_type: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoutingInfo {
    pub ttl: u8,
    pub priority: u8,

    pub sender: Option<Endpoint>,
    // pub receivers: Disjunction<Endpoint>
}
