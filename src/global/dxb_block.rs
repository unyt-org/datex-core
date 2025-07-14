use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::Display;
use std::io::{Cursor, Read};
use std::rc::Rc;
// FIXME no-std

use crate::values::core_values::endpoint::Endpoint;
use crate::global::protocol_structures::routing_header::ReceiverEndpoints;
use crate::utils::buffers::{clear_bit, set_bit, write_u16, write_u32};
use binrw::{BinRead, BinWrite};
use log::error;
use strum::Display;
use thiserror::Error;

use super::protocol_structures::{
    block_header::BlockHeader,
    encrypted_header::EncryptedHeader,
    routing_header::{BlockSize, EncryptionType, RoutingHeader, SignatureType},
};

#[derive(Debug, Display, Error)]
pub enum HeaderParsingError {
    InvalidBlock,
    InsufficientLength,
}

// TODO: RawDXBBlock that is received in com_hub, only containing RoutingHeader, BlockHeader and raw bytes

#[derive(Debug, Clone, Default)]
pub struct DXBBlock {
    pub routing_header: RoutingHeader,
    pub block_header: BlockHeader,
    pub encrypted_header: EncryptedHeader,
    pub body: Vec<u8>,
    pub raw_bytes: Option<Vec<u8>>,
}

impl PartialEq for DXBBlock {
    fn eq(&self, other: &Self) -> bool {
        self.routing_header == other.routing_header
            && self.block_header == other.block_header
            && self.encrypted_header == other.encrypted_header
            && self.body == other.body
    }
}

const ROUTING_HEADER_FLAGS_POSITION: usize = 5;
const SIZE_BYTE_POSITION: usize = ROUTING_HEADER_FLAGS_POSITION + 1;
const MAX_SIZE_BYTE_LENGTH: usize = 4;
const ROUTING_HEADER_FLAGS_SIZE_BIT_POSITION: u8 = 3;

pub type IncomingContextId = u32;
pub type IncomingSectionIndex = u16;
pub type IncomingBlockNumber = u16;
pub type OutgoingContextId = u32;
pub type OutgoingSectionIndex = u16;
pub type OutgoingBlockNumber = u16;

#[derive(Debug, Clone)]
pub enum IncomingSection {
    /// a single block
    SingleBlock((DXBBlock, IncomingEndpointContextSectionId)),
    /// a stream of blocks
    /// the stream is finished when a block has the end_of_block flag set
    BlockStream((Rc<RefCell<VecDeque<DXBBlock>>>, IncomingEndpointContextSectionId)),
}

#[derive(Debug)]
pub enum IncomingSectionIter {
    /// a single block
    SingleBlock(Option<DXBBlock>),
    /// a stream of blocks
    /// the stream is finished when a block has the end_of_block flag set
    BlockStream(Rc<RefCell<VecDeque<DXBBlock>>>),
}

impl IntoIterator for IncomingSection {
    type Item = DXBBlock;
    type IntoIter = IncomingSectionIter;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            IncomingSection::SingleBlock((block, ..)) => IncomingSectionIter::SingleBlock(Some(block)),
            IncomingSection::BlockStream((stream, ..)) => {
                IncomingSectionIter::BlockStream(stream)
            },
        }
    }
}

impl Iterator for IncomingSectionIter {
    type Item = DXBBlock;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            IncomingSectionIter::SingleBlock(block) => {
                block.take()
            }
            IncomingSectionIter::BlockStream(blocks) => {
                let mut blocks = blocks.borrow_mut();
                blocks.pop_front()
            }
        }
    }
}


impl IncomingSection {
    pub fn get_section_index(&self) -> IncomingSectionIndex {
        self.get_section_context_id().section_index
    }

    pub fn get_sender(&self) -> Endpoint {
        self.get_section_context_id().endpoint_context_id.sender.clone()
    }
    
    pub fn get_section_context_id(&self) -> &IncomingEndpointContextSectionId {
        match self {
            IncomingSection::SingleBlock((_, section_context_id)) | IncomingSection::BlockStream((_, section_context_id)) => {
                section_context_id
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IncomingEndpointContextId {
    pub sender: Endpoint,
    pub context_id: IncomingContextId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IncomingEndpointContextSectionId {
    pub endpoint_context_id: IncomingEndpointContextId,
    pub section_index: IncomingSectionIndex,
}

impl IncomingEndpointContextSectionId {
    pub fn new(
        endpoint_context_id: IncomingEndpointContextId,
        section_index: IncomingSectionIndex,
    ) -> Self {
        IncomingEndpointContextSectionId {
            endpoint_context_id,
            section_index,
        }
    }
}

/// An identifier that defines a globally unique block
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockId {
    pub endpoint_context_id: IncomingEndpointContextId,
    pub current_section_index: IncomingSectionIndex,
    pub current_block_number: IncomingBlockNumber,
}

impl DXBBlock {
    pub fn new(
        routing_header: RoutingHeader,
        block_header: BlockHeader,
        encrypted_header: EncryptedHeader,
        body: Vec<u8>,
    ) -> DXBBlock {
        DXBBlock {
            routing_header,
            block_header,
            encrypted_header,
            body,
            raw_bytes: None,
        }
    }
    
    pub fn to_bytes(&self) -> Result<Vec<u8>, binrw::Error> {
        let mut writer = Cursor::new(Vec::new());
        self.routing_header.write(&mut writer)?;
        self.block_header.write(&mut writer)?;
        self.encrypted_header.write(&mut writer)?;
        let mut bytes = writer.into_inner();
        bytes.extend_from_slice(&self.body);
        Ok(DXBBlock::adjust_block_length(bytes, &self.routing_header))
    }
    pub fn recalculate_struct(&mut self) -> &mut Self {
        let bytes = self.to_bytes().unwrap();
        let size = bytes.len() as u32;
        let is_small_size = size <= u16::MAX as u32;
        self.routing_header.flags.set_block_size(if is_small_size {
            BlockSize::Default
        } else {
            BlockSize::Large
        });
        self.routing_header.block_size_u16 = if is_small_size {
            Some(size as u16)
        } else {
            None
        };
        self.routing_header.block_size_u32 =
            if is_small_size { None } else { Some(size) };
        self
    }

    fn adjust_block_length(
        mut bytes: Vec<u8>,
        routing_header: &RoutingHeader,
    ) -> Vec<u8> {
        let size = bytes.len() as u32;
        let is_small_size = size <= u16::MAX as u32;

        if is_small_size {
            // replace u32 size with u16 size
            if routing_header.flags.block_size() == BlockSize::Large {
                bytes.remove(SIZE_BYTE_POSITION);
            }
            write_u16(&mut bytes, &mut SIZE_BYTE_POSITION.clone(), size as u16);
        } else {
            // replace u16 size with u32 size
            if routing_header.flags.block_size() == BlockSize::Default {
                bytes.insert(SIZE_BYTE_POSITION, 0);
            }
            write_u32(&mut bytes, &mut SIZE_BYTE_POSITION.clone(), size);
        }

        // update small size flag
        if is_small_size {
            clear_bit(
                &mut bytes,
                ROUTING_HEADER_FLAGS_POSITION,
                ROUTING_HEADER_FLAGS_SIZE_BIT_POSITION,
            );
        } else {
            set_bit(
                &mut bytes,
                ROUTING_HEADER_FLAGS_POSITION,
                ROUTING_HEADER_FLAGS_SIZE_BIT_POSITION,
            );
        }
        bytes
    }

    pub fn has_dxb_magic_number(dxb: &[u8]) -> bool {
        dxb.len() >= 2 && dxb[0] == 0x01 && dxb[1] == 0x64
    }

    pub fn extract_dxb_block_length(
        dxb: &[u8],
    ) -> Result<u32, HeaderParsingError> {
        if dxb.len() < SIZE_BYTE_POSITION + MAX_SIZE_BYTE_LENGTH {
            return Err(HeaderParsingError::InsufficientLength);
        }
        let routing_header = RoutingHeader::read(&mut Cursor::new(dxb))
            .map_err(|e| {
                error!("Failed to read routing header: {e:?}");
                HeaderParsingError::InvalidBlock
            })?;
        if routing_header.block_size_u16.is_some() {
            Ok(routing_header.block_size_u16.unwrap() as u32)
        } else {
            Ok(routing_header.block_size_u32.unwrap())
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<DXBBlock, binrw::Error> {
        let mut reader = Cursor::new(bytes);
        let routing_header = RoutingHeader::read(&mut reader)?;

        let _signature = match routing_header.flags.signature_type() {
            SignatureType::Encrypted => {
                // extract next 255 bytes as the signature
                let mut signature = Vec::with_capacity(255);
                reader.read_exact(&mut signature)?;

                // TODO: decrypt the signature
                Some(signature)
            }
            SignatureType::Unencrypted => {
                // extract next 255 bytes as the signature
                let mut signature = Vec::with_capacity(255);
                reader.read_exact(&mut signature)?;
                Some(signature)
            }
            SignatureType::None => None,
        };

        // TODO: validate the signature
        let decrypted_bytes = match routing_header.flags.encryption_type() {
            EncryptionType::Encrypted => {
                // TODO: decrypt the body
                let mut decrypted_bytes = Vec::with_capacity(255);
                reader.read_exact(&mut decrypted_bytes)?;
                decrypted_bytes
            }
            EncryptionType::Unencrypted => {
                let mut bytes = Vec::new();
                reader.read_to_end(&mut bytes)?;
                bytes
            }
        };

        let mut reader = Cursor::new(decrypted_bytes);
        let block_header = BlockHeader::read(&mut reader)?;
        let encrypted_header = EncryptedHeader::read(&mut reader)?;

        let mut body = Vec::new();
        reader.read_to_end(&mut body)?;

        Ok(DXBBlock {
            routing_header,
            block_header,
            encrypted_header,
            body,
            raw_bytes: Some(bytes.to_vec()),
        })
    }

    /// Get a list of all receiver endpoints from the routing header.
    pub fn receivers(&self) -> Option<&Vec<Endpoint>> {
        if let Some(endpoints) = &self.routing_header.receivers.endpoints {
            Some(&endpoints.endpoints)
        } else {
            None
        }
    }

    /// Update the receivers list in the routing header.
    pub fn set_receivers(&mut self, receivers: &[Endpoint]) {
        self.routing_header.receivers.endpoints =
            Some(ReceiverEndpoints::new(receivers.to_vec()));
        self.routing_header
            .receivers
            .flags
            .set_has_endpoints(!receivers.is_empty());
    }

    pub fn set_bounce_back(&mut self, bounce_back: bool) {
        self.routing_header.flags.set_is_bounce_back(bounce_back);
    }

    pub fn is_bounce_back(&self) -> bool {
        self.routing_header.flags.is_bounce_back()
    }

    pub fn get_receivers(&self) -> Vec<Endpoint> {
        if let Some(ref endpoints) = self.routing_header.receivers.endpoints {
            endpoints.endpoints.clone()
        } else if let Some(ref endpoints) =
            self.routing_header.receivers.endpoints_with_keys
        {
            endpoints
                .endpoints_with_keys
                .iter()
                .map(|(e, _)| e.clone())
                .collect()
        } else {
            unreachable!("No receivers set in the routing header")
        }
    }

    pub fn get_sender(&self) -> &Endpoint {
        &self.routing_header.sender
    }

    pub fn get_endpoint_context_id(&self) -> IncomingEndpointContextId {
        IncomingEndpointContextId {
            sender: self.routing_header.sender.clone(),
            context_id: self.block_header.context_id,
        }
    }

    pub fn get_block_id(&self) -> BlockId {
        BlockId {
            endpoint_context_id: self.get_endpoint_context_id(),
            current_section_index: self.block_header.section_index,
            current_block_number: self.block_header.block_number,
        }
    }

    /// Returns true if the block has a fixed number of receivers
    /// without wildcard instances, and no @@any receiver.
    pub fn has_exact_receiver_count(&self) -> bool {
        !self
            .get_receivers()
            .iter()
            .any(|e| e.is_broadcast() || e.is_any())
    }

    pub fn clone_with_new_receivers(
        &self,
        new_receivers: &[Endpoint],
    ) -> DXBBlock {
        let mut new_block = self.clone();
        new_block.set_receivers(new_receivers);
        new_block
    }
}

impl Display for DXBBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let block_type = self.block_header.flags_and_timestamp.block_type();
        let sender = &self.routing_header.sender;
        let receivers = self
            .receivers()
            .map(|endpoints| {
                endpoints
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or("none".to_string());

        write!(f, "[{block_type}] {sender} -> {receivers}")?;

        Ok(())
    }
}
