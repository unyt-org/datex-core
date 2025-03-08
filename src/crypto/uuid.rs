use super::crypto::Crypto;
use crate::runtime::global_context::get_global_context;
use std::{fmt::Display, marker::PhantomData};

#[derive(Debug, Clone, PartialEq)]
pub struct UUID<T> {
    uuid: String,
    _phantom: PhantomData<T>,
}

impl<T> UUID<T> {
    pub fn new() -> UUID<T> {
        let crypto = get_global_context().crypto;
        let uuid = crypto.lock().unwrap().create_uuid();
        UUID {
            uuid,
            _phantom: PhantomData,
        }
    }
    pub fn to_string(&self) -> String {
        self.uuid.clone()
    }
}

impl<T> Default for UUID<T> {
    fn default() -> Self {
        UUID {
            uuid: "default".to_string(),
            _phantom: PhantomData,
        }
    }
}

impl<T> Display for UUID<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.uuid)
    }
}
