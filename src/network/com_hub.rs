use std::{cell::RefCell, collections::HashMap, rc::Rc};
use std::collections::HashSet;

use super::com_interfaces::{
    com_interface::{ComInterface, ComInterfaceTrait},
    com_interface_socket::ComInterfaceSocket,
};
use crate::datex_values::Endpoint;

struct DynamicEndpointProperties {
    known_since: u64,
    distance: u32,
}

pub struct ComHub {
    pub interfaces: HashSet<ComInterfaceTrait>,
    pub endpoint_sockets: HashMap<Endpoint, HashMap<ComInterfaceSocket, DynamicEndpointProperties>>,
}

impl ComHub {
    pub fn new() -> Rc<RefCell<ComHub>> {
        return Rc::new(RefCell::new(ComHub {
            interfaces: HashSet::new(),
            endpoint_sockets: HashMap::new(),
        }));
    }

    pub fn add_interface(&mut self, interface: ComInterfaceTrait) -> bool {
        self.interfaces.insert(interface)
    }

    pub fn remove_interface(&mut self, interface: ComInterfaceTrait) -> bool {
        self.interfaces.remove(&interface)
    }

    pub(crate) fn receive_block(&mut self, block: &[u8], socket: &ComInterfaceSocket) {
        println!("Received block: {:?}", block);
    }

    pub fn receive_slice(&mut self, slice: &[u8], socket: &ComInterfaceSocket) {
        self.receive_block(slice, socket);
    }
    
    /*/
    fn iterate_endpoint_sockets(&self) -> Vec<ComInterfaceSocket> {

    }
    */

    pub fn receive_queue(&mut self) {
        
    }


}
