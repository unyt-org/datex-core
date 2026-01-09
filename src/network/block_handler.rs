use crate::collections::HashMap;
use crate::global::dxb_block::{
    BlockId, DXBBlock, IncomingBlockNumber, IncomingContextId,
    IncomingEndpointContextId, IncomingEndpointContextSectionId,
    IncomingSection, IncomingSectionIndex, OutgoingContextId,
    OutgoingSectionIndex,
};
use crate::network::com_interfaces::com_interface::socket::ComInterfaceSocketUUID;
use crate::std_random::RandomState;
use crate::stdlib::boxed::Box;
use crate::stdlib::collections::{BTreeMap, VecDeque};
use crate::stdlib::rc::Rc;
use crate::stdlib::vec;
use crate::stdlib::vec::Vec;
use crate::task::{
    UnboundedReceiver, UnboundedSender, create_unbounded_channel,
};
use crate::utils::time::Time;
use core::cell::RefCell;
use core::fmt::Debug;
use core::prelude::rust_2024::*;
use log::info;
use ringmap::RingMap;

// TODO #170: store scope memory
#[derive(Debug)]
pub struct ScopeContext {
    pub next_section_index: IncomingSectionIndex,
    pub next_block_number: IncomingBlockNumber,
    /// timestamp of the last keep alive block
    /// when a specific time has passed since the timestamp, the scope context is disposed
    /// TODO #171: implement dispose of scope context
    pub keep_alive_timestamp: u64,
    // a reference to the sender for the current section
    pub current_queue_sender: Option<UnboundedSender<DXBBlock>>,
    // a cache for all blocks indexed by their block number
    pub cached_blocks: BTreeMap<IncomingBlockNumber, DXBBlock>,
}

/// A scope context storing scopes of incoming DXB blocks
impl Default for ScopeContext {
    fn default() -> Self {
        ScopeContext {
            next_section_index: 0,
            next_block_number: 0,
            keep_alive_timestamp: Time::now(),
            current_queue_sender: None,
            cached_blocks: BTreeMap::new(),
        }
    }
}

// fn that gets a scope context as callback
type SectionObserver = Box<dyn FnMut(IncomingSection)>;

#[derive(Clone, Debug)]
pub struct BlockHistoryData {
    /// if block originated from local endpoint, the socket uuid is None,
    /// otherwise it is the uuid of the incoming socket
    pub original_socket_uuid: Option<ComInterfaceSocketUUID>,
}

pub struct IncomingSectionsChannel {
    incoming_sections_sender: RefCell<UnboundedSender<IncomingSection>>,
}

/// Utility struct that allows collecting incoming sections for tests instead of passing them to the runtime
/// through a channel
pub enum IncomingSectionsSink {
    Channel(IncomingSectionsChannel),
    Collector(Rc<RefCell<Vec<IncomingSection>>>),
}

pub enum IncomingSectionsSinkType {
    Channel,
    Collector,
}

impl IncomingSectionsSink {
    fn init(
        sink_type: IncomingSectionsSinkType,
    ) -> (Self, Option<UnboundedReceiver<IncomingSection>>) {
        match sink_type {
            IncomingSectionsSinkType::Channel => {
                let (sender, receiver) =
                    create_unbounded_channel::<IncomingSection>();
                (
                    IncomingSectionsSink::Channel(IncomingSectionsChannel {
                        incoming_sections_sender: RefCell::new(sender),
                    }),
                    Some(receiver),
                )
            }
            IncomingSectionsSinkType::Collector => (
                IncomingSectionsSink::Collector(Rc::new(RefCell::new(vec![]))),
                None,
            ),
        }
    }

    fn send(&self, section: IncomingSection) {
        match self {
            IncomingSectionsSink::Channel(channel) => {
                channel
                    .incoming_sections_sender
                    .borrow_mut()
                    .start_send(section)
                    .expect("Failed to send incoming section to channel");
            }
            IncomingSectionsSink::Collector(collector) => {
                collector.borrow_mut().push(section);
            }
        }
    }
}

pub struct BlockHandler {
    pub current_context_id: RefCell<OutgoingContextId>,

    /// a map of active request scopes for incoming blocks
    pub block_cache: RefCell<HashMap<IncomingEndpointContextId, ScopeContext>>,

    /// a queue of incoming request scopes
    /// the scopes can be retrieved from the request_scopes map
    pub incoming_sections_sink: IncomingSectionsSink,

    /// a map of observers for incoming response blocks (by context_id + block_index)
    /// contains an observer callback and an optional queue of blocks if the response block is a multi-block stream
    pub section_observers: RefCell<
        HashMap<(IncomingContextId, IncomingSectionIndex), SectionObserver>,
    >,

    /// history of all incoming blocks
    pub incoming_blocks_history:
        RefCell<RingMap<BlockId, BlockHistoryData, RandomState>>,
}

impl Debug for BlockHandler {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BlockHandler")
            .field("current_context_id", &self.current_context_id)
            .field("block_cache", &self.block_cache)
            .field("incoming_blocks_history", &self.incoming_blocks_history)
            .finish()
    }
}

const RING_MAP_CAPACITY: usize = 500;

impl BlockHandler {
    pub fn init(
        sink_type: IncomingSectionsSinkType,
    ) -> (BlockHandler, Option<UnboundedReceiver<IncomingSection>>) {
        let (incoming_sections_sink, receiver) =
            IncomingSectionsSink::init(sink_type);
        (
            BlockHandler {
                current_context_id: RefCell::new(0),
                block_cache: RefCell::new(HashMap::new()),
                incoming_sections_sink,
                section_observers: RefCell::new(HashMap::new()),
                incoming_blocks_history: RefCell::new(
                    RingMap::with_capacity_and_hasher(
                        RING_MAP_CAPACITY,
                        RandomState::default(),
                    ),
                ),
            },
            receiver,
        )
    }

    pub fn drain_collected_sections(&self) -> Vec<IncomingSection> {
        match &self.incoming_sections_sink {
            IncomingSectionsSink::Collector(collector) => {
                collector.borrow_mut().drain(..).collect()
            }
            _ => panic!("Not a collector sink"),
        }
    }

    /// Adds a block to the history of incoming blocks
    /// if the block is not already in the history
    /// returns true if the block was added and not already in the history
    pub fn add_block_to_history(
        &self,
        block: &DXBBlock,
        original_socket_uuid: Option<ComInterfaceSocketUUID>,
    ) {
        let mut history = self.incoming_blocks_history.borrow_mut();
        let block_id = block.get_block_id();
        // only add if original block
        if !history.contains_key(&block_id) {
            let block_data = BlockHistoryData {
                original_socket_uuid,
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
        block: &DXBBlock,
    ) -> Option<BlockHistoryData> {
        let history = self.incoming_blocks_history.borrow();
        let block_id = block.get_block_id();
        history.get(&block_id).cloned()
    }

    pub fn handle_incoming_block(&self, block: DXBBlock) {
        info!("Handling incoming block...");
        let context_id = block.block_header.context_id;
        let section_index = block.block_header.section_index;
        let block_number = block.block_header.block_number;
        let is_response = block
            .block_header
            .flags_and_timestamp
            .block_type()
            .is_response();

        info!(
            "Received block (context={context_id}, section={section_index}, block_nr={block_number})"
        );

        // handle observers if response block
        if is_response {
            self.handle_incoming_response_block(block);
        } else {
            self.handle_incoming_request_block(block);
        }
    }

    // Handles incoming request blocks by putting them into the request queue
    fn handle_incoming_request_block(&self, block: DXBBlock) {
        let new_sections =
            self.extract_complete_sections_with_new_incoming_block(block);
        // put into request queue
        let mut sender = &self.incoming_sections_sink;
        for section in new_sections {
            sender.send(section);
        }
    }

    /// Handles incoming response blocks by calling the observer if an observer is registered
    /// Returns true when the observer has consumed all blocks and should be removed
    fn handle_incoming_response_block(&self, block: DXBBlock) {
        let context_id = block.block_header.context_id;
        let endpoint_context_id = IncomingEndpointContextId {
            sender: block.routing_header.sender.clone(),
            context_id,
        };
        let new_sections =
            self.extract_complete_sections_with_new_incoming_block(block);
        // try to call the observer for the incoming response block
        for section in new_sections {
            let section_index = section.get_section_index();

            if let Some(observer) = self
                .section_observers
                .borrow_mut()
                .get_mut(&(context_id, section_index))
            {
                // call the observer with the new section
                observer(section);
            } else {
                // no observer for this scope id + block index
                log::warn!(
                    "No observer for incoming response block (scope={endpoint_context_id:?}, block={section_index}), dropping block"
                );
            };
        }
    }

    /// Takes a new incoming block and returns a vector of all new available incoming sections
    /// for the block's scope
    fn extract_complete_sections_with_new_incoming_block(
        &self,
        block: DXBBlock,
    ) -> Vec<IncomingSection> {
        let section_index = block.block_header.section_index;
        let block_number = block.block_header.block_number;
        let is_end_of_section =
            block.block_header.flags_and_timestamp.is_end_of_section();
        let is_end_of_context =
            block.block_header.flags_and_timestamp.is_end_of_context();
        let endpoint_context_id = IncomingEndpointContextId {
            sender: block.routing_header.sender.clone(),
            context_id: block.block_header.context_id,
        };
        let section_context_id = IncomingEndpointContextSectionId::new(
            endpoint_context_id.clone(),
            section_index,
        );

        // get scope context if it already exists
        let has_scope_context =
            self.block_cache.borrow().contains_key(&endpoint_context_id);

        // Case 1: shortcut if no scope context exists and the block is a single block
        if !has_scope_context
            && block_number == 0
            && (is_end_of_section || is_end_of_context)
        {
            return vec![IncomingSection::SingleBlock((
                Some(block),
                section_context_id.clone(),
            ))];
        }

        // make sure a scope context exists from here on
        let mut request_scopes = self.block_cache.borrow_mut();
        let scope_context = request_scopes
            .entry(endpoint_context_id.clone())
            .or_default();

        // TODO #172: what happens if the endpoint has not received all blocks starting with block_number 0?
        // we should still potentially process those blocks

        // Case 2: if the block is the next expected block in the current section, put it into the
        // section block queue and try to drain blocks from the cache
        if block_number == scope_context.next_block_number {
            // list of IncomingSections that is returned at the end
            let mut new_blocks = vec![];

            // initial values for loop variables from input block
            let mut is_end_of_context = is_end_of_context;
            let mut is_end_of_section = is_end_of_section;
            let mut next_block = block;
            let mut section_index = section_index;

            // loop over the input block and potential blocks from the cache until the next block cannot be found
            // or the end of the scope is reached
            loop {
                if let Some(sender) = &mut scope_context.current_queue_sender {
                    // send the next block to the section queue receiver
                    sender.start_send(next_block).expect(
                        "Failed to send block to current section queue",
                    );
                } else {
                    // create a new block queue for the current section
                    let (mut sender, receiver) = create_unbounded_channel();

                    // add the first block to the queue
                    new_blocks.push(IncomingSection::BlockStream((
                        Some(receiver),
                        IncomingEndpointContextSectionId::new(
                            endpoint_context_id.clone(),
                            section_index,
                        ),
                    )));

                    // send the next block to the section queue receiver
                    sender.start_send(next_block).expect(
                        "Failed to send first block to current section queue",
                    );

                    scope_context.current_queue_sender = Some(sender);
                }

                // cleanup / prepare for next block =======================
                // increment next block number
                scope_context.next_block_number += 1;

                // if end of scope, remove the scope context
                if is_end_of_context {
                    request_scopes.remove(&endpoint_context_id);
                    break;
                }
                // cleanup if section is finished
                else if is_end_of_section {
                    // increment section index
                    scope_context.next_section_index += 1;
                    // close and remove the current section queue sender
                    if let Some(sender) =
                        scope_context.current_queue_sender.take()
                    {
                        sender.close_channel();
                    }
                }
                // ========================================================

                // check if next block is in cache for next iteration
                if let Some(next_cached_block) = scope_context
                    .cached_blocks
                    .remove(&scope_context.next_block_number)
                {
                    // check if block is end of section
                    is_end_of_section = next_cached_block
                        .block_header
                        .flags_and_timestamp
                        .is_end_of_section();
                    // check if block is end of scope
                    is_end_of_context = next_cached_block
                        .block_header
                        .flags_and_timestamp
                        .is_end_of_context();
                    // set next block
                    next_block = next_cached_block;

                    // update section index from next block
                    section_index = next_block.block_header.section_index;
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
            // TODO #173: this should not happen, we should make sure duplicate blocks are dropped before
            if scope_context.cached_blocks.contains_key(&block_number) {
                log::warn!(
                    "Block {block_number} already in cache, dropping block"
                );
            }

            // add block to cache
            scope_context.cached_blocks.insert(block_number, block);

            vec![]
        }
    }

    pub fn get_new_context_id(&self) -> OutgoingContextId {
        *self.current_context_id.borrow_mut() += 1;
        *self.current_context_id.borrow()
    }

    /// Adds a new observer for incoming blocks with a specific scope id and block index
    /// Returns a receiver that can be awaited to get the incoming sections
    pub fn register_incoming_block_observer(
        &self,
        context_id: OutgoingContextId,
        section_index: OutgoingSectionIndex,
    ) -> UnboundedReceiver<IncomingSection> {
        let (tx, rx) = create_unbounded_channel::<IncomingSection>();
        let tx = Rc::new(RefCell::new(tx));

        // create observer callback for scope id + block index
        let observer = move |blocks: IncomingSection| {
            tx.clone().borrow_mut().start_send(blocks).unwrap();
        };

        // add new scope observer
        self.section_observers
            .borrow_mut()
            .insert((context_id, section_index), Box::new(observer));

        rx
    }

    /// Waits for incoming response block with a specific scope id and block index
    pub async fn wait_for_incoming_response_block(
        &self,
        context_id: OutgoingContextId,
        section_index: OutgoingSectionIndex,
    ) -> Option<IncomingSection> {
        let rx =
            self.register_incoming_block_observer(context_id, section_index);
        // Await the result from the callback
        // FIXME #174
        None
        // rx.next().await
    }
}
