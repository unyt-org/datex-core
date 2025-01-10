use std::{collections::HashMap, rc::Rc};

use crate::datex_values::Endpoint;
use super::com_interfaces::{com_interface::{ComInterface, ComInterfaceTrait}, com_interface_socket::ComInterfaceSocket};

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
    pub fn new() -> ComHub {
        return ComHub {
            interfaces: HashSet::new(),
            endpoint_sockets: HashMap::new(),
        };
    }

    pub fn add_interface(&mut self, interface: ComInterfaceTrait) -> bool {
        self.interfaces.insert(interface)
    }

    pub fn remove_interface(&mut self, interface: ComInterfaceTrait) -> bool {
        self.interfaces.remove(&interface)
    }

    /*/
    fn iterate_endpoint_sockets(&self) -> Vec<ComInterfaceSocket> {

    }
    */
}
