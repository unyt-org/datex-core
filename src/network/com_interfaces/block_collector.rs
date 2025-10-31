use log::error;

use crate::stdlib::{collections::VecDeque, sync::Arc};
use crate::stdsync::Mutex; // FIXME #192 no-std

use crate::global::dxb_block::{DXBBlock, HeaderParsingError};

#[derive(Debug)]
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

impl Default for BlockCollector {
    fn default() -> Self {
        BlockCollector {
            receive_queue: Arc::new(Mutex::new(VecDeque::new())),
            block_queue: VecDeque::new(),
            current_block: Vec::new(),
            current_block_specified_length: None,
        }
    }
}

impl BlockCollector {
    pub fn new(receive_queue: Arc<Mutex<VecDeque<u8>>>) -> BlockCollector {
        BlockCollector {
            receive_queue,
            ..Default::default()
        }
    }

    pub fn get_block_queue(&mut self) -> &mut VecDeque<DXBBlock> {
        &mut self.block_queue
    }

    fn receive_slice(&mut self, slice: &[u8]) {
        // Add the received data to the current block.
        self.current_block.extend_from_slice(slice);

        while !self.current_block.is_empty() {
            // Extract the block length from the header if it is not already known.
            if self.current_block_specified_length.is_none() {
                let length_result =
                    DXBBlock::extract_dxb_block_length(&self.current_block);

                match length_result {
                    Ok(length) => {
                        self.current_block_specified_length = Some(length);
                    }
                    Err(HeaderParsingError::InsufficientLength) => {
                        break;
                    }
                    Err(err) => {
                        error!("Received invalid block header: {err:?}");
                        self.current_block.clear();
                        self.current_block_specified_length = None;
                    }
                }
            }

            // If the block length is specified and the current block is long enough, extract the block.
            if let Some(specified_length) = self.current_block_specified_length
            {
                if self.current_block.len() >= specified_length as usize {
                    let block_slice = self
                        .current_block
                        .drain(0..specified_length as usize)
                        .collect::<Vec<u8>>();

                    let block_result = DXBBlock::from_bytes(&block_slice);

                    match block_result {
                        Ok(block) => {
                            self.block_queue.push_back(block);
                            self.current_block_specified_length = None;
                        }
                        Err(err) => {
                            error!("Received invalid block header: {err:?}");
                            self.current_block.clear();
                            self.current_block_specified_length = None;
                        }
                    }
                } else {
                    break;
                }
            }
            // otherwise, wait for more data
            else {
                break;
            }
        }
    }

    pub fn update(&mut self) {
        let queue = self.receive_queue.clone();
        let mut receive_queue = queue.lock().unwrap();
        let len = receive_queue.len();
        if len == 0 {
            return;
        }
        let range = 0..len;
        let slice = receive_queue.drain(range).collect::<Vec<u8>>();
        self.receive_slice(&slice);
    }
}
