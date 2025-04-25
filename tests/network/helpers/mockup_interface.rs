use std::{
    future::Future,
    pin::Pin,
    sync::{mpsc, Arc, Mutex},
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
        socket_provider::SingleSocketProvider,
    },
};

pub struct MockupInterface {
    pub outgoing_queue: Vec<(ComInterfaceSocketUUID, Vec<u8>)>,

    info: ComInterfaceInfo,
    pub sender: Option<mpsc::Sender<Vec<u8>>>,
    pub receiver: Option<mpsc::Receiver<Vec<u8>>>,
}

impl SingleSocketProvider for MockupInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets().clone()
    }
}

impl MockupInterface {
    pub fn last_block(&self) -> Option<Vec<u8>> {
        self.outgoing_queue.last().map(|(_, block)| block.clone())
    }
    pub fn last_socket_uuid(&self) -> Option<ComInterfaceSocketUUID> {
        self.outgoing_queue
            .last()
            .map(|(socket_uuid, _)| socket_uuid.clone())
    }

    pub fn find_outgoing_block_for_socket(
        &self,
        socket_uuid: ComInterfaceSocketUUID,
    ) -> Option<Vec<u8>> {
        self.outgoing_queue
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
        self.outgoing_queue.last().cloned()
    }

    pub fn update(&mut self) {
        if let Some(receiver) = &self.receiver {
            let socket = self.get_socket().unwrap();
            let socket = socket.lock().unwrap();
            let mut receive_queue = socket.receive_queue.lock().unwrap();
            while let Ok(block) = receiver.try_recv() {
                receive_queue.extend(block);
            }
        }
    }
}

impl Default for MockupInterface {
    fn default() -> Self {
        MockupInterface {
            outgoing_queue: Vec::new(),
            sender: None,
            receiver: None,
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
        self.outgoing_queue.push((socket_uuid, block.to_vec()));

        if let Some(sender) = &self.sender {
            if let Err(_) = sender.send(block.to_vec()) {
                return Pin::from(Box::new(async move { false }));
            }
        }
        Pin::from(Box::new(async move { true }))
    }

    fn close<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        self.outgoing_queue.clear();
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
