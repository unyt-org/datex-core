use datex_core::stdlib::sync::Arc;
use std::sync::{Mutex, Once}; // FIXME no-std

use datex_core::logger::init_logger;
use datex_core::{
    crypto::crypto_native::CryptoNative,
    runtime::global_context::{set_global_context, GlobalContext},
};

static INIT: Once = Once::new();

pub fn init_global_context() {
    let global_ctx = GlobalContext {
        crypto: Arc::new(Mutex::new(CryptoNative)),
    };

    set_global_context(global_ctx);

    INIT.call_once(|| {
        init_logger();
    });
}
