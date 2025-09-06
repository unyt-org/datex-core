use std::{cell::RefCell, io::Bytes, rc::Rc, sync::Arc, time::Duration};

use datex_core::{
    network::com_interfaces::{
        com_interface::ComInterface,
        com_interface_socket::ComInterfaceSocketUUID,
        default_com_interfaces::webrtc::{
            webrtc_common::{
                media_tracks::{MediaKind, MediaTrack},
                webrtc_trait::{WebRTCTrait, WebRTCTraitInternal},
            },
            webrtc_native_interface::{TrackLocal, WebRTCNativeInterface},
        },
        socket_provider::SingleSocketProvider,
    },
    run_async,
    task::{sleep, spawn_local},
    utils::uuid::UUID,
};
use ntest_timeout::timeout;
use webrtc::{
    media::Sample,
    track::track_local::track_local_static_sample::TrackLocalStaticSample,
};

use crate::{
    context::init_global_context,
    network::helpers::mock_setup::{TEST_ENDPOINT_A, TEST_ENDPOINT_B},
};

#[tokio::test]
#[timeout(10000)]
pub async fn test_connect() {
    const BLOCK_A_TO_B: &[u8] = b"Hello from A";
    const BLOCK_B_TO_A: &[u8] = b"Hello from B";
    run_async! {
        init_global_context();
        // Create a WebRTCNativeInterface instance on each side (remote: @a)
        let mut interface_a = WebRTCNativeInterface::new(
            TEST_ENDPOINT_A.clone(),
        );
        interface_a.open().await.unwrap();


        // Create a WebRTCNativeInterface instance on each side (remote: @b)
        let mut interface_b = WebRTCNativeInterface::new(
            TEST_ENDPOINT_B.clone(),
        );
        interface_b.open().await.unwrap();

        let interface_a = Rc::new(RefCell::new(interface_a));
        let interface_b = Rc::new(RefCell::new(interface_b));

        let interface_a_clone = interface_a.clone();
        let inteface_b_clone = interface_b.clone();

        // Set up the on_ice_candidate callback for both interfaces
        // The candidate would be transmitted to the other side via some signaling server
        // In this case, we are using a mock setup and since we are in the same process,
        // we can directly call the "add_ice_candidate" callback on the other side
        interface_a.clone().borrow().set_on_ice_candidate(Box::new(move |candidate| {
            let interface_b = inteface_b_clone.clone();
            spawn_local(async move {
                interface_b.clone().borrow().add_ice_candidate(candidate).await.unwrap();
            });
        }));

        interface_b.clone().borrow().set_on_ice_candidate(Box::new(move |candidate| {
            let interface_a = interface_a_clone.clone();
            spawn_local(async move {
                interface_a.clone().borrow().add_ice_candidate(candidate).await.unwrap();
            });
        }));


        // Create an offer on one side and an answer on the other side
        // The initator would send the offer to the other side via some other channel
        // When a connection handshake is planned on both side, the initiator should be
        // picked by the endpoint name or something deterministic that both sides
        // can agree on
        let offer = interface_a.clone().borrow().create_offer().await.unwrap();

        // The offer would be transmitted to the other side via some other channel
        // In this case, we are using a mock setup and since we are in the same process,
        // we can directly call the "create_answer" and "set_answer" callbacks on the other side
        let answer = interface_b.clone().borrow().create_answer(offer).await.unwrap();
        interface_a.clone().borrow().set_answer(answer).await.unwrap();

        // Wait for the data channel and socket to be connected
        interface_a.borrow().wait_for_connection().await.unwrap();
        interface_b.borrow().wait_for_connection().await.unwrap();

        // Since the WebRTC connection interface is a single socket provider,
        // it currently doesn't care about the socket uuid. In the future, we could
        // have different sockets for the same endpoint but with different channel configs
        // such as reliable, unreliable, ordered, unordered, etc.
        let socket_stub = ComInterfaceSocketUUID(UUID::from_string("uuid".to_string()));
        assert!(
            interface_a.clone().borrow_mut().send_block(BLOCK_A_TO_B, socket_stub.clone()).await
        );
        assert!(
            interface_b.clone().borrow_mut().send_block(BLOCK_B_TO_A, socket_stub.clone()).await
        );

        // Wait for the messages to be received
        sleep(Duration::from_secs(1)).await;

        // Drain the receive queues
        let receive_a = {
            let  socket = interface_a.borrow_mut().get_socket();
            let socket = socket.unwrap();
            let socket = socket.lock().unwrap();
            let mut socket = socket.receive_queue.lock().unwrap();
            socket.drain(..).collect::<Vec<_>>()
        };
        let receive_b = {
            let  socket = interface_b.borrow_mut().get_socket();
            let socket = socket.unwrap();
            let socket = socket.lock().unwrap();
            let mut socket = socket.receive_queue.lock().unwrap();
            socket.drain(..).collect::<Vec<_>>()
        };

        // Check if the messages are received correctly
        assert_eq!(receive_a, BLOCK_B_TO_A);
        assert_eq!(receive_b, BLOCK_A_TO_B);
    }
}

#[tokio::test]
#[timeout(10000)]
#[ignore = "Media track not working yet"]
pub async fn test_media_track() {
    run_async! {
        init_global_context();
        // Create a WebRTCNativeInterface instance on each side (remote: @a)
        let mut interface_a = WebRTCNativeInterface::new(
            TEST_ENDPOINT_A.clone(),
        );
        interface_a.open().await.unwrap();


        // Create a WebRTCNativeInterface instance on each side (remote: @b)
        let mut interface_b = WebRTCNativeInterface::new(
            TEST_ENDPOINT_B.clone(),
        );
        interface_b.open().await.unwrap();

        let interface_a = Rc::new(RefCell::new(interface_a));
        let interface_b = Rc::new(RefCell::new(interface_b));

        let interface_a_clone = interface_a.clone();
        let inteface_b_clone = interface_b.clone();

        interface_a.clone().borrow().set_on_ice_candidate(Box::new(move |candidate| {
            let interface_b = inteface_b_clone.clone();
            spawn_local(async move {
                interface_b.clone().borrow().add_ice_candidate(candidate).await.unwrap();
            });
        }));

        interface_b.clone().borrow().set_on_ice_candidate(Box::new(move |candidate| {
            let interface_a = interface_a_clone.clone();
            spawn_local(async move {
                interface_a.clone().borrow().add_ice_candidate(candidate).await.unwrap();
            });
        }));
        let track: Rc<RefCell<MediaTrack<Arc<TrackLocal>>>> = interface_a.borrow().create_media_track("dx".to_owned(), MediaKind::Audio).await.unwrap();
        println!("Has local media track: {:?}", track.borrow().kind);

        let offer = interface_a.clone().borrow().create_offer().await.unwrap();

        let answer = interface_b.clone().borrow().create_answer(offer).await.unwrap();
        interface_a.clone().borrow().set_answer(answer).await.unwrap();

        interface_a.borrow().wait_for_connection().await.unwrap();
        interface_b.borrow().wait_for_connection().await.unwrap();
        sleep(Duration::from_secs(2)).await;


        spawn_local(
            async move {
                for _ in 0..100 {
                    let binding = track.borrow();
                     let x = binding.track.as_any().downcast_ref::<TrackLocalStaticSample>().unwrap();
                    x.write_sample(&Sample {
                        duration: Duration::from_secs(1),
                        data: b"test".to_vec().into(),
                        ..Default::default()
                    }).await.unwrap();
                    sleep(Duration::from_millis(20)).await;
                }
            }
        );

        println!("Tracks A: {:?}", interface_a.borrow().provide_local_media_tracks().borrow().tracks.values().next().unwrap().borrow().kind);
        println!("Tracks B: {:?}", interface_b.borrow().provide_remote_media_tracks().borrow().tracks.values().next().unwrap().borrow().kind);
    }
}
