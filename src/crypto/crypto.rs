use crate::stdlib::{future::Future, pin::Pin, usize};
use crate::crypto::crypto_native::{Crypt, KEY_LEN, SIG_LEN};

pub trait CryptoTrait: Send + Sync {
    // Deprecated
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

    // Replacement
    // EdDSA
    fn gen_ed25519(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<([u8; KEY_LEN], [u8; KEY_LEN]), CryptoError>> + 'static>>;

    fn sig_ed25519<'a>(
        &'a self,
        pri_key: &'a [u8; KEY_LEN],
        digest: &'a Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>> + Send + 'a>>;

    fn ver_ed25519<'a>(
        &'a self,
        pub_key: &'a [u8; KEY_LEN],
        sig: &'a [u8; SIG_LEN],
        data: &'a Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, CryptoError>> + Send + 'a>>;

    // Elliptic curve generation
    fn gen_x25519(&self) -> Result<([u8; KEY_LEN], [u8; KEY_LEN]), CryptoError>;

    // Asymmetric encryption
    // Elliptic curve integrated encryption scheme
    fn ecies_encrypt<'a>(
        &'a self,
        rec_pub_raw: &'a [u8; KEY_LEN],
        plaintext: &'a [u8],
        aad: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<Crypt, CryptoError>> + Send + 'a>>;

    fn ecies_decrypt<'a>(
        &'a self,
        rec_pri_raw: &'a [u8; KEY_LEN],
        msg: &'a Crypt,
        aad: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>> + Send + 'a>>;
}

pub struct Crypto;

#[derive(Debug, Clone)]
pub enum CryptoError {
    Other(String),
    KeyGeneratorFailed,
    KeyExportFailed,
    KeyImportFailed,
    KeyDerivationFailed,
    EncryptionError,
    DecryptionError,
    SigningError,
    VerificationError,
}
