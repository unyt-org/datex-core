use datex_core::network::com_interfaces::{
    com_interface::ComInterface, com_interface_properties::InterfaceDirection,
    default_com_interfaces::base_interface::BaseInterface,
    socket_provider::MultipleSocketProvider,
};

use crate::context::init_global_context;

#[tokio::test]
pub async fn test_construct() {
    const MESSAGE_A_TO_B: &[u8] = b"Hello from A";
    const MESSAGE_B_TO_A: &[u8] = b"Hello from B";
    const MESSAGE_C_TO_A: &[u8] = b"Hello from C";

    init_global_context();
    let mut base_interface = BaseInterface::new("mockup");

    let socket_a_uuid =
        base_interface.register_new_socket(InterfaceDirection::IN_OUT);
    let socket_b_uuid =
        base_interface.register_new_socket(InterfaceDirection::IN_OUT);

    assert!(
        base_interface
            .send_block(MESSAGE_A_TO_B, socket_a_uuid.clone())
            .await
    );
    assert!(
        base_interface
            .send_block(MESSAGE_B_TO_A, socket_b_uuid.clone())
            .await
    );

    {
        // check socket a queue
        let socket = base_interface
            .get_socket_with_uuid(socket_a_uuid.clone())
            .unwrap();
        let queue = socket.lock().unwrap().receive_queue.clone();
        let mut queue = queue.lock().unwrap();
        let vec: Vec<u8> = queue.iter().cloned().collect();
        assert_eq!(vec, MESSAGE_A_TO_B);
        queue.clear();
    }
    {
        // check socket b queue
        let socket = base_interface
            .get_socket_with_uuid(socket_b_uuid.clone())
            .unwrap();
        let queue = socket.lock().unwrap().receive_queue.clone();
        let mut queue = queue.lock().unwrap();
        let vec: Vec<u8> = queue.iter().cloned().collect();
        assert_eq!(vec, MESSAGE_B_TO_A);
        queue.clear();
    }

    assert!(base_interface
        .receive(socket_a_uuid.clone(), MESSAGE_C_TO_A.to_vec())
        .is_ok());
    {
        let socket = base_interface
            .get_socket_with_uuid(socket_a_uuid.clone())
            .unwrap();
        let queue = socket.lock().unwrap().receive_queue.clone();
        let queue = queue.lock().unwrap();
        let vec: Vec<u8> = queue.iter().cloned().collect();
        assert_eq!(vec, MESSAGE_C_TO_A);
    }
}
