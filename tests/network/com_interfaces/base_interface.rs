use std::cell::RefCell;
use std::rc::Rc;

use datex_core::network::com_interfaces::com_interface;
use datex_core::network::com_interfaces::default_com_interfaces::base_interface::BaseInterfaceHolder;
use datex_core::stdlib::{future::Future, pin::Pin};

use datex_core::utils::context::init_global_context;
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

use crate::network::helpers::shared_lazy_value::SharedLazyValue;

#[tokio::test]
pub async fn test_construct() {
    const MESSAGE_A_TO_B: &[u8] = b"Hello from A";
    const MESSAGE_B_TO_A: &[u8] = b"Hello from B";
    init_global_context();

    let base_interface_a = SharedLazyValue::<BaseInterfaceHolder>::new();
    let base_interface_b = SharedLazyValue::<BaseInterfaceHolder>::new();
    let socket_a_uuid = SharedLazyValue::<ComInterfaceSocketUUID>::new();
    let socket_b_uuid = SharedLazyValue::<ComInterfaceSocketUUID>::new();

    let base_interface_a_clone = base_interface_a.clone();
    let base_interface_b_clone = base_interface_b.clone();
    let socket_a_uuid_clone = socket_a_uuid.clone();
    let socket_b_uuid_clone = socket_b_uuid.clone();

    let callback_a: Box<
        dyn Fn(
            &[u8],
            ComInterfaceSocketUUID,
        ) -> Pin<Box<dyn Future<Output = bool>>>,
    > = Box::new(
        move |data: &[u8],
              receiver_socket_uuid: ComInterfaceSocketUUID|
              -> Pin<Box<dyn Future<Output = bool>>> {
            // Make sure the receiver socket is the one we expect
            assert_eq!(
                receiver_socket_uuid,
                *socket_a_uuid_clone.get(),
                "Receiver socket uuid does not match"
            );
            let ok = base_interface_b_clone
                .get_mut()
                .receive(socket_a_uuid_clone.get().clone(), data.to_vec())
                .is_ok();
            assert!(ok, "Failed to receive data");
            Box::pin(async move { ok })
        },
    );

    let callback_b: Box<
        dyn Fn(
            &[u8],
            ComInterfaceSocketUUID,
        ) -> Pin<Box<dyn Future<Output = bool>>>,
    > = Box::new(
        move |data: &[u8],
              receiver_socket_uuid: ComInterfaceSocketUUID|
              -> Pin<Box<dyn Future<Output = bool>>> {
            // Make sure the receiver socket is the one we expect
            assert_eq!(
                receiver_socket_uuid,
                *socket_b_uuid_clone.get(),
                "Receiver socket uuid does not match"
            );
            let ok = base_interface_a_clone
                .get_mut()
                .receive(socket_b_uuid_clone.get().clone(), data.to_vec())
                .is_ok();
            assert!(ok, "Failed to receive data");
            Box::pin(async move { ok })
        },
    );

    base_interface_a.set(BaseInterfaceHolder::new(
        BaseInterfaceSetupData::with_callback(callback_a),
    ));
    base_interface_b.set(BaseInterfaceHolder::new(
        BaseInterfaceSetupData::with_callback(callback_b),
    ));

    // This is a socket of mockup-a connected to mockup-b
    let (socket_a_uuid_inner, mut send_a_to_b) = base_interface_a
        .get_mut()
        .register_new_socket_with_endpoint(
            InterfaceDirection::Out,
            Endpoint::new("mockup-b"),
        );
    socket_a_uuid.set(socket_a_uuid_inner);

    // This is a socket of mockup-b connected to mockup-a
    let (socket_b_uuid_inner, mut send_b_to_a) = base_interface_b
        .get_mut()
        .register_new_socket_with_endpoint(
            InterfaceDirection::Out,
            Endpoint::new("mockup-a"),
        );
    socket_b_uuid.set(socket_b_uuid_inner);

    // Send a message from mockup-a to mockup-b via socket_a
    send_a_to_b.start_send(MESSAGE_A_TO_B.to_vec()).unwrap();

    // Send a message from mockup-b to mockup-a via socket_b
    send_b_to_a.start_send(MESSAGE_B_TO_A.to_vec()).unwrap();

    base_interface_a.get().com_interface.close().await;
    base_interface_b.get().com_interface.close().await;
}
