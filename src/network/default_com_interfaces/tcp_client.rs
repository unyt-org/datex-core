use crate::network::com_interface::ComInterface;

pub struct TCPClientInterface {}

impl ComInterface for TCPClientInterface {
    const NAME: &'static str = "tcp_client";
}