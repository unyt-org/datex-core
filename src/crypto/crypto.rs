use crate::stdlib::{future::Future, pin::Pin, usize};

pub trait CryptoTrait: Send + Sync {
    fn create_uuid(&self) -> String;
    fn random_bytes(&self, length: usize) -> Vec<u8>;

    fn gen_ed25519(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<u8>, Vec<u8>), CryptoError>> + 'static>>;

    fn sig_ed25519<'a>(
        &'a self,
        pri_key: &'a Vec<u8>,
        data: &'a Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>> + 'a>>;

    fn ver_ed25519<'a>(
        &'a self,
        pub_key: &'a Vec<u8>,
        sig: &'a Vec<u8>,
        data: &'a Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, CryptoError>> + 'a>>;

    fn aes_ctr_encrypt<'a>(
        &'a self,
        key: &'a [u8; 32],
        iv: &'a [u8; 16],
        plaintext: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>> + 'a>>;
}

pub struct Crypto;

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
