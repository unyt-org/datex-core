use crate::{datex_values::Endpoint, utils::buffers::{read_slice, read_u16, read_u32, read_u64, read_u8}};
use num_enum::TryFromPrimitive;
use crate::utils::buffers::{append_u16, append_u32, append_u64, append_u8};

#[derive(Debug, Clone, PartialEq)]
pub struct HeaderFlags {
    pub allow_execute: bool,
    pub end_of_scope: bool,
    pub device_type: u8,
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
pub struct RoutingInfo {
    pub ttl: u8,
    pub priority: u8,

    pub sender: Option<Endpoint>,
    // pub receivers: Disjunction<Endpoint>
}

#[derive(Debug)]
pub enum HeaderParsingError {
	InvalidMagicNumber,
	InsufficientLength,
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

impl Default for DXBHeader {
    fn default() -> Self { 
        DXBHeader {
            version: 0,
            size: 29,
            signed: false,
            encrypted: false,
            timestamp: 0,
            scope_id: 0,
            block_index: 0,
            block_increment: 0,
            block_type: DXBBlockType::REQUEST,
            flags: HeaderFlags {
                allow_execute: false,
                end_of_scope: false,
                device_type: 0,
            },
            routing: RoutingInfo {
                ttl: 0,
                priority: 0,
                sender: None,
            },
            body_start_offset: 0,
        }
    }
}


impl DXBHeader {

		
	// checks magic number
	pub fn has_dxb_magic_number(dxb: &[u8]) -> bool {
		dxb.len() >= 2 && dxb[0] == 0x01 && dxb[1] == 0x64
	}

	pub fn extract_dxb_block_length(dxb: &[u8]) -> Result<u16, HeaderParsingError> {
		if !DXBHeader::has_dxb_magic_number(dxb) {return Err(HeaderParsingError::InvalidMagicNumber)}
		if dxb.len() < 6 {return Err(HeaderParsingError::InsufficientLength)}
		return Ok(read_u16(dxb, &mut 3));
	}
	
	pub fn from_bytes<'a>(dxb: &'a [u8]) -> Result<DXBHeader, HeaderParsingError> {
		// has magic number?
		if !DXBHeader::has_dxb_magic_number(dxb) {
			return Err(HeaderParsingError::InvalidMagicNumber);
		}
		// header to short
		if dxb.len() < 28 {
			return Err(HeaderParsingError::InsufficientLength);
		}
	
		let index = &mut 2;
	
		// pre header
		let version = read_u8(dxb, index);
		let size = read_u16(dxb, index);
		let ttl = read_u8(dxb, index);
		let priority = read_u8(dxb, index);
		let signed_encrypted = read_u8(dxb, index);
		let signed = signed_encrypted == 1 || signed_encrypted == 2; // is signed?
		let encrypted = signed_encrypted == 2 || signed_encrypted == 3; // is encrypted?
		let sender = DXBHeader::get_dxb_header_sender(dxb, index);
		let _receivers = DXBHeader::get_dxb_header_receivers(dxb, index);
	
		// block header
		let scope_id = read_u32(dxb, index);
		let block_index = read_u16(dxb, index);
		let block_increment = read_u16(dxb, index);
		let block_type = DXBBlockType::try_from(read_u8(dxb, index)).expect("Invalid DXB block type");
		let _flags = read_u8(dxb, index); // TODO: parse
		let timestamp = read_u64(dxb, index);
	
		let header = DXBHeader {
			version,
			size,
			signed,
			encrypted,
	
			scope_id,
			block_index,
			block_increment,
			block_type,
			timestamp,
	
			flags: HeaderFlags {
				allow_execute: true,
				end_of_scope: true,
				device_type: 0,
			},
			routing: RoutingInfo {
				ttl,
				sender,
				priority,
			},
			body_start_offset: *index,
		};
	
		return Ok(header);
	}
	
	fn get_dxb_header_sender(dxb: &[u8], index: &mut usize) -> Option<Endpoint> {
		if read_u8(dxb, index) == std::u8::MAX {
			return None;
		} else {
			*index -= 1;
			return Some(Endpoint::new_from_binary(&read_slice(dxb, index, 21)));
		}
	}
	
	fn get_dxb_header_receivers(dxb: &[u8], index: &mut usize) -> Option<Endpoint> {
		if read_u16(dxb, index) == std::u16::MAX {
			return None;
		} else {
			// TODO:
			return None;
		}
	}
	

	pub fn block_header_to_bytes(&self) -> Vec<u8> {
		let block_header = &mut Vec::<u8>::with_capacity(200);

		// sid
		append_u32(block_header, self.scope_id);
		// block index
		append_u16(block_header, self.block_index);
		// block increment
		append_u16(block_header, self.block_increment);

		// type
		append_u8(block_header, self.block_type as u8);

		// TODO: flags
		append_u8(block_header, 0);

		// timestamp
		append_u64(block_header, self.timestamp);

		return block_header.to_vec();
	}

	pub fn pre_header_to_bytes(&self) -> Vec<u8> {
		let pre_header = &mut Vec::<u8>::with_capacity(200);

		let _index = &mut 0;

		// magic number
		append_u8(pre_header, 0x01);
		append_u8(pre_header, 0x64);

		// version
		append_u8(pre_header, self.version);

		// size
		append_u16(pre_header, self.size);
		// routing
		append_u8(pre_header, self.routing.ttl);
		append_u8(pre_header, self.routing.priority);

		// signed/encrypted
		let signed_encrypted = if self.signed && self.encrypted {
			2
		} else if self.signed {
			1
		} else if self.encrypted {
			3
		} else {
			0
		};
		append_u8(pre_header, signed_encrypted);

		// sender
		if self.routing.sender.is_some() {
			pre_header.extend_from_slice(&self.routing.sender.as_ref().unwrap().get_binary());
		}
		// no sender - anonymous
		else {
			append_u8(pre_header, std::u8::MAX); // 0xff
		}

		// TODO:
		// no receiver - flood
		// else {
		// 	append_u16(dxb, std::u16::MAX); // 0xffff
		// }
		append_u16(pre_header, std::u16::MAX); // 0xffff

		return pre_header.to_vec();
	}

}