use std::sync::{Arc, Mutex};

use datex_core::{
    crypto::crypto_native::CryptoNative,
    runtime::global_context::{set_global_context, GlobalContext},
};

pub fn init_global_context() {
    let global_ctx = GlobalContext {
        crypto: Arc::new(Mutex::new(CryptoNative)),
    };

    set_global_context(global_ctx);
}
