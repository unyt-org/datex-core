use crate::utils::{logger::LoggerContext, crypto::Crypto, rust_crypto::RustCrypto};

mod stack;
use self::execution::execute;

mod execution;


pub struct Runtime<'a> {
    pub version: i8,
	pub logger: LoggerContext,
	pub crypto: &'a dyn Crypto
}

impl Runtime<'_> {
	
	pub fn new_with_crypto_and_logger(crypto: &dyn Crypto, ctx: LoggerContext) -> Runtime {
		return Runtime { version: 1, crypto, logger: ctx }
	}

	pub fn new() -> Runtime<'static> {
		return Runtime { version: 1, crypto: &RustCrypto{}, logger: LoggerContext { log_redirect: None } }
	}

	pub fn execute(&self, dxb: &[u8]) {
		execute(&self.logger, dxb);
	}

}

