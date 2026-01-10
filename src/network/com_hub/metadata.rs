use core::prelude::rust_2024::*;
use log::info;

use crate::collections::HashMap;
use crate::network::com_hub::ComHub;
use crate::network::com_hub::managers::socket_manager::DynamicEndpointProperties;
use crate::network::com_interfaces::com_interface::ComInterfaceUUID;

use crate::network::com_interfaces::com_interface::properties::InterfaceDirection;
use crate::network::com_interfaces::com_interface::properties::InterfaceProperties;
use crate::network::com_interfaces::com_interface::socket::ComInterfaceSocketUUID;
use crate::stdlib::format;
use crate::stdlib::string::String;
use crate::stdlib::string::ToString;
use crate::stdlib::vec::Vec;
use crate::values::core_values::endpoint::Endpoint;
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
            let sorted_sockets = interface.sockets.iter().sorted_by_key(|s| {
                match &s.properties {
                    Some(properties) => properties.distance,
                    None => i8::MAX,
                }
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
    /// Generates metadata about the ComHub, its interfaces and sockets.
    pub fn metadata(&self) -> ComHubMetadata {
        let mut metadata = ComHubMetadata {
            endpoint: self.endpoint.clone(),
            interfaces: Vec::new(),
            endpoint_sockets: HashMap::new(),
        };

        let mut sockets_by_com_interface_uuid: HashMap<
            ComInterfaceUUID,
            Vec<ComHubMetadataInterfaceSocket>,
        > = HashMap::new();

        let socket_manager = self.socket_manager.borrow();
        for (endpoint, sockets) in socket_manager.endpoint_sockets.iter() {
            for (socket_uuid, properties) in sockets {
                let socket = socket_manager.get_socket_by_uuid(socket_uuid);
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

        for (socket_uuid, (socket, endpoints)) in socket_manager.sockets.iter()
        {
            // if no endpoints are registered, we consider it a socket without an endpoint
            if endpoints.is_empty() {
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
                        direction: socket.direction.clone(),
                        endpoint: None,
                        properties: None,
                    });
                continue;
            }
        }
        drop(socket_manager);
        let interface_manager = self.interface_manager.borrow();

        for (interface, _) in interface_manager.interfaces.values() {
            metadata.interfaces.push(ComHubMetadataInterface {
                uuid: interface.uuid().0.to_string(),
                properties: interface.properties().as_ref().clone(),
                sockets: sockets_by_com_interface_uuid
                    .remove(&interface.uuid())
                    .unwrap_or_default(),
            });
        }

        metadata
    }

    /// Prints the ComHub metadata to the log.
    pub fn print_metadata(&self) {
        let metadata = self.metadata();
        info!("ComHub Metadata:\n{metadata}");
    }
}
