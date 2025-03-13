use std::{future::Future, pin::Pin, usize};

pub trait Crypto: Send + Sync {
    fn encrypt(&self, data: &[u8]) -> Vec<u8>;
    fn decrypt(&self, data: &[u8]) -> Vec<u8>;
    fn create_uuid(&self) -> String;
    fn random_bytes(&self, length: usize) -> Vec<u8>;

    fn new_encryption_key_pair(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<u8>, Vec<u8>), CryptoError>>>>;
    fn new_sign_key_pair(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<u8>, Vec<u8>), CryptoError>>>>;
}

#[derive(Debug, Clone)]
pub enum CryptoError {
    Other(String),
    KeyGeneratorFailed,
    KeyExportFailed,
    KeyImportFailed,
}

pub struct CryptoDefault;
impl Crypto for CryptoDefault {
    fn encrypt(&self, data: &[u8]) -> Vec<u8> {
        unreachable!()
    }

    fn decrypt(&self, data: &[u8]) -> Vec<u8> {
        unreachable!()
    }

    fn create_uuid(&self) -> String {
        unreachable!()
    }

    fn random_bytes(&self, length: usize) -> Vec<u8> {
        unreachable!()
    }

    fn new_encryption_key_pair(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<u8>, Vec<u8>), CryptoError>>>>
    {
        unreachable!()
    }

    fn new_sign_key_pair(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<u8>, Vec<u8>), CryptoError>>>>
    {
        unreachable!()
    }
}
