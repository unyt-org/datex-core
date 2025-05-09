use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::rc::Rc;
use futures::channel::oneshot;
use log::info;
use ringmap::RingMap;
use crate::global::dxb_block::{BlockId, DXBBlock, IncomingBlockNumber, IncomingSectionIndex, IncomingEndpointScopeId, IncomingScopeId, OutgoingSectionIndex, OutgoingScopeId, IncomingSection};
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;
use crate::runtime::global_context::{get_global_context, get_global_time};
use crate::utils::time::TimeTrait;

// TODO: store scope memory
pub struct ScopeContext {
    pub next_section_index: IncomingSectionIndex,
    pub next_block_number: IncomingBlockNumber,
    /// timestamp of the last keep alive block
    /// when a specific time has passed since the timestamp, the scope context is disposed
    /// TODO: implement dispose of scope context
    pub keep_alive_timestamp: u64,
    // a reference to the block queue for the current section
    pub current_block_queue: Option<Rc<RefCell<VecDeque<DXBBlock>>>>,
    // a cache for all blocks indexed by their block number
    pub cached_blocks: BTreeMap<IncomingBlockNumber, DXBBlock>
}

/// A scope context storing scopes of incoming DXB blocks
impl ScopeContext {
    pub fn new() -> ScopeContext {
        ScopeContext {
            next_section_index: 0,
            next_block_number: 0,
            keep_alive_timestamp: get_global_time().now(),
            current_block_queue: None,
            cached_blocks: BTreeMap::new()
        }
    }
}

// fn that gets a scope context as callback
type ScopeObserver = Box<dyn FnMut(IncomingSection)>;

#[derive(Clone, Debug)]
pub struct BlockHistoryData {
    pub original_socket_uuid: ComInterfaceSocketUUID,
}

pub struct BlockHandler {
    pub current_scope_id: RefCell<OutgoingScopeId>,

    /// a map of active request scopes for incoming blocks
    pub block_cache: RefCell<HashMap<IncomingEndpointScopeId, ScopeContext>>,

    /// a queue of incoming request scopes
    /// the scopes can be retrieved from the request_scopes map
    pub request_queue: RefCell<VecDeque<(IncomingEndpointScopeId, IncomingSection)>>,

    /// a map of observers for incoming response blocks (by scope_id + block_index)
    /// contains an observer callback and an optional queue of blocks if the response block is a multi-block stream
    pub scope_observers: RefCell<HashMap<(IncomingScopeId, IncomingSectionIndex), (ScopeObserver, Option<Rc<RefCell<VecDeque<DXBBlock>>>>)>>,

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
            block_cache: RefCell::new(HashMap::new()),
            request_queue: RefCell::new(VecDeque::new()),
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
        let section_index = block.block_header.section_index;
        // TODO: correct sorting of incoming blocks
        let block_number = block.block_header.block_number;
        let is_response = block.block_header.flags_and_timestamp.block_type().is_response();

        info!("Received block (scope={scope_id}, section={section_index}, block_nr={block_number})");

        // handle observers if response block
        if is_response {
            self.handle_incoming_response_block(
                block,
            );
        }

        else {
            self.handle_incoming_request_block(
                block,
            );
        }
    }

    // Handles incoming request blocks by putting them into the request queue
    fn handle_incoming_request_block(
        &self,
        block: DXBBlock,
    ) {
        let endpoint_scope_id = IncomingEndpointScopeId {
            sender: block.routing_header.sender.clone(),
            scope_id: block.block_header.scope_id,
        };
        let new_sections = self.extract_complete_sections_with_new_incoming_block(
            block
        );
        // put into request queue
        let mut request_queue = self.request_queue.borrow_mut();
        for section in new_sections {
            request_queue.push_back((endpoint_scope_id.clone(), section));
        }
    }

    /// Handles incoming response blocks by calling the observer if an observer is registered
    /// Returns true when the observer has consumed all blocks and should be removed
    fn handle_incoming_response_block(
        &self,
        block: DXBBlock,
    )  {
        let endpoint_scope_id = IncomingEndpointScopeId {
            sender: block.routing_header.sender.clone(),
            scope_id: block.block_header.scope_id,
        };
        let new_sections = self.extract_complete_sections_with_new_incoming_block(
            block
        );
        // try to call the observer for the incoming response block
        for section in new_sections {
            let section_index = section.get_section_index();

            let remove_observer = if let Some(observer) = self.scope_observers.borrow_mut().get_mut(&(endpoint_scope_id.clone(), section_index)) {
                let (ref mut observer, ref mut block_queue) = observer;
                // call the observer with the new section
                observer(section);
                // remove observer
                true
            }
            else {
                // no observer for this scope id + block index
                log::warn!("No observer for incoming response block (sid={scope_id}, block={section_index}), dropping block");
                false
            };

            if remove_observer {
                self.scope_observers.borrow_mut().remove(&(endpoint_scope_id.clone(), section_index));
            }
        }
    }

    /// Takes a new incoming block and returns a vector of all new available incoming sections
    /// for the block's scope
    fn extract_complete_sections_with_new_incoming_block(
        &self,
        block: DXBBlock
    ) -> Vec<IncomingSection> {
        let section_index = block.block_header.section_index;
        let block_number = block.block_header.block_number;
        let is_end_of_section = block.block_header.flags_and_timestamp.is_end_of_section();
        let endpoint_scope_id = IncomingEndpointScopeId {
            sender: block.routing_header.sender.clone(),
            scope_id: block.block_header.scope_id,
        };

        // get scope context if it already exists
        let mut request_scopes = self.block_cache.borrow_mut();
        let scope_context = request_scopes.get_mut(&endpoint_scope_id);

        // Case 1: shortcut if no scope context exists and the block is a single block
        if scope_context.is_none() && is_end_of_section {
            // TODO: with return value for request blocks:
            // self.request_queue.borrow_mut().push_back((endpoint_scope_id.clone(), IncomingBlocks::SingleBlock(block)));
            return vec![IncomingSection::SingleBlock(block)];
        }

        // make sure a scope context exists from here on
        let scope_context = scope_context.unwrap_or_else(|| {
            // create a new scope context if it doesn't exist
            let new_scope_context = ScopeContext::new();
            request_scopes.insert(endpoint_scope_id.clone(), new_scope_context);
            request_scopes.get_mut(&endpoint_scope_id).unwrap()
        });

        // Case 2: if the block is the next expected block in the current section, put it into the
        // section block queue and try to drain blocks from the cache
        if section_index == scope_context.next_section_index && block_number == scope_context.next_block_number {

            // list of IncomingSections that is returned at the end
            let mut new_blocks = vec![];

            // initial values for loop variables from input block
            let mut is_end_of_scope = block.block_header.flags_and_timestamp.is_end_of_scope();
            let mut is_end_of_section = is_end_of_section;
            let mut next_block = block;

            // loop over the input block and potential blocks from the cache until the next block cannot be found
            // or the end of the scope is reached
            loop {
                let is_first_block_of_section = scope_context.current_block_queue.is_none();
                let current_block_queue = scope_context.current_block_queue.get_or_insert_with(|| {
                    // create a new block queue for the current section
                    Rc::new(RefCell::new(VecDeque::new()))
                });

                // push block to current block queue
                current_block_queue.borrow_mut().push_back(next_block);

                // add a new incoming section if this is the first block of the section
                if is_first_block_of_section {
                    new_blocks.push(
                        IncomingSection::BlockStream(current_block_queue.clone())
                    );
                }

                // cleanup / prepare for next block =======================
                // increment next block number
                scope_context.next_block_number += 1;

                // if end of scope, remove the scope context
                if is_end_of_scope {
                    request_scopes.remove(&endpoint_scope_id);
                    break;
                }
                // cleanup if section is finished
                else if is_end_of_section {
                    // increment section index
                    scope_context.next_section_index += 1;
                    // remove block queue
                    scope_context.current_block_queue = None;
                }
                // ========================================================

                // check if next block is in cache for next iteration
                if let Some(next_cached_block) = scope_context.cached_blocks.remove(&scope_context.next_block_number) {
                    // check if block is end of section
                    is_end_of_section = next_cached_block.block_header.flags_and_timestamp.is_end_of_section();
                    // check if block is end of scope
                    is_end_of_scope = next_cached_block.block_header.flags_and_timestamp.is_end_of_scope();
                    // set next block
                    next_block = next_cached_block;
                }
                // no more blocks in cache, break
                else {
                    break;
                }
            }

            new_blocks
        }


        // Case 3: if the block is not the next expected block in the current section,
        // put it into the block cache
        else {
            // check if block is already in cache
            // TODO: this should not happen, we should make sure duplicate blocks are dropped before
            if scope_context.cached_blocks.contains_key(&block_number) {
                log::warn!("Block {block_number} already in cache, dropping block");
            }

            // add block to cache
            scope_context.cached_blocks.insert(block_number, block);

            vec![]
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
        block_index: OutgoingSectionIndex
    ) -> Option<IncomingSection> {
        let (tx, rx) = oneshot::channel();
        let mut tx = Some(tx);

        // create observer callback for scope id + block index
        let observer = move |blocks: IncomingSection| {
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


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(1, 1);
    }
}