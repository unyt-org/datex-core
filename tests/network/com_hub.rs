
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use datex_core::network::com_hub::ComHub;

use datex_core::network::com_interfaces::com_interface::{
    ComInterface, ComInterfaceTrait,
};
use datex_core::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use datex_core::network::com_interfaces::com_interface_socket::ComInterfaceSocket;

pub struct MockupInterface {
    pub last_block: Option<Vec<u8>>,
    pub queue: Arc<Mutex<VecDeque<u8>>>
}

impl MockupInterface {
    // pub fn new() -> ComInterfaceTrait {
    //     return ComInterfaceTrait::new(Rc::new(RefCell::new(
    //         MockupInterface {
    //             last_block: None,
    //         }
    //     )));
    // }

    pub fn default() -> ComInterfaceTrait {
        return ComInterfaceTrait::new(Rc::new(RefCell::new(
            MockupInterface {
                last_block: None,
                queue: Arc::new(Mutex::new(VecDeque::new()))
            }
        )));
    }

    pub fn new(mockup_interface: MockupInterface) -> ComInterfaceTrait {
        return ComInterfaceTrait::new(Rc::new(RefCell::new(
            mockup_interface
        )));
    }
}

impl ComInterface for MockupInterface {
    fn send_block(&mut self, block: &[u8], socket: ComInterfaceSocket) -> () {
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
    
    fn get_receive_queue(&mut self, socket: ComInterfaceSocket) -> Option<std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<u8>>>> {
        Some(self.queue.clone())
    }
}

#[test]
pub fn test_add_and_remove() {
    let com_hub = &mut ComHub::new();
    let mut com_hub_mut = com_hub.borrow_mut();
    let mockup_interface = MockupInterface::default();

    assert!(com_hub_mut.add_interface(mockup_interface.clone()));
    assert!(com_hub_mut.remove_interface(mockup_interface));
}

#[test]
pub fn test_multiple_add() {
    let com_hub = &mut ComHub::new();
    let mut com_hub_mut = com_hub.borrow_mut();	

    let mockup_interface1: ComInterfaceTrait = MockupInterface::default();
    let mockup_interface2 = MockupInterface::default();

    assert!(com_hub_mut.add_interface(mockup_interface1.clone()));
    assert!(com_hub_mut.add_interface(mockup_interface2.clone()));
    assert_eq!(com_hub_mut.add_interface(mockup_interface1.clone()), false);
    assert_eq!(com_hub_mut.add_interface(mockup_interface2.clone()), false);
}

#[test]
pub fn test_send() {
    let com_hub = ComHub::new();

    let mockup_in = MockupInterface {
        last_block: None,
        queue: Arc::new(Mutex::new(VecDeque::new()))
    };
    
    let mockup_out = MockupInterface {
        last_block: None,
        queue: Arc::new(Mutex::new(VecDeque::new()))
    };

    let mockup_in_rc = Rc::new(RefCell::new(
        mockup_in
    ));
    
    let mockup_in_trait = ComInterfaceTrait::new(mockup_in_rc.clone());

    let mockup_out_trait = MockupInterface::new(mockup_out);

    {
        let mut com_hub_mut = com_hub.borrow_mut();	
        com_hub_mut.add_interface(mockup_in_trait.clone());
        com_hub_mut.add_interface(mockup_out_trait.clone());
    }


    let mockup_in_ref = mockup_in_rc.borrow_mut();
    let mut queue = mockup_in_ref.queue.lock().unwrap();
        
    // Push the value onto the VecDeque
    queue.push_back(1);

    // let interface = &mut mockup_in_trait.interface;
    // let mut interface_mut = interface.borrow_mut();
    //interface_mut.receive_block(&[1, 2, 3, 4]);
}
