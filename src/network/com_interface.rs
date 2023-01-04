pub trait ComInterface {

	const NAME: &'static str;
	const IN: bool;  // can receive data
	const OUT: bool; // can send data
	const GLOBAL: bool; // has a connection to the global network, might be preferred as a default interface
	const VIRTUAL: bool; // only a relayed connection, don't use for DATEX rooting

	fn send_block(&mut self, block: &[u8]) -> ();


}