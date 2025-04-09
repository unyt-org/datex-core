use log::{debug, info};

use crate::datex_values::Endpoint;
use crate::network::com_hub::{ComHub, DynamicEndpointProperties};
use crate::network::com_interfaces::com_interface::ComInterfaceUUID;
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;
use std::collections::HashMap;
use std::fmt::Display;

pub struct ComHubMetadataInterfaceSocket {
    pub uuid: String,
    pub endpoint: Endpoint,
    pub direction: InterfaceDirection,
    pub properties: DynamicEndpointProperties,
}
pub struct ComHubMetadataInterface {
    pub uuid: String,
    pub properties: InterfaceProperties,
    pub sockets: Vec<ComHubMetadataInterfaceSocket>,
}

pub struct ComHubMetadata {
    pub interfaces: Vec<ComHubMetadataInterface>,
    pub endpoint_sockets: HashMap<
        Endpoint,
        Vec<(ComInterfaceSocketUUID, DynamicEndpointProperties)>,
    >,
}

impl Display for ComHubMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ComHubMetadata {{\n")?;
        for interface in &self.interfaces {
            write!(f, "  Interface: {}\n", interface.uuid)?;
            write!(f, "    Properties: {:?}\n", interface.properties)?;
            for socket in &interface.sockets {
                write!(
                    f,
                    "    Socket: {} ({}), Properties: {:?}\n",
                    socket.uuid, socket.endpoint, socket.properties
                )?;
            }
        }
        write!(f, "}}\n")
    }
}

impl ComHub {
    pub fn get_metadata(&self) -> ComHubMetadata {
        let mut metadata = ComHubMetadata {
            interfaces: Vec::new(),
            endpoint_sockets: HashMap::new(),
        };

        let mut sockets_by_com_interface_uuid: HashMap<
            ComInterfaceUUID,
            Vec<ComHubMetadataInterfaceSocket>,
        > = HashMap::new();

        for (endpoint, sockets) in &self.endpoint_sockets {
            for (socket_uuid, properties) in sockets {
                let socket = self.get_socket_by_uuid(socket_uuid);
                let socket = socket.borrow();
                let com_interface_uuid = socket.interface_uuid.clone();
                if !sockets_by_com_interface_uuid
                    .contains_key(&com_interface_uuid)
                {
                    sockets_by_com_interface_uuid
                        .insert(com_interface_uuid.clone(), Vec::new());
                }
                sockets_by_com_interface_uuid
                    .get_mut(&com_interface_uuid)
                    .unwrap()
                    .push(ComHubMetadataInterfaceSocket {
                        uuid: socket_uuid.0.to_string(),
                        endpoint: endpoint.clone(),
                        direction: socket.direction.clone(),
                        properties: properties.clone(),
                    });
            }
        }

        for interface in self.interfaces.values() {
            let interface = interface.borrow();

            metadata.interfaces.push(ComHubMetadataInterface {
                uuid: interface.get_uuid().0.to_string(),
                properties: interface.get_properties(),
                sockets: sockets_by_com_interface_uuid
                    .remove(&interface.get_uuid())
                    .unwrap_or(Vec::new()),
            });
        }

        metadata
    }

    pub fn print_metadata(&self) {
        let metadata = self.get_metadata();
        debug!("ComHub Metadata: {}", metadata);
    }
}
