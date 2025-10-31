use log::info;

use crate::values::core_values::endpoint::Endpoint;
use crate::network::com_hub::{ComHub, DynamicEndpointProperties};
use crate::network::com_interfaces::com_interface::ComInterfaceUUID;
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;
use std::collections::HashMap;
use core::fmt::Display;
use itertools::Itertools;

pub struct ComHubMetadataInterfaceSocket {
    pub uuid: String,
    pub direction: InterfaceDirection,
    pub endpoint: Option<Endpoint>,
    pub properties: Option<DynamicEndpointProperties>,
}
pub struct ComHubMetadataInterfaceSocketWithoutEndpoint {
    pub uuid: String,
    pub direction: InterfaceDirection,
}
pub struct ComHubMetadataInterface {
    pub uuid: String,
    pub properties: InterfaceProperties,
    pub sockets: Vec<ComHubMetadataInterfaceSocket>,
}

pub struct ComHubMetadata {
    pub endpoint: Endpoint,
    pub interfaces: Vec<ComHubMetadataInterface>,
    pub endpoint_sockets: HashMap<
        Endpoint,
        Vec<(ComInterfaceSocketUUID, DynamicEndpointProperties)>,
    >,
}

impl Display for ComHubMetadata {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        writeln!(f, "ComHub ({})", self.endpoint)?;

        // print interfaces
        for interface in &self.interfaces {
            writeln!(
                f,
                "  {}/{}{}:",
                interface.properties.interface_type,
                interface.properties.channel,
                interface
                    .properties
                    .name
                    .clone()
                    .map(|n| format!(" ({n})"))
                    .unwrap_or("".to_string()),
            )?;

            // print sockets
            let sorted_sockets = interface
                .sockets
                .iter()
                .sorted_by_key(|s| match &s.properties {
                    Some(properties) => properties.distance,
                    None => i8::MAX,
                });
            
            for socket in sorted_sockets {
                writeln!(
                    f,
                    "   {} {}{} (distance: {}, uuid: {})",
                    match socket.direction {
                        InterfaceDirection::In => "──▶".to_string(),
                        InterfaceDirection::Out => "◀──".to_string(),
                        InterfaceDirection::InOut => "◀──▶".to_string(),
                    },
                    match &socket.properties {
                        Some(properties) => match properties.is_direct {
                            true => "".to_string(),
                            false => "[INDIRECT] ".to_string(),
                        },
                        None => "".to_string(),
                    },
                    match &socket.endpoint {
                        Some(endpoint) => endpoint.to_string(),
                        None => "unknown".to_string(),
                    },
                    match &socket.properties {
                        Some(properties) => properties.distance.to_string(),
                        None => "unknown".to_string(),
                    },
                    socket.uuid
                )?;
            }
        }

        Ok(())
    }
}

impl ComHub {
    pub fn get_metadata(&self) -> ComHubMetadata {
        let mut metadata = ComHubMetadata {
            endpoint: self.endpoint.clone(),
            interfaces: Vec::new(),
            endpoint_sockets: HashMap::new(),
        };

        let mut sockets_by_com_interface_uuid: HashMap<
            ComInterfaceUUID,
            Vec<ComHubMetadataInterfaceSocket>,
        > = HashMap::new();

        for (endpoint, sockets) in self.endpoint_sockets.borrow().iter() {
            for (socket_uuid, properties) in sockets {
                let socket = self.get_socket_by_uuid(socket_uuid);
                let socket = socket.lock().unwrap();
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
                        endpoint: Some(endpoint.clone()),
                        direction: socket.direction.clone(),
                        properties: Some(properties.clone()),
                    });
            }
        }

        for (socket_uuid, (socket, endpoints)) in self.sockets.borrow().iter() {
            // if no endpoints are registered, we consider it a socket without an endpoint
            if endpoints.is_empty() {
                let socket = socket.lock().unwrap();
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
                    .push(
                    ComHubMetadataInterfaceSocket {
                        uuid: socket_uuid.0.to_string(),
                        direction: socket.direction.clone(),
                        endpoint: None,
                        properties: None,
                    },
                );
                continue;
            }
        }

        for (interface, _) in self.interfaces.borrow().values() {
            let interface = interface.borrow();

            metadata.interfaces.push(ComHubMetadataInterface {
                uuid: interface.get_uuid().0.to_string(),
                properties: interface.init_properties(),
                sockets: sockets_by_com_interface_uuid
                    .remove(interface.get_uuid())
                    .unwrap_or_default(),
            });
        }

        metadata
    }

    pub fn print_metadata(&self) {
        let metadata = self.get_metadata();
        info!("ComHub Metadata:\n{metadata}");
    }
}
