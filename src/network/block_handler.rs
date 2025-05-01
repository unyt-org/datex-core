use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::rc::Rc;
use futures::channel::oneshot;
use log::info;
use crate::global::dxb_block::DXBBlock;

pub type IncomingScopeId = u32;
pub type IncomingBlockIndex = u16;
pub type IncomingBlockIncrement = u16;
pub type OutgoingScopeId = u32;
pub type OutgoingBlockIndex = u16;
pub type OutgoingBlockIncrement = u16;

#[derive(Debug, Clone)]
pub enum ResponseBlocks {
    SingleBlock(DXBBlock),
    /// a stream of blocks
    /// the stream is finished when a block has the end_of_block flag set
    BlockStream(Rc<RefCell<VecDeque<DXBBlock>>>),
}

// TODO: store scope memory
pub struct ScopeContext {
    pub scope_id: IncomingScopeId,
    pub current_block_index: IncomingBlockIndex,
    pub current_block_increment: IncomingBlockIncrement,
    // one or multiple blocks for each block index
    pub blocks: BTreeMap<IncomingBlockIndex, ResponseBlocks>
}

/// A scope context storing scopes of incoming DXB blocks
impl ScopeContext {
    pub fn new(scope_id: IncomingScopeId) -> ScopeContext {
        ScopeContext {
            scope_id,
            current_block_index: 0,
            current_block_increment: 0,
            blocks: BTreeMap::new(),
        }
    }
}

// fn that gets a scope context as callback
type ScopeObserver = Box<dyn FnMut(ResponseBlocks) -> ()>;

pub struct BlockHandler {
    pub current_scope_id: OutgoingScopeId,

    /// a map of active request scopes for incoming blocks
    pub request_scopes: HashMap<IncomingScopeId, ScopeContext>,
    /// a map of active response scopes for incoming blocks
    /// TODO: what to do with responses that are not handled by an observer?
    pub response_scopes: HashMap<IncomingScopeId, ScopeContext>,

    /// a map of observers for incoming response blocks (by scope_id + block_index)
    pub scope_observers: HashMap<(IncomingScopeId, IncomingBlockIndex), ScopeObserver>,
}

impl BlockHandler {
    pub fn new() -> BlockHandler {
        BlockHandler {
            current_scope_id: 0,
            request_scopes: HashMap::new(),
            response_scopes: HashMap::new(),
            scope_observers: HashMap::new(),
        }
    }

    pub fn handle_incoming_block(&mut self, block: DXBBlock) {
        let scope_id = block.block_header.scope_id;
        let block_index = block.block_header.block_index;
        // TODO: correct sorting of incoming blocks
        let block_increment = block.block_header.block_increment;
        let is_end_of_block = block.block_header.flags_and_timestamp.is_end_of_block();
        let is_response = block.block_header.flags_and_timestamp.block_type().is_response();

        info!("Received block (sid={scope_id}, block={block_index}, inc={block_increment})");

        // either store block in request or response scopes
        let scopes = if is_response {
            &mut self.response_scopes
        } else {
            &mut self.request_scopes
        };

        // create scope context if it doesn't exist
        if !scopes.contains_key(&scope_id) {
            scopes.insert(scope_id, ScopeContext::new(scope_id));
        }

        let scope_context = scopes.get_mut(&scope_id).unwrap();

        // create a new block entry if it doesn't exist
        if !scope_context.blocks.contains_key(&block_index) {
            // single block
            if is_end_of_block {
                scope_context.blocks.insert(
                    block_index,
                    ResponseBlocks::SingleBlock(block),
                );
            } else {
                // block stream
                let mut blocks = VecDeque::new();
                blocks.push_back(block);
                scope_context.blocks.insert(
                    block_index,
                    ResponseBlocks::BlockStream(Rc::new(RefCell::new(blocks))),
                );
            }
        }

        // add block to the existing block entry
        else {
            let blocks = scope_context.blocks.get_mut(&block_index).unwrap();
            // must be a block stream
            if let ResponseBlocks::BlockStream(block_stream) = blocks {
                block_stream.borrow_mut().push_back(block);
            } else {
                log::error!("Block index {block_index} only has a single block, but received a block stream");
                // TODO:
            }
        }
        
        // handle observers if response block
        if is_response {
            if let Some(mut observer) = self.scope_observers.remove(&(scope_id, block_index)) {
                // TODO: optimize: don't add and remove block from context if directly moved into observer afterwards
                let blocks = scope_context.blocks.remove(&block_index).unwrap();
                observer(blocks);
            }
        }
    }

    pub fn get_new_scope_id(&mut self) -> OutgoingScopeId {
        self.current_scope_id += 1;
        self.current_scope_id
    }

    /// wait for incoming response block with a specific scope id and block index
    pub async fn wait_for_incoming_response_block(
        &mut self,
        scope_id: OutgoingScopeId,
        block_index: OutgoingBlockIndex
    ) -> Option<ResponseBlocks> {
        let (tx, rx) = oneshot::channel();
        let mut tx = Some(tx);

        // create observer callback for scope id + block index
        let observer = move |blocks: ResponseBlocks| {
            if let Some(tx) = tx.take() {
                tx.send(blocks).expect("Failed to send block queue from observer");
            }
        };

        // add new scope observer
        self.scope_observers.insert(
            (scope_id, block_index),
            Box::new(observer)
        );

        // Await the result from the callback
        let res = rx.await.ok();

        res
    }
}