use std::{
  collections::VecDeque,
  sync::{Arc, Mutex},
};

use crate::{
  global::dxb_block::{DXBBlock, HeaderParsingError},
  utils::logger::Logger,
};

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
  current_block_specified_length: Option<u32>,

  logger: Option<Logger>,
}

impl Default for BlockCollector {
  fn default() -> Self {
    BlockCollector {
      receive_queue: Arc::new(Mutex::new(VecDeque::new())),
      block_queue: VecDeque::new(),
      current_block: Vec::new(),
      current_block_specified_length: None,
      logger: None,
    }
  }
}

impl BlockCollector {
  pub fn new<'a>(receive_queue: Arc<Mutex<VecDeque<u8>>>) -> BlockCollector {
    BlockCollector {
      receive_queue,
      ..Default::default()
    }
  }
  pub fn new_with_logger<'a>(
    receive_queue: Arc<Mutex<VecDeque<u8>>>,
    logger: Option<Logger>,
  ) -> BlockCollector {
    BlockCollector {
      receive_queue,
      logger,
      ..Default::default()
    }
  }

  pub fn get_block_queue(&self) -> &VecDeque<DXBBlock> {
    &self.block_queue
  }

  fn receive_slice(&mut self, slice: &[u8]) {
    if let Some(logger) = &self.logger {
      logger.info(&format!("Received slice of size {:?}", slice.len()));
    }

    // Add the received data to the current block.
    self.current_block.extend_from_slice(slice);

    while self.current_block.len() > 0 {
      if let Some(logger) = &self.logger {
        logger.info(&format!("length_result A {:?}", self.current_block.len()));
      }

      // Extract the block length from the header if it is not already known.
      if self.current_block_specified_length.is_none() {
        let length_result =
          DXBBlock::extract_dxb_block_length(&self.current_block);
        if let Some(logger) = &self.logger {
          logger.info(&format!("length_result B {:?}", length_result));
        }

        match length_result {
          Ok(length) => {
            self.current_block_specified_length = Some(length);
          }
          Err(HeaderParsingError::InsufficientLength) => {
            break;
          }
          Err(err) => {
            if let Some(logger) = &self.logger {
              logger
                .error(&format!("Received invalid block header: {:?}", err));
            }
            self.current_block.clear();
            self.current_block_specified_length = None;
          }
        }
      }

      // If the block length is specified and the current block is long enough, extract the block.
      if let Some(specified_length) = self.current_block_specified_length {
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
              if let Some(logger) = &self.logger {
                logger
                  .error(&format!("Received invalid block header: {:?}", err));
              }
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
    if let Some(logger) = &self.logger {
      logger.success(&format!("Update block collector (length={})", len));
    }
    if len == 0 {
      return;
    }
    let range = 0..len;
    let slice = receive_queue.drain(range).collect::<Vec<u8>>();
    self.receive_slice(&slice);
  }
}
