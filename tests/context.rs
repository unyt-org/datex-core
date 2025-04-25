use datex_core::stdlib::sync::Arc;
use datex_core::utils::time_native::TimeNative;
use std::sync::{Mutex, Once}; // FIXME no-std

use datex_core::logger::init_logger;
use datex_core::{
    crypto::crypto_native::CryptoNative,
    runtime::global_context::{set_global_context, GlobalContext},
};

pub fn init_global_context() {
    let global_ctx = GlobalContext {
        crypto: Arc::new(Mutex::new(CryptoNative)),
        time: Arc::new(Mutex::new(TimeNative)),
    };

    set_global_context(global_ctx);
    init_logger();
}
