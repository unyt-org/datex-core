use std::io::{Cursor, Read};

use anyhow::Result;
use binrw::{BinRead, BinWrite};
use strum::Display;
use thiserror::Error;

use crate::utils::buffers::{clear_bit, set_bit, write_u16, write_u32};

use super::protocol_structures::{
    block_header::BlockHeader,
    encrypted_header::EncryptedHeader,
    routing_header::{
        self, BlockSize, EncryptionType, RoutingHeader, SignatureType,
    },
};

#[derive(Debug, Display, Error)]
pub enum HeaderParsingError {
    InvalidBlock,
    InsufficientLength,
}

// TODO fix partial eq
#[derive(Debug, Clone)]
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

impl Default for DXBBlock {
    fn default() -> Self {
        DXBBlock {
            routing_header: RoutingHeader::default(),
            block_header: BlockHeader::default(),
            encrypted_header: EncryptedHeader::default(),
            body: Vec::new(),
            raw_bytes: None,
        }
    }
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

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut writer = Cursor::new(Vec::new());
        self.routing_header.write(&mut writer)?;
        self.block_header.write(&mut writer)?;
        self.encrypted_header.write(&mut writer)?;
        let mut bytes = writer.into_inner();
        bytes.extend_from_slice(&self.body);
        return Ok(DXBBlock::adjust_block_length(bytes, &self.routing_header));
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
            if routing_header.flags.block_size() == BlockSize::Large {
                bytes.remove(13);
            }
            write_u16(&mut bytes, &mut 13, size as u16);
        } else {
            if routing_header.flags.block_size() == BlockSize::Default {
                bytes.insert(13, 0);
            }
            write_u32(&mut bytes, &mut 13, size);
        }
        if is_small_size {
            clear_bit(&mut bytes, 5, 3);
        } else {
            set_bit(&mut bytes, 5, 3);
        }
        bytes
    }

    pub fn has_dxb_magic_number(dxb: &[u8]) -> bool {
        dxb.len() >= 2 && dxb[0] == 0x01 && dxb[1] == 0x64
    }

    pub fn extract_dxb_block_length(
        dxb: &[u8],
    ) -> Result<u32, HeaderParsingError> {
        if dxb.len() < 6 {
            return Err(HeaderParsingError::InsufficientLength.into());
        }
        let routing_header = RoutingHeader::read(&mut Cursor::new(dxb))
            .map_err(|_| HeaderParsingError::InvalidBlock)?;
        if routing_header.block_size_u16.is_some() {
            return Ok(routing_header.block_size_u16.unwrap() as u32);
        } else {
            return Ok(routing_header.block_size_u32.unwrap());
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<DXBBlock> {
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
            SignatureType::Invalid => todo!(),
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

        return Ok(DXBBlock {
            routing_header,
            block_header,
            encrypted_header,
            body,
            raw_bytes: Some(bytes.to_vec()),
        });
    }
}
