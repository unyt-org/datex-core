use crate::stdlib::collections::VecDeque;
use crate::stdlib::{cell::RefCell, rc::Rc};
use anyhow::Result;
use log::info;
use std::collections::HashMap; // FIXME no-std

use super::com_interfaces::{
    com_interface::ComInterface, com_interface_socket::ComInterfaceSocket,
};
use crate::datex_values::Endpoint;
use crate::global::dxb_block::DXBBlock;
use crate::network::com_interfaces::com_interface::ComInterfaceUUID;
use crate::network::com_interfaces::com_interface_properties::InterfaceProperties;
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;
use crate::runtime::Context;
struct DynamicEndpointProperties {
    pub known_since: u64,
    pub distance: u32,
}

pub struct ComHub {
    pub interfaces: HashMap<ComInterfaceUUID, Rc<RefCell<dyn ComInterface>>>,
    pub endpoint_sockets: HashMap<
        Endpoint,
        HashMap<ComInterfaceSocket, DynamicEndpointProperties>,
    >,
    //pub sockets: HashSet<RefCell<ComInterfaceSocket>>,
    pub incoming_blocks: Rc<RefCell<VecDeque<Rc<DXBBlock>>>>,
    pub context: Rc<RefCell<Context>>,
}

#[derive(Debug, Clone)]
struct EndpointIterateOptions {
    pub only_direct: bool,
    pub only_outgoing: bool,
    pub exact_instance: bool,
    pub exclude_socket: Option<ComInterfaceSocketUUID>,
}

impl Default for EndpointIterateOptions {
    fn default() -> Self {
        EndpointIterateOptions {
            only_direct: false,
            only_outgoing: false,
            exact_instance: false,
            exclude_socket: None,
        }
    }
}

impl Default for ComHub {
    fn default() -> Self {
        ComHub {
            interfaces: HashMap::new(),
            endpoint_sockets: HashMap::new(),
            context: Rc::new(RefCell::new(Context::default())),
            incoming_blocks: Rc::new(RefCell::new(VecDeque::new())),
        }
    }
}

impl ComHub {
    pub fn new(context: Rc<RefCell<Context>>) -> Rc<RefCell<ComHub>> {
        Rc::new(RefCell::new(ComHub {
            context,
            ..ComHub::default()
        }))
    }

    pub fn add_interface(
        &mut self,
        interface: Rc<RefCell<dyn ComInterface>>,
    ) -> Result<()> {
        let uuid = interface.borrow().get_uuid();
        if self.interfaces.contains_key(&uuid) {
            return Err(anyhow::anyhow!("Interface already exists"));
        }

        interface.borrow_mut().connect()?;
        self.interfaces.insert(uuid, interface);

        Ok(())
    }

    pub fn remove_interface(
        &mut self,
        interface: Rc<RefCell<dyn ComInterface>>,
    ) -> bool {
        self.interfaces
            .remove(&interface.borrow().get_uuid())
            .is_some()
    }

    pub(crate) fn receive_block(
        &self,
        block: &DXBBlock,
        socket: &RefCell<ComInterfaceSocket>,
    ) {
        println!("Received block: {:?}", block);

        // TODO: routing

        // own incoming blocks
        let mut incoming_blocks = self.incoming_blocks.borrow_mut();
        incoming_blocks.push_back(Rc::new(block.clone()));
    }

    // iterate over all sockets of all interfaces
    fn iterate_all_sockets(&self) -> Vec<Rc<RefCell<ComInterfaceSocket>>> {
        let mut sockets = Vec::new();
        for (_, interface) in &self.interfaces {
            let interface_ref = interface.borrow();
            for socket in interface_ref.get_sockets().borrow().iter() {
                sockets.push(socket.clone());
            }
        }
        sockets.clone()
    }

    fn get_socket_interface_properties(
        interfaces: &HashMap<ComInterfaceUUID, Rc<RefCell<dyn ComInterface>>>,
        socket: &ComInterfaceSocket,
    ) -> InterfaceProperties {
        interfaces
            .get(&socket.interface_uuid)
            .unwrap()
            .borrow()
            .get_properties()
    }

    fn iterate_endpoint_sockets<'a>(
        &'a self,
        endpoint: &'a Endpoint,
        options: EndpointIterateOptions,
    ) -> impl Iterator<Item = &'a ComInterfaceSocket> + 'a {
        let endpoint_sockets = self.endpoint_sockets.get(&endpoint);
        let interfaces = &self.interfaces;

        std::iter::from_coroutine(
            #[coroutine]
            move || {
                for (socket, _) in endpoint_sockets.unwrap() {
                    // check if is direct socket if only_redirect is set to true
                    if !options.only_direct
                        && match &socket.endpoint {
                            Some(e) => e == endpoint,
                            _ => false,
                        }
                    {
                        continue;
                    }

                    // check if the socket is excluded if exclude_socket is set
                    if let Some(exclude_socket) = &options.exclude_socket {
                        if socket.uuid == *exclude_socket {
                            continue;
                        }
                    }

                    // check if the socket is outgoing if only_outgoing is set to true
                    let properties = ComHub::get_socket_interface_properties(
                        interfaces, socket,
                    );
                    if options.only_outgoing && !properties.can_send() {
                        continue;
                    }
                    yield socket;
                }
            },
        )
    }

    fn find_matching_endpoint_socket<'a>(
        &'a self,
        endpoint: &'a Endpoint,
        exclude_socket: Option<ComInterfaceSocketUUID>,
    ) -> Option<&'a ComInterfaceSocket> {
        // iterate over all sockets of all interfaces
        let options = EndpointIterateOptions {
            only_direct: false,
            only_outgoing: true,
            exact_instance: true,
            exclude_socket: exclude_socket.clone(),
        };
        for socket in self.iterate_endpoint_sockets(&endpoint, options) {
            return Some(socket);
        }

        // no matching socket found, check other instances of the endpoint
        let options = EndpointIterateOptions {
            only_direct: false,
            only_outgoing: true,
            exact_instance: false,
            exclude_socket,
        };
        for socket in self.iterate_endpoint_sockets(&endpoint, options) {
            // TODO
        }
        None
    }

    /**
     * Update all sockets and interfaces,
     * collecting incoming data and sending out queued blocks.
     */
    pub fn update(&mut self) {
        // update sockets
        self.update_sockets();

        // receive blocks from all sockets
        self.receive_incoming_blocks();

        // send all queued blocks from all interfaces
        self.flush_outgoing_blocks();
    }

    /**
     * Send a block to all endpoints specified in block header.
     * The routing algorithm decides which sockets are used to send the block, based on the endpoint.
     * A block can be sent to multiple endpoints at the same time over a socket or to multiple sockets for each endpoint.
     * The original_socket parameter is used to prevent sending the block back to the sender.
     * When this method is called, the block is queued in the send queue.
     */
    pub fn send_block(
        &self,
        block: &DXBBlock,
        original_socket: Option<&mut ComInterfaceSocket>,
    ) {
        // TODO: routing
        for socket in &self.iterate_all_sockets() {
            let mut socket_ref = socket.borrow_mut();

            match &block.to_bytes() {
                Ok(bytes) => {
                    socket_ref.queue_outgoing_block(bytes);
                }
                Err(err) => {
                    eprintln!("Failed to convert block to bytes: {:?}", err);
                }
            }
        }
    }

    fn update_sockets(&self) {
        // update sockets, collect incoming data into full blocks
        info!("Collecting incoming data from all sockets");
        for socket in &self.iterate_all_sockets() {
            let mut socket_ref = socket.borrow_mut();
            socket_ref.collect_incoming_data();
        }
    }

    /**
     * Collect all blocks from the receive queues of all sockets and process them
     * in the receive_block method.
     */
    fn receive_incoming_blocks(&mut self) {
        // iterate over all sockets
        for socket in &self.iterate_all_sockets() {
            let socket_ref = socket.borrow();
            let block_queue = socket_ref.get_incoming_block_queue();
            for block in block_queue {
                self.receive_block(block, socket);
            }
        }
    }

    /**
     * Send all queued blocks from all interfaces.
     */
    fn flush_outgoing_blocks(&mut self) {
        for (_, interface) in &self.interfaces {
            interface.borrow_mut().flush_outgoing_blocks();
        }
    }
}
