use std::collections::{HashSet, VecDeque};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use anyhow::Result;
use wasm_bindgen::prelude::wasm_bindgen;

use super::com_interfaces::{
    com_interface::{ComInterface, ComInterfaceTrait},
    com_interface_socket::ComInterfaceSocket,
};
use crate::datex_values::Endpoint;
use crate::global::dxb_block::DXBBlock;

struct DynamicEndpointProperties {
    known_since: u64,
    distance: u32,
}

pub struct ComHub {
    pub interfaces: HashSet<ComInterfaceTrait>,
    pub endpoint_sockets: HashMap<Endpoint, HashMap<ComInterfaceSocket, DynamicEndpointProperties>>,
    //pub sockets: HashSet<RefCell<ComInterfaceSocket>>,
    pub incoming_blocks: RefCell<VecDeque<Rc<DXBBlock>>>,
}

impl ComHub {
    pub fn new() -> Rc<RefCell<ComHub>> {
        return Rc::new(RefCell::new(ComHub {
            interfaces: HashSet::new(),
            endpoint_sockets: HashMap::new(),
            // sockets: HashSet::new(),
            incoming_blocks: RefCell::new(VecDeque::new()),
        }));
    }

    pub fn add_interface(&mut self, mut interface: ComInterfaceTrait) -> Result<()> {
        if self.interfaces.contains(&interface) {
            return Err(anyhow::anyhow!("Interface already exists"));
        }

        interface.connect()?;
        self.interfaces.insert(interface);

        Ok(())
    }

    pub fn remove_interface(&mut self, interface: ComInterfaceTrait) -> bool {
        self.interfaces.remove(&interface)
    }

    pub(crate) fn receive_block(&self, block: &DXBBlock, socket: &RefCell<ComInterfaceSocket>) {
        println!("Received block: {:?}", block);

        // TODO: routing

        // own incoming blocks
        let mut incoming_blocks = self.incoming_blocks.borrow_mut();
        incoming_blocks.push_back(Rc::new(block.clone()));
    }

    // iterate over all sockets of all interfaces
    fn iterate_all_sockets(&self) -> Vec<Rc<RefCell<ComInterfaceSocket>>> {
        let mut sockets = Vec::new();
        for interface in &self.interfaces {
            let interface_ref = interface;
            for socket in interface_ref.get_sockets().borrow().iter() {
                sockets.push(socket.clone());
            }
        }
        sockets.clone()
    }

    fn iterate_endpoint_sockets(&self) -> Vec<ComInterfaceSocket> {
        todo!()
    }

    /**
     * Update all sockets and interfaces,
     * collecting incoming data and sending out queued blocks.
     */
    pub fn update(&mut self) {
        // update sockets
        self.update_sockets();

        // receive blocks from all sockets
        self.receive_incoming_blocks();

        // send all queued blocks from all interfaces
        self.flush_outgoing_blocks();
    }

    /**
     * Send a block to all endpoints specified in block header.
     * The routing algorithm decides which sockets are used to send the block, based on the endpoint.
     * A block can be sent to multiple endpoints at the same time over a socket or to multiple sockets for each endpoint.
     * The original_socket parameter is used to prevent sending the block back to the sender.
     * When this method is called, the block is queued in the send queue.
     */
    pub fn send_block(&self, block: &DXBBlock, original_socket: Option<&mut ComInterfaceSocket>) {
        // TODO: routing
        for socket in &self.iterate_all_sockets() {
            let mut socket_ref = socket.borrow_mut();

            match &block.to_bytes() {
                Ok(bytes) => {
                    socket_ref.queue_outgoing_block(bytes);
                }
                Err(err) => {
                    eprintln!("Failed to convert block to bytes: {:?}", err);
                }
            }
        }
    }

    fn update_sockets(&self) {
        // update sockets, collect incoming data into full blocks
        for socket in &self.iterate_all_sockets() {
            let mut socket_ref = socket.borrow_mut();
            socket_ref.collect_incoming_data();
        }
    }

    /**
     * Collect all blocks from the receive queues of all sockets and process them
     * in the receive_block method.
     */
    fn receive_incoming_blocks(&mut self) {
        // iterate over all sockets
        for socket in &self.iterate_all_sockets() {
            let socket_ref = socket.borrow();
            let block_queue = socket_ref.get_incoming_block_queue();
            for block in block_queue {
                self.receive_block(block, socket);
            }
        }
    }

    /**
     * Send all queued blocks from all interfaces.
     */
    fn flush_outgoing_blocks(&mut self) {
        for interface in &self.interfaces {
            interface.flush_outgoing_blocks();
        }
    }
}