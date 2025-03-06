use std::{
  cell::{Ref, RefCell},
  rc::Rc,
  sync::{Arc, Mutex},
};

use crate::crypto::crypto::CryptoDefault;
use crate::{
  crypto::crypto::Crypto,
  datex_values::ValueResult,
  network::com_hub::ComHub,
  utils::logger::{Logger, LoggerContext},
};

mod execution;
pub mod global_context;
pub mod memory;
mod stack;

use self::{execution::execute, memory::Memory};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
pub struct Context {
  pub logger_context: Rc<RefCell<LoggerContext>>,
}

pub struct Runtime {
  pub version: String,
  pub context: Rc<RefCell<Context>>,
  pub memory: Rc<RefCell<Memory>>,
  pub com_hub: Rc<RefCell<ComHub>>,
  pub logger: Logger,
}

impl Runtime {
  pub fn new(context: Rc<RefCell<Context>>) -> Runtime {
    let logger = Logger::new_for_development(
      context.borrow().logger_context.clone(),
      "DATEX".to_string(),
    );
    logger.success("Runtime initialized!");
    return Runtime {
      version: VERSION.to_string(),
      context: context.clone(),
      logger,
      memory: Rc::new(RefCell::new(Memory::new())),
      com_hub: ComHub::new(context.clone()),
    };
  }

  pub fn default() -> Runtime {
    let context = Rc::new(RefCell::new(Context {
      logger_context: Rc::new(RefCell::new(LoggerContext {
        log_redirect: None,
      })),
    }));
    return Runtime::new(context);
  }

  pub fn execute(&self, dxb: &[u8]) -> ValueResult {
    execute(self.context.clone(), dxb)
  }
}
