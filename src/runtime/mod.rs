use std::{cell::RefCell, rc::Rc};

use crate::{
    datex_values::ValueResult,
    network::com_hub::ComHub,
    utils::{
        crypto::Crypto,
        logger::{Logger, LoggerContext},
        rust_crypto::RustCrypto,
    },
};

mod execution;
pub mod memory;
mod stack;

use self::{execution::execute, memory::Memory};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Runtime<'a> {
    pub version: String,
    pub ctx: &'a LoggerContext,
    pub crypto: &'a dyn Crypto,
    pub memory: Rc<RefCell<Memory>>,
    pub com_hub: Rc<RefCell<ComHub>>,
}

impl Runtime<'_> {
    pub fn new_with_crypto_and_logger<'a>(
        crypto: &'a dyn Crypto,
        ctx: &'a LoggerContext,
    ) -> Runtime<'a> {
        let logger = Logger::new_for_development(&ctx, "DATEX");
        logger.success("initialized!");
        return Runtime {
            version: VERSION.to_string(),
            crypto,
            ctx,
            memory: Rc::new(RefCell::new(Memory::new())),
            com_hub: ComHub::new(),
        };
    }

    pub fn new() -> Runtime<'static> {
        return Runtime::new_with_crypto_and_logger(
            &RustCrypto {},
            &LoggerContext { log_redirect: None },
        );
    }

    pub fn execute(&self, dxb: &[u8]) -> ValueResult {
        execute(&self.ctx, dxb)
    }
}
