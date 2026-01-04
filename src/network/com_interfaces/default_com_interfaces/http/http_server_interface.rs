use axum::extract::Request;
use axum::routing::post;
use bytes::Bytes;
use core::cell::RefCell;

use crate::collections::HashMap;
use crate::std_sync::Mutex;
use crate::stdlib::net::SocketAddr;
use crate::stdlib::pin::Pin;
use crate::stdlib::rc::Rc;
use crate::stdlib::sync::Arc;
use crate::task::spawn;
use axum::response::Response;
use core::future::Future;
use core::time::Duration;
use futures::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use axum::{
    Router,
    extract::{Path, State},
    routing::get,
};
use datex_macros::{com_interface, create_opener};
use log::{debug, error, info};
use tokio::sync::{RwLock, broadcast, mpsc};
use url::Url;

use crate::network::com_interfaces::com_interface::{
    ComInterface, ComInterfaceState,
};
use crate::network::com_interfaces::com_interface::{
    ComInterfaceInfo, ComInterfaceSockets,
};
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface_socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};
use crate::network::com_interfaces::socket_provider::MultipleSocketProvider;
use crate::values::core_values::endpoint::Endpoint;
use crate::{delegate_com_interface_info, set_opener};

use super::http_common::HTTPError;

async fn server_to_client_handler(
    Path(route): Path<String>,
    State(state): State<HTTPServerState>,
) -> Response {
    let map = state.channels.read().await;
    if let Some((sender, _)) = map.get(&route) {
        let receiver = sender.subscribe();
        let stream = BroadcastStream::new(receiver);
        Response::builder()
            .header("Content-Type", "application/octet-stream")
            .header("Cache-Control", "no-cache")
            .body(axum::body::Body::from_stream(stream))
            .unwrap()
    } else {
        Response::builder()
            .status(404)
            .body("Channel not found".into())
            .unwrap()
    }
}
async fn client_to_server_handler(
    Path(route): Path<String>,
    State(state): State<HTTPServerState>,
    req: Request,
) -> Response {
    let map = state.channels.read().await;
    if let Some((_, sender)) = map.get(&route) {
        let mut stream = req.into_body().into_data_stream();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    debug!("Received junk {}", chunk.len());
                    sender
                        .send(chunk)
                        .await
                        .map_err(|_| HTTPError::SendError)
                        .unwrap();
                }
                Err(e) => {
                    error!("Error reading body {e}");
                    return Response::builder()
                        .status(400)
                        .body("Bad Request".into())
                        .unwrap();
                }
            }
        }
        Response::builder().status(200).body("OK".into()).unwrap()
    } else {
        Response::builder()
            .status(404)
            .body("Channel not found".into())
            .unwrap()
    }
}

pub struct HTTPServerNativeInterface {
    pub address: Url,
    info: ComInterfaceInfo,
    socket_channel_mapping:
        Rc<RefCell<HashMap<String, ComInterfaceSocketUUID>>>,
    channels: Arc<RwLock<HTTPChannelMap>>,
}

type HTTPChannelMap =
    HashMap<String, (broadcast::Sender<Bytes>, mpsc::Sender<Bytes>)>;

#[derive(Clone)]
struct HTTPServerState {
    channels: Arc<RwLock<HTTPChannelMap>>,
}

impl MultipleSocketProvider for HTTPServerNativeInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets().clone()
    }
}

#[com_interface]
impl HTTPServerNativeInterface {
    pub fn new(port: &u16) -> Result<HTTPServerNativeInterface, HTTPError> {
        let info = ComInterfaceInfo::new();
        let address: String = format!("http://127.0.0.1:{port}");
        let address =
            Url::parse(&address).map_err(|_| HTTPError::InvalidAddress)?;

        let interface = HTTPServerNativeInterface {
            channels: Arc::new(RwLock::new(HashMap::new())),
            address,
            socket_channel_mapping: Rc::new(RefCell::new(HashMap::new())),
            info,
        };
        Ok(interface)
    }

    pub async fn add_channel(&mut self, route: &str, endpoint: Endpoint) {
        let mut map = self.channels.write().await;
        if !map.contains_key(route) {
            let (server_tx, _) = broadcast::channel::<Bytes>(100);
            let (client_tx, mut rx) = mpsc::channel::<Bytes>(100); // FIXME #198 not braodcast needed
            map.insert(route.to_string(), (server_tx, client_tx));
            let socket = ComInterfaceSocket::new(
                self.get_uuid().clone(),
                InterfaceDirection::InOut,
                1,
            );
            let socket_uuid = socket.uuid.clone();
            let receive_queue = socket.receive_queue.clone();
            self.add_socket(Arc::new(Mutex::new(socket)));
            self.register_socket_endpoint(socket_uuid.clone(), endpoint, 1)
                .unwrap();
            self.socket_channel_mapping
                .borrow_mut()
                .insert(route.to_string(), socket_uuid.clone());

            spawn(async move {
                loop {
                    if let Some(data) = rx.recv().await {
                        debug!(
                            "Received data from socket {:?}: {}",
                            data.to_vec(),
                            socket_uuid
                        );
                        receive_queue.try_lock().unwrap().extend(data.to_vec());
                    }
                }
            });
        }
    }

    pub async fn remove_channel(&mut self, route: &str) {
        let mapping = self.socket_channel_mapping.clone();
        let socket_uuid = {
            let mapping = mapping.borrow();
            if let Some(socket_uuid) = mapping.get(route) {
                socket_uuid.clone()
            } else {
                return;
            }
        };
        self.remove_socket(&socket_uuid);
        let mut map = self.channels.write().await;
        if map.get(route).is_some() {
            map.remove(route);
        }
    }

    #[create_opener]
    async fn open(&mut self) -> Result<(), HTTPError> {
        let address = self.address.clone();
        info!("Spinning up server at {address}");

        let state = HTTPServerState {
            channels: self.channels.clone(),
        };
        let app = Router::new()
            .route("/{route}/rx", get(server_to_client_handler))
            .route("/{route}/tx", post(client_to_server_handler))
            .with_state(state.clone());

        let addr: SocketAddr = self
            .address
            .socket_addrs(|| None)
            .map_err(|_| HTTPError::InvalidAddress)?
            .first()
            .cloned()
            .ok_or(HTTPError::InvalidAddress)?;

        println!("HTTP server starting on http://{addr}");
        spawn(async move {
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
            interface_type: "http-server".to_string(),
            channel: "http".to_string(),
            round_trip_time: Duration::from_millis(20),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }
    fn handle_close<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        // TODO #199
        Box::pin(async move { true })
    }
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let route = self.socket_channel_mapping.borrow();
        let route = route.iter().find(|(_, v)| *v == &socket).map(|(k, _)| k);
        if route.is_none() {
            return Box::pin(async { false });
        }
        let route = route.unwrap().to_string();
        let channels = self.channels.clone();
        Box::pin(async move {
            let map = channels.read().await;
            if let Some((sender, _)) = map.get(&route) {
                let _ = sender.send(Bytes::copy_from_slice(block));
                true
            } else {
                false
            }
        })
    }

    delegate_com_interface_info!();
    set_opener!(open);
}
