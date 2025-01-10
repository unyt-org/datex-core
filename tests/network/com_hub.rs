use std::rc::Rc;

use datex_core::network::com_hub::{ComHub};

use datex_core::network::com_interfaces::com_interface::{ComInterface, ComInterfaceTrait};
use datex_core::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};

pub struct MockupInterface {
    last_block: Option<Vec<u8>>,
}

impl MockupInterface {
    pub fn new() -> MockupInterface {
        return MockupInterface { last_block: None };
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
}


#[test]
pub fn test_add_and_remove() {
	let mut com_hub = ComHub::new();
	let mockup_interface = ComInterfaceTrait::new(Rc::new(MockupInterface::new()));
	assert!(com_hub.add_interface(mockup_interface.clone()));
	assert!(com_hub.remove_interface(mockup_interface));
}

#[test]
pub fn test_multiple_add() {
	let mut com_hub = ComHub::new();
	let mockup_interface1 = ComInterfaceTrait::new(Rc::new(MockupInterface::new()));
	let mockup_interface2 = ComInterfaceTrait::new(Rc::new(MockupInterface::new()));
	assert!(com_hub.add_interface(mockup_interface1.clone()));
	assert!(com_hub.add_interface(mockup_interface2.clone()));
	assert_eq!(com_hub.add_interface(mockup_interface1.clone()), false);
	assert_eq!(com_hub.add_interface(mockup_interface2.clone()), false);
}
