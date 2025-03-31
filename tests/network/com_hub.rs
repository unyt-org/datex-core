use datex_core::datex_values::Endpoint;
use datex_core::global::dxb_block::DXBBlock;
use datex_core::global::protocol_structures::encrypted_header::{
    self, EncryptedHeader,
};
use datex_core::global::protocol_structures::routing_header::RoutingHeader;
use datex_core::network::com_hub::ComHub;
use datex_core::stdlib::cell::RefCell;
use datex_core::stdlib::rc::Rc;
use std::io::Write;
// FIXME no-std

use datex_core::network::com_interfaces::com_interface::{
    ComInterface, ComInterfaceError, ComInterfaceUUID,
};
use datex_core::network::com_interfaces::com_interface_properties::InterfaceProperties;
use datex_core::network::com_interfaces::com_interface_socket::ComInterfaceSocket;
use datex_core::utils::uuid::UUID;

use crate::context::init_global_context;

pub struct MockupInterface {
    pub last_block: Option<Vec<u8>>,
    pub sockets: Rc<RefCell<Vec<Rc<RefCell<ComInterfaceSocket>>>>>,
    uuid: ComInterfaceUUID,
}

impl MockupInterface {}

impl Default for MockupInterface {
    fn default() -> Self {
        MockupInterface {
            last_block: None,
            sockets: Rc::new(RefCell::new(Vec::new())),
            uuid: ComInterfaceUUID(UUID::new()),
        }
    }
}

impl ComInterface for MockupInterface {
    fn send_block(&mut self, block: &[u8], socket: &ComInterfaceSocket) {
        self.last_block = Some(block.to_vec());
    }

    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "mockup".to_string(),
            ..Default::default()
        }
    }

    fn get_sockets(&self) -> Rc<RefCell<Vec<Rc<RefCell<ComInterfaceSocket>>>>> {
        self.sockets.clone()
    }

    fn connect(&mut self) -> Result<(), ComInterfaceError> {
        Ok(())
    }

    fn get_uuid(&self) -> ComInterfaceUUID {
        self.uuid.clone()
    }
}

fn get_mock_setup() -> (
    Rc<RefCell<ComHub>>,
    Rc<RefCell<MockupInterface>>,
    Rc<RefCell<ComInterfaceSocket>>,
) {
    // init com hub
    let com_hub = Rc::new(RefCell::new(ComHub::default()));
    let mut com_hub_mut = com_hub.borrow_mut();

    // init mockup interface
    let mockup_interface_ref =
        Rc::new(RefCell::new(MockupInterface::default()));

    // add mockup interface to com_hub
    com_hub_mut
        .add_interface(mockup_interface_ref.clone())
        .unwrap_or_else(|e| {
            panic!("Error adding interface: {:?}", e);
        });

    let socket = Rc::new(RefCell::new(ComInterfaceSocket::new(
        mockup_interface_ref.borrow().uuid.clone(),
    )));

    {
        let mockup_interface = mockup_interface_ref.borrow_mut();
        // add socket to mockup interface
        mockup_interface.add_socket(socket.clone());
    }

    (com_hub.clone(), mockup_interface_ref, socket)
}

#[test]
pub fn test_add_and_remove() {
    init_global_context();
    let com_hub = Rc::new(RefCell::new(ComHub::default()));
    let mut com_hub_mut = com_hub.borrow_mut();
    let mockup_interface = Rc::new(RefCell::new(MockupInterface::default()));

    com_hub_mut
        .add_interface(mockup_interface.clone())
        .unwrap_or_else(|e| {
            panic!("Error adding interface: {:?}", e);
        });
    assert!(com_hub_mut.remove_interface(mockup_interface));
}

#[test]
pub fn test_multiple_add() {
    init_global_context();

    let com_hub = Rc::new(RefCell::new(ComHub::default()));
    let mut com_hub_mut = com_hub.borrow_mut();

    let mockup_interface1 = Rc::new(RefCell::new(MockupInterface::default()));
    let mockup_interface2 = Rc::new(RefCell::new(MockupInterface::default()));

    com_hub_mut
        .add_interface(mockup_interface1.clone())
        .unwrap_or_else(|e| {
            panic!("Error adding interface: {:?}", e);
        });
    com_hub_mut
        .add_interface(mockup_interface2.clone())
        .unwrap_or_else(|e| {
            panic!("Error adding interface: {:?}", e);
        });

    assert!(com_hub_mut
        .add_interface(mockup_interface1.clone())
        .is_err());
    assert!(com_hub_mut
        .add_interface(mockup_interface2.clone())
        .is_err());
}

#[test]
pub fn test_send() {
    // init mock setup
    init_global_context();
    let (com_hub, com_interface, _) = get_mock_setup();

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
pub fn test_recalculate() {
    init_global_context();

    let mut block = DXBBlock {
        body: vec![0x01, 0x02, 0x03],
        encrypted_header: EncryptedHeader {
            flags: encrypted_header::Flags::new()
                .with_device_type(encrypted_header::DeviceType::Unused11),
            ..Default::default()
        },
        routing_header: RoutingHeader {
            block_size_u16: Some(420),
            sender: Endpoint::new_from_string("@test").unwrap(),
            ..Default::default()
        },
        ..DXBBlock::default()
    };

    {
        // invalid block size
        let block_bytes = block.to_bytes().unwrap();
        let block2: DXBBlock = DXBBlock::from_bytes(&block_bytes).unwrap();
        assert_ne!(block, block2);
    }

    {
        // valid block size
        block.recalculate_struct();
        let block_bytes = block.to_bytes().unwrap();
        let block3: DXBBlock = DXBBlock::from_bytes(&block_bytes).unwrap();
        assert_eq!(block, block3);
    }
}

#[test]
pub fn test_receive() {
    // init mock setup
    init_global_context();
    let (com_hub, _, socket) = get_mock_setup();
    let mut com_hub_mut = com_hub.borrow_mut();

    // receive block
    let mut block = DXBBlock {
        body: vec![0x01, 0x02, 0x03],
        encrypted_header: EncryptedHeader {
            flags: encrypted_header::Flags::new()
                .with_device_type(encrypted_header::DeviceType::Unused11),
            ..Default::default()
        },
        ..DXBBlock::default()
    };
    block.recalculate_struct();

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
    init_global_context();
    let (com_hub, _, socket) = get_mock_setup();
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
    let block_bytes: Vec<Vec<u8>> = blocks
        .iter()
        .map(|block| block.to_bytes().unwrap())
        .collect();

    {
        let socket_ref = socket.borrow();
        let receive_queue = socket_ref.get_receive_queue();
        let mut receive_queue_mut = receive_queue.lock().unwrap();
        for block in block_bytes.iter() {
            let _ = receive_queue_mut.write(block);
        }
    }

    com_hub_mut.update();

    let incoming_blocks_ref = com_hub_mut.incoming_blocks.clone();
    let incoming_blocks = incoming_blocks_ref.borrow();

    assert_eq!(incoming_blocks.len(), blocks.len());

    for (incoming_block, block) in incoming_blocks.iter().zip(blocks.iter()) {
        assert_eq!(
            incoming_block.raw_bytes.clone().unwrap(),
            block.to_bytes().unwrap()
        );
    }
}

#[test]
pub fn test_send_receive() {}
