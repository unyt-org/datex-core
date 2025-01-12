use std::{collections::VecDeque, sync::{Arc, Mutex}};

use crate::{global::dxb_block::DXBBlock, parser::header::{extract_dxb_block_length, parse_dxb_header, HeaderParsingError}};

pub struct BlockCollector {
	receive_queue: Arc<Mutex<VecDeque<u8>>>,
	/**
	 * Full DATEX blocks are stored in this queue.
	 */
	block_queue: VecDeque<DXBBlock>,
	/**
	 * The current block being received.
	 */
	current_block: Vec<u8>,
	/**
	 * The length of the current block as specified by the block header.
	 */
	current_block_specified_length: Option<u16>,
}

impl BlockCollector {
	pub fn new(receive_queue: Arc<Mutex<VecDeque<u8>>>) -> BlockCollector {
		BlockCollector {
			receive_queue,
			block_queue: VecDeque::new(),
			current_block: Vec::new(),
			current_block_specified_length: None,
		}
	}

	pub fn get_block_queue(&self) -> &VecDeque<DXBBlock> {
		&self.block_queue
	}

    fn receive_slice(&mut self, slice: &[u8]) {

		// Add the received data to the current block.
		self.current_block.extend_from_slice(slice);

		// Extract the block length from the header if it is not already known.
		if self.current_block_specified_length.is_none() {
			let length_result = extract_dxb_block_length(&self.current_block);
			match length_result {
				Ok(length) => {
					self.current_block_specified_length = Some(length);
				}
				Err(HeaderParsingError::InsufficientLength) => (),
				Err(_) => {
					println!("Received invalid block header.");
					self.current_block.clear();
					self.current_block_specified_length = None;
				}
			}
		}

		// If the block length is specified and the current block is long enough, extract the block.
		if let Some(specified_length) = self.current_block_specified_length {
        	if self.current_block.len() >= specified_length as usize {
				let block_slice = self.current_block.drain(0..specified_length as usize).collect::<Vec<u8>>();

				let header_result = parse_dxb_header(&block_slice);

				match header_result {
					Ok(header) => {
						let block = DXBBlock {
							header,
							body: block_slice,
						};
						self.block_queue.push_back(block);
						self.current_block_specified_length = None;
					}
					Err(_) => {
						println!("Received invalid block header.");
						self.current_block.clear();
						self.current_block_specified_length = None;
					}
				}
				
			}
		}
    }


	pub fn update(&mut self) {
		let queue = self.receive_queue.clone();
		let mut receive_queue = queue.lock().unwrap();
		let range = 0..receive_queue.len();
		let slice = receive_queue.drain(range).collect::<Vec<u8>>();
		self.receive_slice(&slice);
	}
}