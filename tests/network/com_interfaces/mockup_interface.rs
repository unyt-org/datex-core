use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use datex_core::{
    delegate_com_interface_info,
    network::com_interfaces::{
        com_interface::{
            ComInterface, ComInterfaceInfo, ComInterfaceSockets,
            ComInterfaceState, ComInterfaceUUID,
        },
        com_interface_properties::InterfaceProperties,
        com_interface_socket::ComInterfaceSocketUUID,
    },
};

pub struct MockupInterface {
    pub block_queue: Vec<(ComInterfaceSocketUUID, Vec<u8>)>,
    info: ComInterfaceInfo,
}

impl MockupInterface {
    pub fn last_block(&self) -> Option<Vec<u8>> {
        self.block_queue.last().map(|(_, block)| block.clone())
    }
    pub fn last_socket_uuid(&self) -> Option<ComInterfaceSocketUUID> {
        self.block_queue
            .last()
            .map(|(socket_uuid, _)| socket_uuid.clone())
    }

    pub fn find_outgoing_block_for_socket(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Option<Vec<u8>> {
        self.block_queue
            .iter()
            .find(|(uuid, _)| uuid == &socket_uuid)
            .map(|(_, block)| block.clone())
    }
    pub fn has_outgoing_block_for_socket(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
    ) -> bool {
        self.find_outgoing_block_for_socket(socket_uuid).is_some()
    }

    pub fn last_block_and_socket(
        &self,
    ) -> Option<(ComInterfaceSocketUUID, Vec<u8>)> {
        self.block_queue.last().cloned()
    }
}

impl Default for MockupInterface {
    fn default() -> Self {
        MockupInterface {
            block_queue: Vec::new(),
            info: ComInterfaceInfo::new_with_state(
                ComInterfaceState::Connected,
            ),
        }
    }
}

impl ComInterface for MockupInterface {
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        // FIXME this should be inside the async body, why is it not working?
        self.block_queue.push((socket_uuid, block.to_vec()));

        Pin::from(Box::new(async move { true }))
    }

    fn close<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        self.block_queue.clear();
        Pin::from(Box::new(async move { true }))
    }

    fn init_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "mockup".to_string(),
            name: Some("mockup".to_string()),
            ..Default::default()
        }
    }

    delegate_com_interface_info!();
}
