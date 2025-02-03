use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::Write;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use datex_core::global::dxb_block::DXBBlock;
use datex_core::global::protocol_structures::encrypted_header::{self, EncryptedHeader};
use datex_core::global::protocol_structures::routing_header::RoutingHeader;
use datex_core::network::com_hub::ComHub;

use datex_core::network::com_interfaces::com_interface::{ComInterface, ComInterfaceTrait};
use datex_core::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use datex_core::network::com_interfaces::com_interface_socket::ComInterfaceSocket;

pub struct MockupInterface {
    pub last_block: Option<Vec<u8>>,
    pub sockets: Rc<RefCell<Vec<Rc<RefCell<ComInterfaceSocket>>>>>,
}

impl MockupInterface {
    // pub fn new() -> ComInterfaceTrait {
    //     return ComInterfaceTrait::new(Rc::new(RefCell::new(
    //         MockupInterface {
    //             last_block: None,
    //         }
    //     )));
    // }

    pub fn default_com_interface_trait() -> ComInterfaceTrait {
        return ComInterfaceTrait::new(Rc::new(RefCell::new(MockupInterface {
            last_block: None,
            sockets: Rc::new(RefCell::new(Vec::new())),
        })));
    }

    pub fn get_com_interface_trait(mockup_interface: Rc<RefCell<MockupInterface>>) -> ComInterfaceTrait {
        return ComInterfaceTrait::new(mockup_interface);
    }
}


impl Default for MockupInterface {
    fn default() -> Self {
        MockupInterface {
            last_block: None,
            sockets: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl ComInterface for MockupInterface {
    fn send_block(&mut self, block: &[u8], socket: &ComInterfaceSocket) -> () {
        self.last_block = Some(block.to_vec());
    }

    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "mockup".to_string(),
            name: None,
            direction: InterfaceDirection::IN_OUT,
            reconnect_interval: None,
            latency: 0,
            bandwidth: 1000,
            continuous_connection: true,
            allow_redirects: true,
        }
    }

    fn get_sockets(&self) -> Rc<RefCell<Vec<Rc<RefCell<ComInterfaceSocket>>>>> {
        self.sockets.clone()
    }
}


fn get_mock_setup() -> (Rc<RefCell<ComHub>>, Rc<RefCell<MockupInterface>>, ComInterfaceTrait, Rc<RefCell<ComInterfaceSocket>>) {
    // init com hub
    let com_hub = ComHub::new();
    let mut com_hub_mut = com_hub.borrow_mut();

    // init mockup interface
    let mock_interface = MockupInterface::default();
    let mockup_interface_in_ref = Rc::new(RefCell::new(mock_interface));
    let mockup_in_trait = MockupInterface::get_com_interface_trait(mockup_interface_in_ref.clone());

    // add mockup interface to com hub
    com_hub_mut.add_interface(mockup_in_trait.clone());

    let socket = Rc::new(RefCell::new(
        ComInterfaceSocket {
            uuid: "mockup_in_socket".to_string(),
            ..Default::default()
        }
    ));

    // add socket to mockup interface
    mockup_in_trait.add_socket(socket.clone());

    (
        com_hub.clone(),
        mockup_interface_in_ref,
        mockup_in_trait,
        socket,
    )
}


#[test]
pub fn test_add_and_remove() {
    let com_hub = &mut ComHub::new();
    let mut com_hub_mut = com_hub.borrow_mut();
    let mockup_interface = MockupInterface::default_com_interface_trait();

    assert!(com_hub_mut.add_interface(mockup_interface.clone()));
    assert!(com_hub_mut.remove_interface(mockup_interface));
}

#[test]
pub fn test_multiple_add() {
    let com_hub = &mut ComHub::new();
    let mut com_hub_mut = com_hub.borrow_mut();

    let mockup_interface1: ComInterfaceTrait = MockupInterface::default_com_interface_trait();
    let mockup_interface2 = MockupInterface::default_com_interface_trait();

    assert!(com_hub_mut.add_interface(mockup_interface1.clone()));
    assert!(com_hub_mut.add_interface(mockup_interface2.clone()));
    assert_eq!(com_hub_mut.add_interface(mockup_interface1.clone()), false);
    assert_eq!(com_hub_mut.add_interface(mockup_interface2.clone()), false);
}

#[test]
pub fn test_send() {

    // init mock setup
    let (com_hub, com_interface, _, _) = get_mock_setup();

    // send block
    let block: DXBBlock = DXBBlock::default();
    let mut com_hub_mut = com_hub.borrow_mut();
    com_hub_mut.send_block(&block, None);
    com_hub_mut.update();

    // get last block that was sent
    let mockup_interface_out = com_interface.clone();
    let mockup_interface_out = mockup_interface_out.borrow();
    let block_bytes = mockup_interface_out.last_block.as_ref().unwrap();

    assert!(mockup_interface_out.last_block.is_some());
    assert_eq!(block_bytes, &block.to_bytes().unwrap());
}

#[test]
pub fn test_receive() {
    // init mock setup
    let (com_hub, _, _, socket) = get_mock_setup();
    let mut com_hub_mut = com_hub.borrow_mut();

    // receive block
    let block = DXBBlock {
        body: vec![0x01, 0x02, 0x03],
        encrypted_header: EncryptedHeader {
            flags: encrypted_header::Flags::new().with_device_type(
                encrypted_header::DeviceType::Unused11,
            ),
            ..Default::default()
        },
        routing_header: RoutingHeader {
            block_size_u16: Some(62 + 3),
            ..Default::default()
        },
        ..DXBBlock::default()
    };
    let block_bytes = block.to_bytes().unwrap();

    {
        let socket_ref = socket.borrow();
        let receive_queue = socket_ref.get_receive_queue();
        let mut receive_queue_mut = receive_queue.lock().unwrap();
        let _ = receive_queue_mut.write(block_bytes.as_slice());
    }

    com_hub_mut.update();

    let incoming_blocks_ref = com_hub_mut.incoming_blocks.clone();
    let incoming_blocks = incoming_blocks_ref.borrow();

    assert_eq!(incoming_blocks.len(), 1);
    let incoming_block = incoming_blocks.front().unwrap();
    assert_eq!(incoming_block.raw_bytes.clone().unwrap(), block_bytes);
}

#[test]
pub fn test_receive_multiple() {
    // init mock setup
    let (com_hub, _, _, socket) = get_mock_setup();
    let mut com_hub_mut = com_hub.borrow_mut();

    // receive block
    let blocks = vec![
        DXBBlock {
            routing_header: RoutingHeader {
                block_index: 0,
                ..Default::default()
            },
            ..Default::default()
        },
        DXBBlock {
            routing_header: RoutingHeader {
                block_index: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        DXBBlock {
            routing_header: RoutingHeader {
                block_index: 2,
                ..Default::default()
            },
            ..Default::default()
        },
    ];
    let block_bytes: Vec<Vec<u8>> = blocks.iter().map(|block| block.to_bytes().unwrap()).collect();

    {
        let socket_ref = socket.borrow();
        let receive_queue = socket_ref.get_receive_queue();
        let mut receive_queue_mut = receive_queue.lock().unwrap();
        for block in block_bytes.iter() {
            let _ = receive_queue_mut.write(&block);
        }
    }

    com_hub_mut.update();

    let incoming_blocks_ref = com_hub_mut.incoming_blocks.clone();
    let incoming_blocks = incoming_blocks_ref.borrow();

    assert_eq!(incoming_blocks.len(), blocks.len());

    for (incoming_block, block) in incoming_blocks.iter().zip(blocks.iter()) {
        assert_eq!(incoming_block.raw_bytes.clone().unwrap(), block.to_bytes().unwrap());
    }
}

#[test]
pub fn test_send_receive() {

}
