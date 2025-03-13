use std::{future::Future, pin::Pin, usize};

pub trait Crypto: Send + Sync {
    fn encrypt_rsa(
        &self,
        data: &[u8],
        public_key: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>>>>;
    fn decrypt_rsa(
        &self,
        data: &[u8],
        private_key: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>>>>;
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
    EncryptionError,
    DecryptionError,
}

pub struct CryptoDefault;
impl Crypto for CryptoDefault {
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

    fn encrypt_rsa(
        &self,
        data: &[u8],
        public_key: Vec<u8>,
    ) -> Pin<
        Box<
            (dyn std::future::Future<Output = Result<Vec<u8>, CryptoError>>
                 + 'static),
        >,
    > {
        unreachable!()
    }

    fn decrypt_rsa(
        &self,
        data: &[u8],
        private_key: Vec<u8>,
    ) -> Pin<
        Box<
            (dyn std::future::Future<Output = Result<Vec<u8>, CryptoError>>
                 + 'static),
        >,
    > {
        unreachable!()
    }
}
