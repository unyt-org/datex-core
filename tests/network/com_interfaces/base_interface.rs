use std::{cell::RefCell, future::Future, pin::Pin, rc::Rc, time::Duration};

use datex_core::network::com_interfaces::{
    com_interface_old::{ComInterfaceOld, ComInterfaceState},
    com_interface_properties::{
        InterfaceDirection, InterfaceProperties, ReconnectionConfig,
    },
    com_interface_socket::ComInterfaceSocketUUID,
    default_com_interfaces::base_interface::BaseInterface,
    socket_provider::MultipleSocketProvider,
};

use crate::context::init_global_context;

#[tokio::test]
pub async fn test_close() {
    init_global_context();
    // Create a new interface
    let mut base_interface =
        BaseInterface::new_with_properties(InterfaceProperties {
            reconnection_config: ReconnectionConfig::ReconnectWithTimeout {
                timeout: Duration::from_secs(1),
            },
            ..InterfaceProperties::default()
        });
    assert_eq!(base_interface.get_state(), ComInterfaceState::NotConnected);
    assert!(base_interface.get_properties().close_timestamp.is_none());

    // Open the interface
    base_interface.open().unwrap();
    assert_eq!(base_interface.get_state(), ComInterfaceState::Connected);
    assert!(base_interface.get_properties().close_timestamp.is_none());

    // Close the interface
    assert!(base_interface.close().await);
    assert_eq!(base_interface.get_state(), ComInterfaceState::NotConnected);
    assert!(base_interface.get_properties().close_timestamp.is_some());
}

#[tokio::test]
pub async fn test_construct() {
    const MESSAGE_A_TO_B: &[u8] = b"Hello from A";
    const MESSAGE_B_TO_A: &[u8] = b"Hello from B";

    init_global_context();
    let base_interface_a =
        Rc::new(RefCell::new(BaseInterface::new_with_name("mockup-a")));
    let base_interface_b =
        Rc::new(RefCell::new(BaseInterface::new_with_name("mockup-b")));

    // This is a socket of mockup-a connected to mockup-b
    let socket_a_uuid = base_interface_a
        .clone()
        .borrow_mut()
        .register_new_socket(InterfaceDirection::Out);

    // This is a socket of mockup-b connected to mockup-a
    let socket_b_uuid = base_interface_b
        .clone()
        .borrow_mut()
        .register_new_socket(InterfaceDirection::Out);

    let base_interface_b_clone = base_interface_b.clone();
    {
        let socket_b_uuid = socket_b_uuid.clone();
        let socket_a_uuid = socket_a_uuid.clone();

        // This method get's called when we call the sendBlock of mockup-a to
        // send a message to mockup-b via socket_a
        base_interface_a.borrow_mut().set_on_send_callback(Box::new(
            move |data: &[u8],
                  receiver_socket_uuid: ComInterfaceSocketUUID|
                  -> Pin<Box<dyn Future<Output = bool>>> {
                // Make sure the receiver socket is the one we expect
                assert_eq!(
                    receiver_socket_uuid, socket_a_uuid,
                    "Receiver socket uuid does not match"
                );
                let ok = base_interface_b_clone
                    .borrow_mut()
                    .receive(socket_b_uuid.clone(), data.to_vec())
                    .is_ok();
                assert!(ok, "Failed to receive data");
                Box::pin(async move { ok })
            },
        ));
    }

    let base_interface_a_clone = base_interface_a.clone();
    {
        let socket_a_uuid = socket_a_uuid.clone();
        let socket_b_uuid = socket_b_uuid.clone();

        // This method get's called when we call the sendBlock of mockup-b to
        // send a message to mockup-a via socket_b
        base_interface_b.borrow_mut().set_on_send_callback(Box::new(
            move |data: &[u8],
                  receiver_socket_uuid: ComInterfaceSocketUUID|
                  -> Pin<Box<dyn Future<Output = bool>>> {
                // Make sure the receiver socket is the one we expect
                assert_eq!(
                    receiver_socket_uuid, socket_b_uuid,
                    "Receiver socket uuid does not match"
                );

                let ok = base_interface_a_clone
                    .borrow_mut()
                    .receive(socket_a_uuid.clone(), data.to_vec())
                    .is_ok();
                assert!(ok, "Failed to receive data");
                Box::pin(async move { ok })
            },
        ));
    }

    // Send a message from mockup-a to mockup-b via socket_a
    let base_interface_a_clone = base_interface_a.clone();
    assert!(
        base_interface_a_clone
            .clone()
            .borrow_mut()
            .send_block(MESSAGE_A_TO_B, socket_a_uuid.clone())
            .await,
        "Failed to send message from A to B"
    );

    // Send a message from mockup-b to mockup-a via socket_b
    let base_interface_b_clone = base_interface_b.clone();
    assert!(
        base_interface_b_clone
            .clone()
            .borrow_mut()
            .send_block(MESSAGE_B_TO_A, socket_b_uuid.clone())
            .await,
        "Failed to send message from B to A"
    );

    {
        // check receive queue of socket_a
        let socket = base_interface_a
            .clone()
            .borrow_mut()
            .get_socket_with_uuid(socket_a_uuid.clone())
            .unwrap();
        // FIXME update loop
        // let queue = socket.try_lock().unwrap().receive_queue.clone();
        // let mut queue = queue.try_lock().unwrap();
        // let vec: Vec<u8> = queue.iter().cloned().collect();
        // assert_eq!(vec, MESSAGE_B_TO_A);
        // queue.clear();
    }
    {
        // check receive queue of socket_b
        let socket = base_interface_b
            .clone()
            .borrow_mut()
            .get_socket_with_uuid(socket_b_uuid.clone())
            .unwrap();
        // FIXME update loop
        // let queue = socket.try_lock().unwrap().receive_queue.clone();
        // let mut queue = queue.try_lock().unwrap();
        // let vec: Vec<u8> = queue.iter().cloned().collect();
        // assert_eq!(vec, MESSAGE_A_TO_B);
        // queue.clear();
    }

    base_interface_a.take().destroy().await;
    base_interface_b.take().destroy().await;
}
