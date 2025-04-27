use std::sync::{Arc, Mutex};

#[cfg(feature = "native_crypto")]
use crate::crypto::crypto_native::CryptoNative;
use crate::datex_values::Endpoint;
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

pub struct Runtime {
    pub version: String,
    pub memory: Rc<RefCell<Memory>>,
    pub com_hub: Arc<Mutex<ComHub>>,
    pub endpoint: Endpoint,
}

impl Runtime {
    pub fn new(endpoint: impl Into<Endpoint>) -> Runtime {
        let endpoint = endpoint.into();
        let com_hub = ComHub::new(endpoint.clone());
        Runtime {
            endpoint,
            com_hub: Arc::new(Mutex::new(com_hub)),
            ..Runtime::default()
        }
    }
    pub fn init(endpoint: impl Into<Endpoint>, global_context: GlobalContext) -> Runtime {
        set_global_context(global_context);
        init_logger();
        info!(
            "Runtime initialized - Version {VERSION} Time: {}",
            get_global_context().time.lock().unwrap().now()
        );
        Self::new(endpoint)
    }

    #[cfg(feature = "native_crypto")]
    pub fn init_native(endpoint: impl Into<Endpoint>) -> Runtime {
        use crate::utils::time_native::TimeNative;

        Self::init(
            endpoint,
            GlobalContext::new(
                Arc::new(Mutex::new(CryptoNative)),
                Arc::new(Mutex::new(TimeNative)),
            ),
        )
    }

    /// Starts the common update loop:
    ///  - ComHub
    pub async fn start(&self) {
        info!("starting runtime...");
        let com_hub = self.com_hub.clone();
        com_hub
            .lock()
            .unwrap()
            .init()
            .await
            .expect("Failed to initialize ComHub");
        ComHub::start_update_loop(com_hub.clone());
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Runtime {
            endpoint: Endpoint::default(),
            version: VERSION.to_string(),
            memory: Rc::new(RefCell::new(Memory::new())),
            com_hub: Arc::new(Mutex::new(ComHub::default())),
        }
    }
}
