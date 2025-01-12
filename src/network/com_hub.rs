use std::collections::HashSet;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

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
    pub sockets: HashSet<ComInterfaceSocket>
}

impl ComHub {
    pub fn new() -> Rc<RefCell<ComHub>> {
        return Rc::new(RefCell::new(ComHub {
            interfaces: HashSet::new(),
            endpoint_sockets: HashMap::new(),
            sockets: HashSet::new(),
        }));
    }

    pub fn add_interface(&mut self, interface: ComInterfaceTrait) -> bool {
        self.interfaces.insert(interface)
    }

    pub fn remove_interface(&mut self, interface: ComInterfaceTrait) -> bool {
        self.interfaces.remove(&interface)
    }

    pub(crate) fn receive_block(&self, block: &DXBBlock, socket: &ComInterfaceSocket) {
        println!("Received block: {:?}", block);
    }

    /*/
    fn iterate_endpoint_sockets(&self) -> Vec<ComInterfaceSocket> {

    }
    */

    pub fn receive_blocks(&self) {
        // iterate over all sockets
        for socket in &self.sockets {
            let block_queue = socket.get_block_queue();
            for block in block_queue {
                self.receive_block(block, socket);
            }
        }
    }
}
