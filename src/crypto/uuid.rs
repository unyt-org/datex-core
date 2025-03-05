use std::{fmt::Display, marker::PhantomData};

use super::crypto::Crypto;

#[derive(Debug, Clone, PartialEq)]
pub struct UUID<T> {
  uuid: String,
  _phantom: PhantomData<T>,
}

impl<T> UUID<T> {
  pub fn new(crypto: &dyn Crypto) -> UUID<T> {
    UUID {
      uuid: crypto.create_uuid(),
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
