use binrw::{BinRead, BinWrite};

// 1 byte + 18 byte + 2 byte = 21 byte
#[derive(Debug, Clone, PartialEq, Default, BinWrite, BinRead)]
pub struct Endpoint {
    pub type_: EndpointType,
    pub identifier: [u8; 18],
    pub instance: u16,
}

// 1 byte
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Default, BinWrite, BinRead)]
#[brw(repr(u8))]
pub enum EndpointType {
    Person = 0,
    Institution = 1,
    Anonymous = 2,
    #[default]
    Any = 255,
}

// min: 1 byte
// max: 21 byte
#[derive(Debug, Clone, Default, BinWrite, BinRead, PartialEq)]
pub struct Sender {
    pub sender_type: EndpointType,
    #[brw(if(sender_type.clone() != EndpointType::Any))]
    pub sender_id: [u8; 20],
}
