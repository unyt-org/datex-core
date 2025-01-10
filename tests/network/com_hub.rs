use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::rc::Rc;

use datex_core::network::com_hub::ComHub;

use datex_core::network::com_interfaces::com_interface::{
    ComInterface, ComInterfaceHandler, ComInterfaceTrait,
};
use datex_core::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};

pub struct MockupInterface {
    pub handler: ComInterfaceHandler,
    pub last_block: Option<Vec<u8>>,
}

impl MockupInterface {
    pub fn new(com_hub: Rc<RefCell<ComHub>>) -> ComInterfaceTrait {
        return ComInterfaceTrait::new(Rc::new(MockupInterface {
            handler: ComInterfaceHandler { com_hub },
            last_block: None,
        }));
    }
}

impl ComInterface for MockupInterface {
    fn send_block(&mut self, block: &[u8]) -> () {
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

    fn get_com_interface_handler(&self) -> &ComInterfaceHandler {
        &self.handler
    }
}

#[test]
pub fn test_add_and_remove() {
    let com_hub  = ComHub::new();	
    let mockup_interface = MockupInterface::new(com_hub.clone());
    // assert!(com_hub.borrow_mut().add_interface(mockup_interface.clone()));
    // assert!(com_hub.borrow_mut().remove_interface(mockup_interface));
}

#[test]
pub fn test_multiple_add() {
    let mut com_hub = ComHub::new();
    let mockup_interface1 = MockupInterface::new(com_hub.clone());
    // let mockup_interface2 = MockupInterface::new(com_hub.clone());

	let x = com_hub.borrow_mut();
	x.get_mut().add_interface(mockup_interface1.clone());


    // assert!(com_hub.borrow_mut().add_interface(mockup_interface1.clone()));
    // assert!(com_hub.borrow_mut().add_interface(mockup_interface2.clone()));
    // assert_eq!(com_hub.borrow_mut().add_interface(mockup_interface1.clone()), false);
    // assert_eq!(com_hub.borrow_mut().add_interface(mockup_interface2.clone()), false);
}

#[test]
pub fn test_send() {
    let com_hub = ComHub::new();

	let mockup_in = MockupInterface::new(com_hub.clone());
    let mockup_out = MockupInterface::new(com_hub.clone());

    // com_hub.borrow_mut().add_interface(mockup_in.clone());
    // com_hub.borrow_mut().add_interface(mockup_out.clone());

    // mockup_in.interface.borrow_mut().send_block(&[1, 2, 3]);
}
