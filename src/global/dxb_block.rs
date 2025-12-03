use super::protocol_structures::{
    block_header::BlockHeader,
    encrypted_header::EncryptedHeader,
    routing_header::{EncryptionType, RoutingHeader, SignatureType},
};
use crate::global::protocol_structures::routing_header::Receivers;
use crate::stdlib::vec::Vec;
use crate::task::UnboundedReceiver;
use crate::utils::buffers::write_u16;
use crate::values::core_values::endpoint::Endpoint;
use binrw::io::{Cursor, Read};
use binrw::{BinRead, BinWrite};
use core::fmt::Display;
use core::prelude::rust_2024::*;
use core::result::Result;
use core::unimplemented;
use log::error;
use strum::Display;
use thiserror::Error;

#[derive(Debug, Display, Error)]
pub enum HeaderParsingError {
    InvalidBlock,
    InsufficientLength,
}

// TODO #110: RawDXBBlock that is received in com_hub, only containing RoutingHeader, BlockHeader and raw bytes

// TODO #429 @Norbert
// Add optional raw signature, and encrypted part
#[cfg_attr(feature = "debug", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct DXBBlock {
    pub routing_header: RoutingHeader,
    pub block_header: BlockHeader,
    pub signature: Option<Vec<u8>>,
    pub encrypted_header: EncryptedHeader,
    pub body: Vec<u8>,

    #[cfg_attr(feature = "debug", serde(skip))]
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

const SIZE_BYTE_POSITION: usize = 3; // magic number (2 bytes) + version (1 byte)
const SIZE_BYTES: usize = 2;

pub type IncomingContextId = u32;
pub type IncomingSectionIndex = u16;
pub type IncomingBlockNumber = u16;
pub type OutgoingContextId = u32;
pub type OutgoingSectionIndex = u16;
pub type OutgoingBlockNumber = u16;

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum IncomingSection {
    /// a single block
    SingleBlock((Option<DXBBlock>, IncomingEndpointContextSectionId)),
    /// a stream of blocks
    /// the stream is finished when a block has the end_of_block flag set
    BlockStream(
        (
            Option<UnboundedReceiver<DXBBlock>>,
            IncomingEndpointContextSectionId,
        ),
    ),
}

impl IncomingSection {
    pub async fn next(&mut self) -> Option<DXBBlock> {
        match self {
            IncomingSection::SingleBlock((block, _)) => block.take(),
            IncomingSection::BlockStream((blocks, _)) => {
                if let Some(receiver) = blocks {
                    receiver.next().await
                } else {
                    None // No blocks to receive
                }
            }
        }
    }

    pub async fn drain(&mut self) -> Vec<DXBBlock> {
        let mut blocks = Vec::new();
        while let Some(block) = self.next().await {
            blocks.push(block);
        }
        blocks
    }
}

impl IncomingSection {
    pub fn get_section_index(&self) -> IncomingSectionIndex {
        self.get_section_context_id().section_index
    }

    pub fn get_sender(&self) -> Endpoint {
        self.get_section_context_id()
            .endpoint_context_id
            .sender
            .clone()
    }

    pub fn get_section_context_id(&self) -> &IncomingEndpointContextSectionId {
        match self {
            IncomingSection::SingleBlock((_, section_context_id))
            | IncomingSection::BlockStream((_, section_context_id)) => {
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
    pub timestamp: u64,
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
            signature: None,
            encrypted_header,
            body,
            raw_bytes: None,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, binrw::Error> {
        let mut writer = Cursor::new(Vec::new());
        self.routing_header.write(&mut writer)?;
        self.signature.write(&mut writer)?;
        self.block_header.write(&mut writer)?;
        self.encrypted_header.write(&mut writer)?;
        let mut bytes = writer.into_inner();
        bytes.extend_from_slice(&self.body);
        Ok(DXBBlock::adjust_block_length(bytes))
    }
    pub fn recalculate_struct(&mut self) -> &mut Self {
        let bytes = self.to_bytes().unwrap();
        let size = bytes.len() as u16;
        self.routing_header.block_size = size;
        self
    }

    fn adjust_block_length(mut bytes: Vec<u8>) -> Vec<u8> {
        let size = bytes.len() as u32;
        write_u16(&mut bytes, &mut SIZE_BYTE_POSITION.clone(), size as u16);
        bytes
    }

    pub fn has_dxb_magic_number(dxb: &[u8]) -> bool {
        dxb.len() >= 2 && dxb[0] == 0x01 && dxb[1] == 0x64
    }

    pub fn extract_dxb_block_length(
        dxb: &[u8],
    ) -> Result<u16, HeaderParsingError> {
        if dxb.len() < SIZE_BYTE_POSITION + SIZE_BYTES {
            return Err(HeaderParsingError::InsufficientLength);
        }
        let routing_header = RoutingHeader::read(&mut Cursor::new(dxb))
            .map_err(|e| {
                error!("Failed to read routing header: {e:?}");
                HeaderParsingError::InvalidBlock
            })?;
        Ok(routing_header.block_size)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<DXBBlock, binrw::Error> {
        let mut reader = Cursor::new(bytes);
        let routing_header = RoutingHeader::read(&mut reader)?;

        let signature = match routing_header.flags.signature_type() {
            SignatureType::Encrypted => {
                // extract next 255 bytes as the signature
                let mut signature = Vec::from([0u8; 108]);
                reader.read_exact(&mut signature)?;

                // TODO #111: decrypt the signature
                Some(signature)
            }
            SignatureType::Unencrypted => {
                // extract next 255 bytes as the signature
                let mut signature = Vec::from([0u8; 108]);
                reader.read_exact(&mut signature)?;
                Some(signature)
            }
            SignatureType::None => None,
        };

        // TODO #112: validate the signature
        let decrypted_bytes = match routing_header.flags.encryption_type() {
            EncryptionType::Encrypted => {
                // TODO #113: decrypt the body
                let mut decrypted_bytes = Vec::from([0u8; 255]);
                reader.read_exact(&mut decrypted_bytes)?;
                decrypted_bytes
            }
            EncryptionType::None => {
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
            signature,
            encrypted_header,
            body,
            raw_bytes: Some(bytes.to_vec()),
        })
    }

    /// Get a list of all receiver endpoints from the routing header.
    pub fn receiver_endpoints(&self) -> Vec<Endpoint> {
        match self.routing_header.receivers() {
            Receivers::Endpoints(endpoints) => endpoints,
            Receivers::EndpointsWithKeys(endpoints_with_keys) => {
                endpoints_with_keys.into_iter().map(|(e, _)| e).collect()
            }
            Receivers::PointerId(_) => unimplemented!(),
            _ => Vec::new(),
        }
    }
    pub fn receivers(&self) -> Receivers {
        self.routing_header.receivers()
    }

    /// Update the receivers list in the routing header.
    pub fn set_receivers<T>(&mut self, endpoints: T)
    where
        T: Into<Receivers>,
    {
        self.routing_header.set_receivers(endpoints.into());
    }

    pub fn set_bounce_back(&mut self, bounce_back: bool) {
        self.routing_header.flags.set_is_bounce_back(bounce_back);
    }

    pub fn is_bounce_back(&self) -> bool {
        self.routing_header.flags.is_bounce_back()
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
            timestamp: self
                .block_header
                .flags_and_timestamp
                .creation_timestamp(),
            current_section_index: self.block_header.section_index,
            current_block_number: self.block_header.block_number,
        }
    }

    /// Returns true if the block has a fixed number of receivers
    /// without wildcard instances, and no @@any receiver.
    pub fn has_exact_receiver_count(&self) -> bool {
        !self
            .receiver_endpoints()
            .iter()
            .any(|e| e.is_broadcast() || e.is_any())
    }

    pub fn clone_with_new_receivers<T>(&self, new_receivers: T) -> DXBBlock
    where
        T: Into<Receivers>,
    {
        let mut new_block = self.clone();
        new_block.set_receivers(new_receivers.into());
        new_block
    }
}

impl Display for DXBBlock {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let block_type = self.block_header.flags_and_timestamp.block_type();
        let sender = &self.routing_header.sender;
        let receivers = self.receivers();
        core::write!(f, "[{block_type}] {sender} -> {receivers}")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::str::FromStr;

    use crate::{
        global::{
            dxb_block::DXBBlock,
            protocol_structures::{
                encrypted_header::{self, EncryptedHeader},
                routing_header::{RoutingHeader, SignatureType},
            },
        },
        values::core_values::endpoint::Endpoint,
    };

    #[test]
    pub fn test_recalculate() {
        let mut routing_header = RoutingHeader::default()
            .with_sender(Endpoint::from_str("@test").unwrap())
            .to_owned();
        routing_header.set_size(420);
        let mut block = DXBBlock {
            body: vec![0x01, 0x02, 0x03],
            encrypted_header: EncryptedHeader {
                flags: encrypted_header::Flags::new()
                    .with_user_agent(encrypted_header::UserAgent::Unused11),
                ..Default::default()
            },
            routing_header,
            ..DXBBlock::default()
        };

        {
            // invalid block size
            let block_bytes = block.to_bytes().unwrap();
            let block2: DXBBlock = DXBBlock::from_bytes(&block_bytes).unwrap();
            assert_ne!(block, block2);
        }

        {
            // valid block size
            block.recalculate_struct();
            let block_bytes = block.to_bytes().unwrap();
            let block3: DXBBlock = DXBBlock::from_bytes(&block_bytes).unwrap();
            println!("Block: {:?}", block);
            assert_eq!(block, block3);
        }
    }

    #[test]
    pub fn signature_to_and_from_bytes() {
        let mut routing_header = RoutingHeader::default()
            .with_sender(Endpoint::from_str("@test").unwrap())
            .to_owned();
        routing_header.set_size(157);
        let mut block = DXBBlock {
            body: vec![0x01, 0x02, 0x03],
            encrypted_header: EncryptedHeader {
                ..Default::default()
            },
            routing_header,
            ..DXBBlock::default()
        };
        block
            .routing_header
            .flags
            .set_signature_type(SignatureType::Unencrypted);
        block.signature = Some(vec![0u8; 108]);

        let block_bytes = block.to_bytes().unwrap();
        let block2: DXBBlock = DXBBlock::from_bytes(&block_bytes).unwrap();
        assert_eq!(block, block2);
        assert_eq!(block.signature, block2.signature);
    }
}
