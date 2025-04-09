use std::sync::{Arc, Mutex, Once};

#[cfg(feature = "native_crypto")]
use crate::crypto::crypto_native::CryptoNative;
use crate::logger::init_logger;
use crate::stdlib::{cell::RefCell, rc::Rc};
use global_context::{get_global_context, set_global_context, GlobalContext};
use log::info;

use crate::network::com_hub::ComHub;

mod execution;
pub mod global_context;
pub mod memory;
mod stack;

use self::memory::Memory;

const VERSION: &str = env!("CARGO_PKG_VERSION");

static INIT: Once = Once::new();

pub struct Runtime {
    pub version: String,
    pub memory: Rc<RefCell<Memory>>,
    pub com_hub: Rc<RefCell<ComHub>>,
}

impl Runtime {
    pub fn new() -> Runtime {
        Runtime::default()
    }
    pub fn init(global_context: GlobalContext) -> Runtime {
        set_global_context(global_context);
        INIT.call_once(|| {
            init_logger();

            info!(
                "Runtime initialized - Version {VERSION} Time: {}",
                get_global_context().time.lock().unwrap().now()
            );
        });
        Self::new()
    }

    #[cfg(feature = "native_crypto")]
    pub fn init_native() -> Runtime {
        use crate::utils::time_native::TimeNative;

        Self::init(GlobalContext {
            crypto: Arc::new(Mutex::new(CryptoNative)),
            time: Arc::new(Mutex::new(TimeNative)),
        })
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Runtime {
            version: VERSION.to_string(),
            memory: Rc::new(RefCell::new(Memory::new())),
            com_hub: ComHub::new(),
        }
    }
}
