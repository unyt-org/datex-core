use crate::crypto::crypto::Crypto;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct GlobalContext {
    pub crypto: Arc<Mutex<dyn Crypto>>,
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
