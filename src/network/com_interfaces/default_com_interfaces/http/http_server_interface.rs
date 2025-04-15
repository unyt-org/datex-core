use axum::body::Body;
use axum::Extension;
use bytes::Bytes;

use axum::response::IntoResponse;
use futures::Stream;
use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use axum::{
    body::BodyDataStream,
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use log::{error, info, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::spawn;
use tokio::sync::{broadcast, mpsc};
use url::Url;

use crate::delegate_com_interface_info;
use crate::network::com_interfaces::com_interface::{
    ComInterface, ComInterfaceState,
};
use crate::network::com_interfaces::com_interface::{
    ComInterfaceInfo, ComInterfaceSockets, ComInterfaceUUID,
};
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface_socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};

use super::http_common::HTTPError;

pub struct HTTPServerNativeInterface {
    pub address: Url,
    info: ComInterfaceInfo,
}

type SharedClients = Arc<
    Mutex<HashMap<ComInterfaceSocketUUID, Arc<Mutex<mpsc::Sender<Bytes>>>>>,
>;

// async fn connect_handler(
//     Path(id): Path<String>,
//     State(state): State<HTTPServerState>,
// ) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
//     let uuid = ComInterfaceSocketUUID::from(id); // Your conversion
//     let (tx, rx) = mpsc::channel(100);
//     state
//         .clients
//         .lock()
//         .unwrap()
//         .insert(uuid.clone(), Arc::new(Mutex::new(tx)));

//     let mut global_rx = state.global_tx.subscribe();

//     let client_stream = ReceiverStream::new(rx)
//         .map(Ok::<_, Infallible>)
//         .map(|msg| msg.map(|data| Event::default().data(data)));

//     let global_stream =
//         BroadcastStream::new(global_rx).filter_map(|msg| async {
//             msg.ok().map(|data| Ok(Event::default().data(data)))
//         });

//     Sse::new(client_stream.merge(global_stream))
//         .keep_alive(KeepAlive::default())
// }

// async fn send_to_client_handler(
//     Path(id): Path<String>,
//     State(state): State<HTTPServerState>,
//     Json(msg): Json<SendMessage>,
// ) -> &'static str {
//     let uuid = ComInterfaceSocketUUID::from(id); // Your conversion
//     let map = state.clients.lock().unwrap();
//     if let Some(sender) = map.get(&uuid) {
//         let _ = sender.lock().unwrap().send(msg.message.clone()).await;
//         "sent"
//     } else {
//         "client not found"
//     }
// }

#[derive(Clone)]
struct HTTPServerState {
    clients: SharedClients,
    interface_uuid: ComInterfaceUUID,
    global_tx: broadcast::Sender<Bytes>,
}
async fn rx_handler(
    Path(id): Path<String>,
    Extension(state): Extension<Arc<HTTPServerState>>,
) -> impl IntoResponse {
    let socket = ComInterfaceSocket::new(
        state.interface_uuid,
        InterfaceDirection::IN_OUT,
        1,
    );
    let uuid = socket.uuid.clone();
    let (tx, rx) = mpsc::channel::<Bytes>(100);

    state
        .clients
        .lock()
        .unwrap()
        .insert(uuid.clone(), Arc::new(Mutex::new(tx)));

    // Start streaming
    tokio_stream::wrappers::ReceiverStream::new(rx)
        // .keep_alive(KeepAlive::default())
        .map(|data| Event::default().data(data))

    // let stream = tokio_stream::wrappers::ReceiverStream::new(rx);

    // Body::from_stream(stream)
    // .map_err(|_| Infallible)
    // .map(|bytes| {
    //     let data = bytes.to_vec();
    //     Event::default().data(data)
    // })
    // .into_response();
    // Return the stream as the response
    // stream
}

impl HTTPServerNativeInterface {
    pub async fn open(
        port: &u16,
    ) -> Result<HTTPServerNativeInterface, HTTPError> {
        let info = ComInterfaceInfo::new();
        let address: String = format!("http://127.0.0.1:{}", port);
        let address =
            Url::parse(&address).map_err(|_| HTTPError::InvalidAddress)?;

        let mut interface = HTTPServerNativeInterface { address, info };
        interface.start().await?;
        Ok(interface)
    }

    async fn start(&mut self) -> Result<(), HTTPError> {
        let address = self.address.clone();
        info!("Spinning up server at {}", address);

        let (global_tx, _) = broadcast::channel(100);
        let state = HTTPServerState {
            global_tx,
            clients: Arc::new(Mutex::new(HashMap::new())),
            interface_uuid: self.get_uuid().clone(),
        };
        // let interface_uuid = self.get_uuid().clone();
        let app = Router::new()
            .route("/:id/rx", get(rx_handler))
            // .route("/send/:id", post(send_to_client_handler))
            // .route("/broadcast", post(broadcast_handler))
            .with_state(state);

        let addr: SocketAddr = self
            .address
            .socket_addrs(|| None)
            .map_err(|_| HTTPError::InvalidAddress)?
            .first()
            .cloned()
            .ok_or(HTTPError::InvalidAddress)?;

        println!("HTTP server starting on http://{}", addr);
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
            axum::serve(listener, app.into_make_service())
                .await
                .unwrap();
        });

        Ok(())
    }
}

impl ComInterface for HTTPServerNativeInterface {
    fn init_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "http".to_string(),
            round_trip_time: Duration::from_millis(20),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }
    fn close<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        // TODO
        Box::pin(async move { true })
    }
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        // let tx_queues = self.tx.clone();
        // let tx_queues = tx_queues.lock().unwrap();
        // let tx = tx_queues.get(&socket);
        // if tx.is_none() {
        //     error!("Client is not connected");
        //     return Box::pin(async { false });
        // }
        // let tx = tx.unwrap().clone();
        Box::pin(async move { true })
    }

    delegate_com_interface_info!();
}
