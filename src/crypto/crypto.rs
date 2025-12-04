use crate::stdlib::boxed::Box;
use crate::stdlib::string::String;
use crate::stdlib::vec::Vec;
use core::fmt::Display;
use core::prelude::rust_2024::*;
use core::result::Result;

use crate::stdlib::{future::Future, pin::Pin};

pub enum MaybeAsync<'a, T> {
    Syn(Result<T, CryptoError>),
    Asy(Pin<Box<dyn Future<Output = Result<T, CryptoError>> + 'a>>),
}
impl<'a, T> MaybeAsync<'a, T>
where
    T: Send + 'a,
{
    pub fn asy_resolve(
        self,
    ) -> Pin<Box<dyn Future<Output = Result<T, CryptoError>> + 'a>> {
        match self {
            MaybeAsync::Syn(res) => Box::pin(async move { res }),
            MaybeAsync::Asy(fut) => fut,
        }
    }
    pub fn syn_resolve(self) -> Result<T, CryptoError> {
        match self {
            MaybeAsync::Syn(res) => res,
            MaybeAsync::Asy(_) => Err(CryptoError::AsyncError),
        }
    }
}

pub trait CryptoTrait: Send + Sync {
    /// Creates a new UUID.
    fn create_uuid(&self) -> String;

    /// Generates cryptographically secure random bytes of the specified length.
    fn random_bytes(&self, length: usize) -> Vec<u8>;

    /// Sha256 hash
    fn hash<'a>(
        &'a self,
        to_digest: &'a [u8],
    ) -> Result<MaybeAsync<'a, [u8; 32]>, CryptoError>;

    /// Hash key derivation function.
    fn hkdf<'a>(
        &'a self,
        ikm: &'a [u8],
        salt: &'a [u8],
    ) -> Result<MaybeAsync<'a, [u8; 32]>, CryptoError>;

    /// Generates an Ed25519 key pair.
    fn gen_ed25519<'a>(
        &'a self,
    ) -> Result<MaybeAsync<'a, (Vec<u8>, Vec<u8>)>, CryptoError>;

    /// Signs data with the given Ed25519 private key.
    fn sig_ed25519<'a>(
        &'a self,
        pri_key: &'a [u8],
        data: &'a [u8],
    ) -> Result<MaybeAsync<'a, [u8; 64]>, CryptoError>;

    /// Verifies an Ed25519 signature with the given public key and data.
    fn ver_ed25519<'a>(
        &'a self,
        pub_key: &'a [u8],
        sig: &'a [u8],
        data: &'a [u8],
    ) -> Result<MaybeAsync<'a, bool>, CryptoError>;

    /// AES-256 in CTR mode encryption, returns the ciphertext.
    fn aes_ctr_encrypt<'a>(
        &'a self,
        key: &'a [u8; 32],
        iv: &'a [u8; 16],
        plaintext: &'a [u8],
    ) -> Result<MaybeAsync<'a, Vec<u8>>, CryptoError>;

    /// AES-256 in CTR mode decryption, returns the plaintext.
    fn aes_ctr_decrypt<'a>(
        &'a self,
        key: &'a [u8; 32],
        iv: &'a [u8; 16],
        cipher: &'a [u8],
    ) -> Result<MaybeAsync<'a, Vec<u8>>, CryptoError>;

    /// AES Key Wrap (RFC 3394), returns the wrapped key (ciphertext).
    fn key_upwrap<'a>(
        &'a self,
        kek_bytes: &'a [u8; 32],
        rb: &'a [u8; 32],
    ) -> Result<MaybeAsync<'a, [u8; 40]>, CryptoError>;

    /// AES Key Unwrap (RFC 3394), returns the unwrapped key (plaintext).
    fn key_unwrap<'a>(
        &'a self,
        kek_bytes: &'a [u8; 32],
        cipher: &'a [u8; 40],
    ) -> Result<MaybeAsync<'a, [u8; 32]>, CryptoError>;

    /// Generates an X25519 key pair, returns (public_key, private_key).
    fn gen_x25519<'a>(
        &'a self,
    ) -> Result<MaybeAsync<'a, ([u8; 44], [u8; 48])>, CryptoError>;

    /// Derives a shared secret using X25519 given my private key and the peer's public key.
    fn derive_x25519<'a>(
        &'a self,
        pri_key: &'a [u8; 48],
        peer_pub: &'a [u8; 44],
    ) -> Result<MaybeAsync<'a, Vec<u8>>, CryptoError>;
}

pub struct Crypto;

#[derive(Debug, Clone)]
pub enum CryptoError {
    Other(String),
    KeyGenerationError,
    KeyExportError,
    KeyImportError,
    EncryptionError,
    DecryptionError,
    SigningError,
    VerificationError,
    AsyncError,
}

impl Display for CryptoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CryptoError::Other(msg) => core::write!(f, "CryptoError: {}", msg),
            CryptoError::KeyGenerationError => {
                core::write!(f, "CryptoError: Key generation failed")
            }
            CryptoError::KeyExportError => {
                core::write!(f, "CryptoError: Key export failed")
            }
            CryptoError::KeyImportError => {
                core::write!(f, "CryptoError: Key import failed")
            }
            CryptoError::EncryptionError => {
                core::write!(f, "CryptoError: Encryption failed")
            }
            CryptoError::DecryptionError => {
                core::write!(f, "CryptoError: Decryption failed")
            }
            CryptoError::SigningError => {
                core::write!(f, "CryptoError: Signing failed")
            }
            CryptoError::VerificationError => {
                core::write!(f, "CryptoError: Verification failed")
            }
            CryptoError::AsyncError => {
                core::write!(f, "CryptoError: Async code execution failed")
            }
        }
    }
}
