use core::prelude::rust_2024::*;
use crate::{crypto::crypto::CryptoTrait, utils::time::TimeTrait};
use crate::stdlib::{sync::Arc};
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
            enable_deterministic_behavior: true,
        }
    }
}

#[derive(Clone)]
pub struct GlobalContext {
    pub crypto: Arc<dyn CryptoTrait>,
    pub time: Arc<dyn TimeTrait>,

    #[cfg(feature = "debug")]
    pub debug_flags: DebugFlags,
}

impl GlobalContext {
    pub fn new(
        crypto: Arc<dyn CryptoTrait>,
        time: Arc<dyn TimeTrait>,
    ) -> GlobalContext {
        GlobalContext {
            crypto,
            time,
            #[cfg(feature = "debug")]
            debug_flags: DebugFlags::default(),
        }
    }

    #[cfg(all(feature = "native_crypto", feature = "std", feature = "native_time"))]
    pub fn native() -> GlobalContext {
        use crate::{
            crypto::crypto_native::CryptoNative, utils::time_native::TimeNative,
        };
        GlobalContext {
            crypto: Arc::new(CryptoNative),
            time: Arc::new(TimeNative),
            #[cfg(feature = "debug")]
            debug_flags: DebugFlags::default(),
        }
    }
}

#[cfg_attr(not(feature = "embassy_runtime"), thread_local)]
pub static mut GLOBAL_CONTEXT: Option<GlobalContext> = None;

pub fn set_global_context(c: GlobalContext) {
    unsafe {
        GLOBAL_CONTEXT.replace(c);
    }
}
pub(crate) fn get_global_context() -> GlobalContext {
    unsafe {
        GLOBAL_CONTEXT.clone().expect("Global context not initialized - call set_global_context first!")
    }
}
