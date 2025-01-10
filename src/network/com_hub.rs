use std::{cell::RefCell, collections::HashMap, rc::Rc};

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
use std::collections::HashSet;

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

    pub(crate) fn receive_block(&mut self, block: &[u8]) {
        todo!()
    }
    /*/
    fn iterate_endpoint_sockets(&self) -> Vec<ComInterfaceSocket> {

    }
    */
}
