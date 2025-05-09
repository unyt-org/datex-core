use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::rc::Rc;
use futures::channel::oneshot;
use log::info;
use ringmap::RingMap;
use crate::global::dxb_block::{BlockId, DXBBlock, IncomingBlockIncrement, IncomingBlockIndex, IncomingEndpointScopeId, IncomingScopeId, OutgoingBlockIndex, OutgoingScopeId, IncomingBlocks};
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;

// TODO: store scope memory
pub struct ScopeContext {
    pub endpoint_scope_id: IncomingEndpointScopeId,
    pub current_block_index: IncomingBlockIndex,
    pub current_block_increment: IncomingBlockIncrement,
    // one or multiple blocks for each block index
    pub blocks: BTreeMap<IncomingBlockIndex, IncomingBlocks>
}

/// A scope context storing scopes of incoming DXB blocks
impl ScopeContext {
    pub fn new(endpoint_scope_id: IncomingEndpointScopeId) -> ScopeContext {
        ScopeContext {
            endpoint_scope_id,
            current_block_index: 0,
            current_block_increment: 0,
            blocks: BTreeMap::new(),
        }
    }
}

// fn that gets a scope context as callback
type ScopeObserver = Box<dyn FnMut(IncomingBlocks)>;

#[derive(Clone, Debug)]
pub struct BlockHistoryData {
    pub original_socket_uuid: ComInterfaceSocketUUID,
}

pub struct BlockHandler {
    pub current_scope_id: RefCell<OutgoingScopeId>,

    /// a map of active request scopes for incoming blocks
    pub request_scopes: RefCell<HashMap<IncomingEndpointScopeId, ScopeContext>>,

    /// a map of observers for incoming response blocks (by scope_id + block_index)
    /// contains a observer callback and an optional queue of blocks if the response block is a multi-block stream
    pub scope_observers: RefCell<HashMap<(IncomingScopeId, IncomingBlockIndex), (ScopeObserver, Option<Rc<RefCell<VecDeque<DXBBlock>>>>)>>,

    /// history of all incoming blocks
    pub incoming_blocks_history: RefCell<RingMap<BlockId, BlockHistoryData>>
}

impl Default for BlockHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockHandler {
    pub fn new() -> BlockHandler {
        BlockHandler {
            current_scope_id: RefCell::new(0),
            request_scopes: RefCell::new(HashMap::new()),
            scope_observers: RefCell::new(HashMap::new()),
            incoming_blocks_history: RefCell::new(RingMap::with_capacity(500)),
        }
    }

    /// Adds a block to the history of incoming blocks
    /// if the block is not already in the history
    /// returns true if the block was added and not already in the history
    pub fn add_block_to_history(&self, block: &DXBBlock, socket_uuid: ComInterfaceSocketUUID) {
        let mut history = self.incoming_blocks_history.borrow_mut();
        let block_id = block.get_block_id();
        // only add if original block
        if !history.contains_key(&block_id) {
            let block_data = BlockHistoryData {
                original_socket_uuid: socket_uuid,
            };
            history.insert(block_id, block_data);
        }
    }

    /// Checks if a block is already in the history
    pub fn is_block_in_history(&self, block: &DXBBlock) -> bool {
        let history = self.incoming_blocks_history.borrow();
        let block_id = block.get_block_id();
        history.contains_key(&block_id)
    }

    pub fn get_block_data_from_history(
        &self,
        block: &DXBBlock
    ) -> Option<BlockHistoryData> {
        let history = self.incoming_blocks_history.borrow();
        let block_id = block.get_block_id();
        history.get(&block_id).cloned()
    }

    pub fn handle_incoming_block(&self, block: DXBBlock) {
        info!("Handling incoming block...");
        let scope_id = block.block_header.scope_id;
        let block_index = block.block_header.block_index;
        // TODO: correct sorting of incoming blocks
        let block_increment = block.block_header.block_increment;
        let is_end_of_block = block.block_header.flags_and_timestamp.is_end_of_block();
        let is_response = block.block_header.flags_and_timestamp.block_type().is_response();
        let endpoint_scope_id = IncomingEndpointScopeId {
            sender: block.routing_header.sender.clone(),
            scope_id,
        };

        info!("Received block (sid={scope_id}, block={block_index}, inc={block_increment})");

        // handle observers if response block
        // TODO: if expecting multiple responses, handle them, otherwise the observer can directly be removed
        if is_response {
            let remove_observer = match self.scope_observers.borrow_mut().get_mut(&(scope_id, block_index)) {
                Some((ref mut observer, ref mut block_queue)) => {
                    // is end of block and no previous block queue -> is single block
                    let is_single_block = is_end_of_block && block_queue.is_none();
                    match is_single_block {
                        // single block
                        true => {
                            observer(
                                IncomingBlocks::SingleBlock(block)
                            );
                        }
                        // block stream
                        false => {
                            // push block to existing block queue for observer
                            if let Some(block_queue) = block_queue {
                                block_queue.borrow_mut().push_back(block);
                            }
                            else {
                                // start of new block stream, create and send to observer
                                let mut blocks = VecDeque::new();
                                blocks.push_back(block);
                                let blocks = Rc::new(RefCell::new(blocks));

                                observer(
                                    IncomingBlocks::BlockStream(blocks.clone())
                                );
                            }
                        }
                    };

                    // cleanup observer if is_end_of_block
                    if is_end_of_block {
                        // remove observer
                        log::info!("Removing observer for incoming response block (sid={scope_id}, block={block_index})");
                        true
                    }
                    else {
                        false
                    }
                }
                None => {
                    // no observer for this scope id + block index
                    log::warn!("No observer for incoming response block (sid={scope_id}, block={block_index}), dropping block");
                    false
                }
            };
            
            if remove_observer {
                // remove observer
                self.scope_observers.borrow_mut().remove(&(scope_id, block_index));
            }
        }

        else {
            // either store block in request or response scopes
            let mut request_scopes = self.request_scopes.borrow_mut();

            // create scope context if it doesn't exist
            request_scopes.entry(endpoint_scope_id.clone()).or_insert_with(|| ScopeContext::new(endpoint_scope_id.clone()));

            let scope_context = request_scopes.get_mut(&endpoint_scope_id).unwrap();

            // create a new block entry if it doesn't exist
            if let std::collections::btree_map::Entry::Vacant(e) = scope_context.blocks.entry(block_index) {
                // single block
                if is_end_of_block {
                    e.insert(IncomingBlocks::SingleBlock(block));
                } else {
                    // block stream
                    let mut blocks = VecDeque::new();
                    blocks.push_back(block);
                    e.insert(IncomingBlocks::BlockStream(Rc::new(RefCell::new(blocks))));
                }
            } else {
                let blocks = scope_context.blocks.get_mut(&block_index).unwrap();
                // must be a block stream
                if let IncomingBlocks::BlockStream(block_stream) = blocks {
                    block_stream.borrow_mut().push_back(block);
                } else {
                    log::error!("Block index {block_index} only has a single block, but received a block stream");
                    // TODO:
                }
            }
        }

    }

    pub fn get_new_scope_id(&self) -> OutgoingScopeId {
        *self.current_scope_id.borrow_mut() += 1;
        *self.current_scope_id.borrow()
    }

    /// Waits for incoming response block with a specific scope id and block index
    pub async fn wait_for_incoming_response_block(
        &self,
        endpoint_scope_id: OutgoingScopeId,
        block_index: OutgoingBlockIndex
    ) -> Option<IncomingBlocks> {
        let (tx, rx) = oneshot::channel();
        let mut tx = Some(tx);

        // create observer callback for scope id + block index
        let observer = move |blocks: IncomingBlocks| {
            if let Some(tx) = tx.take() {
                tx.send(blocks).expect("Failed to send block queue from observer");
            }
        };

        // add new scope observer
        self.scope_observers.borrow_mut().insert(
            (endpoint_scope_id, block_index),
            (Box::new(observer), None)
        );

        // Await the result from the callback
        let res = rx.await.ok();

        res
    }
}