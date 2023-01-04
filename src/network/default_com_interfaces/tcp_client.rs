use crate::network::com_interface::ComInterface;

pub struct TCPClientInterface {}

impl ComInterface for TCPClientInterface {
    const NAME: &'static str = "tcp_client";
    const IN: bool = true;
    const OUT: bool = true;
	const GLOBAL: bool = true;
	const VIRTUAL: bool = false;

	fn send_block(&mut self, block: &[u8]) -> () {
        todo!()
    }
	
}