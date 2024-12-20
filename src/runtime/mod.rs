use std::{cell::RefCell, rc::Rc};

use crate::{utils::{logger::{LoggerContext, Logger}, crypto::Crypto, rust_crypto::RustCrypto}, datex_values::ValueResult};

mod stack;
mod execution;
pub mod memory;

use self::{execution::execute, memory::Memory};

const VERSION: &str = env!("CARGO_PKG_VERSION");


pub struct Runtime<'a> {
    pub version: String,
	pub ctx: &'a LoggerContext,
	pub crypto: &'a dyn Crypto,
	pub memory: Rc<RefCell<Memory>>
}

impl Runtime<'_> {
	
	pub fn new_with_crypto_and_logger<'a>(crypto: &'a dyn Crypto, ctx: &'a LoggerContext) -> Runtime<'a> {
		let logger = Logger::new_for_development(&ctx, "DATEX");
    	logger.success("initialized!");
		return Runtime { 
			version: VERSION.to_string(),
			crypto, 
			ctx, 
			memory: Rc::new(RefCell::new(Memory::new()))
		}
	}

	pub fn new() -> Runtime<'static> {
		return Runtime { 
			version: VERSION.to_string(), 
			crypto: &RustCrypto{},
			ctx: &LoggerContext { log_redirect: None},
			memory: Rc::new(RefCell::new(Memory::new()))
		}
	}

	pub fn execute(&self, dxb: &[u8]) -> ValueResult {
		execute(&self.ctx, dxb)
	}

}

