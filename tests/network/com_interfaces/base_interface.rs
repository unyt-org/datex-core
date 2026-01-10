use datex_core::stdlib::{future::Future, pin::Pin};

use datex_core::{
    network::com_interfaces::{
        com_interface::{
            ComInterface, properties::InterfaceDirection,
            socket::ComInterfaceSocketUUID, state::ComInterfaceState,
        },
        default_com_interfaces::base_interface::{
            BaseInterface, BaseInterfaceSetupData,
        },
    },
    values::core_values::endpoint::Endpoint,
};

use crate::context::init_global_context;

#[tokio::test]
pub async fn test_construct() {
    const MESSAGE_A_TO_B: &[u8] = b"Hello from A";
    const MESSAGE_B_TO_A: &[u8] = b"Hello from B";

    init_global_context();
    let com_interface_a = ComInterface::create_with_implementation::<
        BaseInterface,
    >(BaseInterfaceSetupData::default())
    .expect("Failed to create BaseInterface");

    let com_interface_b = ComInterface::create_with_implementation::<
        BaseInterface,
    >(BaseInterfaceSetupData::default())
    .expect("Failed to create BaseInterface");

    let com_interface_a_clone = com_interface_a.clone();
    let mut com_interface_a_borrow = com_interface_a_clone.borrow_mut();
    let base_interface_a =
        com_interface_a_borrow.implementation_mut::<BaseInterface>();

    let com_interface_b_clone = com_interface_b.clone();
    let mut com_interface_b_borrow = com_interface_b_clone.borrow_mut();
    let base_interface_b =
        com_interface_b_borrow.implementation_mut::<BaseInterface>();

    // This is a socket of mockup-a connected to mockup-b
    let socket_a_uuid = base_interface_a.register_new_socket_with_endpoint(
        InterfaceDirection::Out,
        Endpoint::new("mockup-b"),
    );

    // This is a socket of mockup-b connected to mockup-a
    let socket_b_uuid = base_interface_b.register_new_socket_with_endpoint(
        InterfaceDirection::Out,
        Endpoint::new("mockup-a"),
    );

    {
        let socket_b_uuid = socket_b_uuid.clone();
        let socket_a_uuid = socket_a_uuid.clone();
        let com_interface_b = com_interface_b.clone();
        // This method get's called when we call the sendBlock of mockup-a to
        // send a message to mockup-b via socket_a
        base_interface_a.set_on_send_callback(Box::new(
            move |data: &[u8],
                  receiver_socket_uuid: ComInterfaceSocketUUID|
                  -> Pin<Box<dyn Future<Output = bool>>> {
                // Make sure the receiver socket is the one we expect
                assert_eq!(
                    receiver_socket_uuid, socket_a_uuid,
                    "Receiver socket uuid does not match"
                );
                let ok = com_interface_b
                    .borrow_mut()
                    .implementation_mut::<BaseInterface>()
                    .receive(socket_b_uuid.clone(), data.to_vec())
                    .is_ok();
                assert!(ok, "Failed to receive data");
                Box::pin(async move { ok })
            },
        ));
    }

    {
        let socket_a_uuid = socket_a_uuid.clone();
        let socket_b_uuid = socket_b_uuid.clone();
        let com_interface_a = com_interface_a.clone();
        // This method get's called when we call the sendBlock of mockup-b to
        // send a message to mockup-a via socket_b
        base_interface_b.set_on_send_callback(Box::new(
            move |data: &[u8],
                  receiver_socket_uuid: ComInterfaceSocketUUID|
                  -> Pin<Box<dyn Future<Output = bool>>> {
                // Make sure the receiver socket is the one we expect
                assert_eq!(
                    receiver_socket_uuid, socket_b_uuid,
                    "Receiver socket uuid does not match"
                );

                let ok = com_interface_a
                    .borrow_mut()
                    .implementation_mut::<BaseInterface>()
                    .receive(socket_a_uuid.clone(), data.to_vec())
                    .is_ok();
                assert!(ok, "Failed to receive data");
                Box::pin(async move { ok })
            },
        ));
    }
    drop(base_interface_a);
    drop(base_interface_b);

    // Send a message from mockup-a to mockup-b via socket_a
    let mut com_interface_a_borrow = com_interface_a.borrow_mut();
    assert!(
        com_interface_a_borrow
            .send_block(MESSAGE_A_TO_B, socket_a_uuid.clone())
            .await,
        "Failed to send message from A to B"
    );

    // Send a message from mockup-b to mockup-a via socket_b
    let mut com_interface_b_borrow = com_interface_b.borrow_mut();
    assert!(
        com_interface_b_borrow
            .send_block(MESSAGE_B_TO_A, socket_b_uuid.clone())
            .await,
        "Failed to send message from B to A"
    );

    com_interface_a_borrow.close().await;
    com_interface_b_borrow.close().await;
}
