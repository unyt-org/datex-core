
pub struct DXBBlock {
	pub header: DXBHeader
}

#[derive(Debug, Clone, PartialEq)]
pub struct DXBHeader {
	pub version: u8,

	pub size: u16,

	pub signed: bool,
	pub encrypted: bool,

	pub routing: RoutingInfo
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoutingInfo {
	pub ttl: u8,
	pub priority: u8,
}