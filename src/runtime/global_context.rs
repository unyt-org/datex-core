use crate::{crypto::crypto::CryptoTrait, utils::time::TimeTrait};
use std::sync::{Arc, Mutex}; // FIXME no-std

#[cfg(feature = "debug")]
#[derive(Clone, Debug)]
pub struct DebugFlags {
    pub allow_unsigned_blocks: bool,
    pub enable_deterministic_behavior: bool,
}
#[cfg(feature = "debug")]
impl Default for DebugFlags {
    fn default() -> Self {
        DebugFlags {
            allow_unsigned_blocks: true,
            enable_deterministic_behavior: true
        }
    }
}

#[derive(Clone)]
pub struct GlobalContext {
    pub crypto: Arc<Mutex<dyn CryptoTrait>>,
    pub time: Arc<Mutex<dyn TimeTrait>>,

    #[cfg(feature = "debug")]
    pub debug_flags: DebugFlags,
}

impl GlobalContext {
    pub fn new(
        crypto: Arc<Mutex<dyn CryptoTrait>>,
        time: Arc<Mutex<dyn TimeTrait>>,
    ) -> GlobalContext {
        GlobalContext {
            crypto,
            time,
            #[cfg(feature = "debug")]
            debug_flags: DebugFlags::default(),
        }
    }
}

lazy_static::lazy_static! {
    static ref GLOBAL_CONTEXT: Mutex<Option<GlobalContext>> = Mutex::new(None);
}

pub fn set_global_context(c: GlobalContext) {
    let mut crypto = GLOBAL_CONTEXT.lock().unwrap();
    *crypto = Some(c);
}

pub fn get_global_context() -> GlobalContext {
    let context = GLOBAL_CONTEXT.lock().unwrap().clone();
    match context {
        Some(c) => c,
        None => panic!(
            "Global context not initialized - call set_global_context first!"
        ),
    }
}
