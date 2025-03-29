use std::sync::Once;

use log::info;

use crate::logger::init_logger;
use crate::stdlib::{cell::RefCell, rc::Rc};

use crate::network::com_hub::ComHub;

mod execution;
pub mod global_context;
pub mod memory;
mod stack;

use self::memory::Memory;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
pub struct Context {}
static INIT: Once = Once::new();

pub struct Runtime {
    pub version: String,
    pub context: Rc<RefCell<Context>>,
    pub memory: Rc<RefCell<Memory>>,
    pub com_hub: Rc<RefCell<ComHub>>,
}

impl Runtime {
    pub fn new(context: Rc<RefCell<Context>>) -> Runtime {
        INIT.call_once(|| {
            init_logger();
        });
        info!("Runtime initialized!");
        Runtime {
            version: VERSION.to_string(),
            context: context.clone(),
            memory: Rc::new(RefCell::new(Memory::new())),
            com_hub: ComHub::new(context.clone()),
        }
    }

    pub fn default() -> Runtime {
        Runtime::new(Rc::new(RefCell::new(Context {})))
    }
}
