use crate::global::dxb_block::{DXBBlock, HeaderParsingError};
use crate::network::com_interfaces::block_collector;
use crate::std_sync::Mutex;
use crate::stdlib::vec::Vec;
use crate::stdlib::{collections::VecDeque, sync::Arc};
use crate::task::{UnboundedReceiver, UnboundedSender};
use crate::task::{create_unbounded_channel, spawn_local};
use core::prelude::rust_2024::*;
use futures::StreamExt;
use log::error;

#[derive(Debug)]
pub struct BlockCollector {
    /// Incoming blocks received from the network are sent through this channel.
    bytes_in_receiver: UnboundedReceiver<Vec<u8>>,

    /// The receive queue from which the DXB blocks are collected.
    block_out_sender: UnboundedSender<DXBBlock>,

    // The current block being received.
    current_block: Vec<u8>,

    // The specified length of the current block being received, if known.
    current_block_specified_length: Option<u16>,
}

impl BlockCollector {
    async fn receive_slice(&mut self, slice: &[u8]) {
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

                    let block_result = DXBBlock::from_bytes(&block_slice).await;

                    match block_result {
                        Ok(block) => {
                            self.block_out_sender.send(block).await;
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

    /// Starts the block collector task.
    /// Returns the sender to send byte slices (socket) to and the receiver to receive collected DXB blocks (ComHub).
    pub fn init() -> (UnboundedSender<Vec<u8>>, UnboundedReceiver<DXBBlock>) {
        let (bytes_in_sender, bytes_in_receiver) = create_unbounded_channel();
        let (block_out_sender, block_out_receiver) = create_unbounded_channel();
        let block_collector = BlockCollector {
            bytes_in_receiver,
            block_out_sender,
            current_block: Vec::new(),
            current_block_specified_length: None,
        };
        spawn_local(run_block_collector_task(block_collector));
        (bytes_in_sender, block_out_receiver)
    }
}

#[cfg_attr(feature = "embassy_runtime", embassy_executor::task)]
pub async fn run_block_collector_task(mut block_collector: BlockCollector) {
    while let Some(slice) = block_collector.bytes_in_receiver.next().await {
        block_collector.receive_slice(&slice).await;
    }
}
