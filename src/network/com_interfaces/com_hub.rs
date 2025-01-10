use std::collections::HashMap;

use crate::datex_values::Endpoint;

use super::{com_interface::ComInterface, com_interface_socket::ComInterfaceSocket};

struct DynamicEndpointProperties {
    known_since: u64,
    distance: u32,
}

pub struct ComHub {
    pub interfaces: Vec<Box<dyn ComInterface>>,
    pub endpoint_sockets: HashMap<Endpoint, HashMap<ComInterfaceSocket, DynamicEndpointProperties>>,
}

impl ComHub {
    fn add_interface(&mut self, interface: Box<dyn ComInterface>) -> bool {
        self.interfaces.push(interface);

        return true;
    }

    fn remove_interface(&mut self, interface: Box<dyn ComInterface>) {
        self.interfaces
            .retain(|x| std::ptr::eq(x.as_ref(), interface.as_ref()));
    }

    /*/
    fn iterate_endpoint_sockets(&self) -> Vec<ComInterfaceSocket> {

    }
    */
}
