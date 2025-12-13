use crate::stdlib::boxed::Box;
use crate::stdlib::string::String;
use crate::stdlib::vec::Vec;
use crate::stdlib::{future::Future, pin::Pin};
use core::fmt::Display;
use core::prelude::rust_2024::*;
use core::result::Result;
pub type CryptoResult<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CryptoError>> + 'a>>;

pub trait CryptoTrait: Send + Sync {
    /// Creates a new UUID.
    fn create_uuid(&self) -> String;

    /// Generates cryptographically secure random bytes of the specified length.
    fn random_bytes(&self, length: usize) -> Vec<u8>;

    /// Returns representation of 32bytes (64 character string)
    fn hash_make_hex<'a>(
        &'a self,
        hash: &'a [u8; 32],
    ) -> Result<String, CryptoError>;
    /// Expexts a 64 character string hex representation of 32 bytes and returns them
    fn hash_from_hex<'a>(
        &'a self,
        fp: &'a str,
    ) -> Result<[u8; 32], CryptoError>;

    /// Sha256 hash
    fn hash_sha256<'a>(
        &'a self,
        to_digest: &'a [u8],
    ) -> CryptoResult<'a, [u8; 32]>;

    /// Hash key derivation function.
    fn hkdf_sha256<'a>(
        &'a self,
        ikm: &'a [u8],
        salt: &'a [u8],
    ) -> CryptoResult<'a, [u8; 32]>;

    /// Generates an Ed25519 key pair.
    fn gen_ed25519<'a>(&'a self) -> CryptoResult<'a, (Vec<u8>, Vec<u8>)>;

    /// Signs data with the given Ed25519 private key.
    fn sig_ed25519<'a>(
        &'a self,
        pri_key: &'a [u8],
        data: &'a [u8],
    ) -> CryptoResult<'a, [u8; 64]>;

    /// Verifies an Ed25519 signature with the given public key and data.
    fn ver_ed25519<'a>(
        &'a self,
        pub_key: &'a [u8],
        sig: &'a [u8],
        data: &'a [u8],
    ) -> CryptoResult<'a, bool>;

    /// AES-256 in CTR mode encryption, returns the ciphertext.
    fn aes_ctr_encrypt<'a>(
        &'a self,
        key: &'a [u8; 32],
        iv: &'a [u8; 16],
        plaintext: &'a [u8],
    ) -> CryptoResult<'a, Vec<u8>>;

    /// AES-256 in CTR mode decryption, returns the plaintext.
    fn aes_ctr_decrypt<'a>(
        &'a self,
        key: &'a [u8; 32],
        iv: &'a [u8; 16],
        cipher: &'a [u8],
    ) -> CryptoResult<'a, Vec<u8>>;

    /// AES Key Wrap (RFC 3394), returns the wrapped key (ciphertext).
    fn key_upwrap<'a>(
        &'a self,
        kek_bytes: &'a [u8; 32],
        rb: &'a [u8; 32],
    ) -> CryptoResult<'a, [u8; 40]>;

    /// AES Key Unwrap (RFC 3394), returns the unwrapped key (plaintext).
    fn key_unwrap<'a>(
        &'a self,
        kek_bytes: &'a [u8; 32],
        cipher: &'a [u8; 40],
    ) -> CryptoResult<'a, [u8; 32]>;

    /// Generates an X25519 key pair, returns (public_key, private_key).
    fn gen_x25519<'a>(&'a self) -> CryptoResult<'a, ([u8; 44], [u8; 48])>;

    /// Derives a shared secret using X25519 given my private key and the peer's public key.
    fn derive_x25519<'a>(
        &'a self,
        pri_key: &'a [u8; 48],
        peer_pub: &'a [u8; 44],
    ) -> CryptoResult<'a, Vec<u8>>;
}

pub struct Crypto;

#[derive(Debug, Clone)]
pub enum CryptoError {
    Other(String),
    KeyGeneration,
    KeyExport,
    KeyImport,
    Encryption,
    Decryption,
    Signing,
    Verification,
}

impl Display for CryptoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CryptoError::Other(msg) => core::write!(f, "Crypto: {}", msg),
            CryptoError::KeyGeneration => {
                core::write!(f, "CryptoError: Key generation failed")
            }
            CryptoError::KeyExport => {
                core::write!(f, "CryptoError: Key export failed")
            }
            CryptoError::KeyImport => {
                core::write!(f, "CryptoError: Key import failed")
            }
            CryptoError::Encryption => {
                core::write!(f, "CryptoError: Encryption failed")
            }
            CryptoError::Decryption => {
                core::write!(f, "CryptoError: Decryption failed")
            }
            CryptoError::Signing => {
                core::write!(f, "CryptoError: Signing failed")
            }
            CryptoError::Verification => {
                core::write!(f, "CryptoError: Verification failed")
            }
        }
    }
}
