use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::info;
use tokio::net::TcpStream;
use url::Url;

use crate::network::com_interfaces::com_interface::{
    ComInterfaceSockets, ComInterfaceUUID,
};
use crate::network::com_interfaces::com_interface_properties::InterfaceProperties;
use crate::network::com_interfaces::com_interface_socket::ComInterfaceSocketUUID;
use crate::network::com_interfaces::tcp::tcp_common::TCPError;
use crate::utils::uuid::UUID;

use super::super::com_interface::ComInterface;

pub struct TCPClientNativeInterface {
    pub address: Url,
    pub uuid: ComInterfaceUUID,
    pub stream: Option<TcpStream>,
}

impl TCPClientNativeInterface {
    pub async fn open(
        address: &str,
    ) -> Result<TCPClientNativeInterface, TCPError> {
        let uuid = ComInterfaceUUID(UUID::new());
        let mut interface = TCPClientNativeInterface {
            address: Url::parse(address).map_err(|_| TCPError::InvalidURL)?,
            uuid,
            stream: None,
        };
        interface.start().await?;

        Ok(interface)
    }

    async fn start(&mut self) -> Result<(), TCPError> {
        let address = self.address.clone();
        info!(
            "Connecting to WebSocket server at {}",
            address.host_str().unwrap()
        );
        Ok(())
    }
}

impl ComInterface for TCPClientNativeInterface {
    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "tcp".to_string(),
            round_trip_time: Duration::from_millis(20),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        todo!()
    }

    fn get_uuid(&self) -> &ComInterfaceUUID {
        todo!()
    }

    fn get_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        todo!()
    }
}
