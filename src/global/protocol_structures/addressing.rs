use binrw::{BinRead, BinWrite};

#[derive(Debug, Clone, PartialEq, Default)]
#[derive(BinWrite, BinRead)]
pub struct Endpoint {
    pub endpoint_type: EndpointType,
    pub endpoint_id: [u8; 18],
    pub instance: u16,
}

#[derive(Debug, PartialEq, Clone, Default)]
#[derive(BinWrite, BinRead)]
#[brw(repr(u8))]
pub enum EndpointType {
    Person = 0,
    Institution = 1,
    Anonymous = 2,
    #[default]
    Any = 255,
}

#[derive(Debug, Clone, Default)]
#[derive(BinWrite, BinRead)]
pub struct Sender {
    pub sender_type: EndpointType,
    #[br(if(sender_type != EndpointType::Any))]
    pub sender_id: [u8; 20],
}
