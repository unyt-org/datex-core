use crate::stdlib::{future::Future, pin::Pin, usize};

pub trait CryptoTrait: Send + Sync {
    fn encrypt_rsa(
        &self,
        data: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>>>>;

    fn decrypt_rsa(
        &self,
        data: Vec<u8>,
        private_key: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>>>>;

    fn sign_rsa(
        &self,
        data: Vec<u8>,
        private_key: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>>>>;

    fn verify_rsa(
        &self,
        data: Vec<u8>,
        signature: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, CryptoError>>>>;

    fn create_uuid(&self) -> String;
    fn random_bytes(&self, length: usize) -> Vec<u8>;

    fn new_encryption_key_pair(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<u8>, Vec<u8>), CryptoError>>>>;
    fn new_sign_key_pair(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<u8>, Vec<u8>), CryptoError>>>>;
}

pub struct Crypto;
impl Crypto {
    // TODO add other method if needed here as wrapper

    // pub fn create_uuid(&self) -> String {
    //     let crypto: std::sync::Arc<std::sync::Mutex<dyn CryptoTrait>> =
    //         get_global_context().crypto;
    //     let crypto = crypto.lock().unwrap();
    //     crypto.create_uuid()
    // }
}

#[derive(Debug, Clone)]
pub enum CryptoError {
    Other(String),
    KeyGeneratorFailed,
    KeyExportFailed,
    KeyImportFailed,
    EncryptionError,
    DecryptionError,
    SigningError,
    VerificationError,
}
