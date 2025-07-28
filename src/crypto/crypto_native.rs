use crate::stdlib::{future::Future, pin::Pin, usize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use super::crypto::{CryptoError, CryptoTrait};
use crate::runtime::global_context::get_global_context;
use rand::{rngs::OsRng, Rng};
use rsa::{
    pkcs8::{EncodePrivateKey, EncodePublicKey},
    RsaPrivateKey, RsaPublicKey,
};
use uuid::Uuid;

static UUID_COUNTER: OnceLock<AtomicU64> = OnceLock::new();

fn init_counter() -> &'static AtomicU64 {
    UUID_COUNTER.get_or_init(|| AtomicU64::new(1))
}
fn generate_pseudo_uuid() -> String {
    let counter = init_counter();
    let count = counter.fetch_add(1, Ordering::Relaxed);

    // Encode counter into last segment, keeping UUID-like structure
    format!("00000000-0000-0000-0000-{count:012x}")
}

#[derive(Debug, Clone, PartialEq)]
pub struct CryptoNative;
impl CryptoTrait for CryptoNative {
    fn encrypt_rsa(
        &self,
        data: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Pin<Box<(dyn Future<Output = Result<Vec<u8>, CryptoError>> + 'static)>>
    {
        todo!("#164 Undescribed by author.")
    }

    fn decrypt_rsa(
        &self,
        data: Vec<u8>,
        private_key: Vec<u8>,
    ) -> Pin<Box<(dyn Future<Output = Result<Vec<u8>, CryptoError>> + 'static)>>
    {
        todo!("#165 Undescribed by author.")
    }

    fn sign_rsa(
        &self,
        data: Vec<u8>,
        private_key: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>>>> {
        todo!("#166 Undescribed by author.")
    }

    fn verify_rsa(
        &self,
        data: Vec<u8>,
        signature: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, CryptoError>>>> {
        todo!("#167 Undescribed by author.")
    }

    fn create_uuid(&self) -> String {
        // use pseudo-random UUID for testing
        cfg_if::cfg_if! {
            if #[cfg(feature = "debug")] {
                if get_global_context().debug_flags.enable_deterministic_behavior {
                    generate_pseudo_uuid()
                }
                else {
                    Uuid::new_v4().to_string()
                }
            }
            else {
                Uuid::new_v4().to_string()
            }
        }
    }

    fn random_bytes(&self, length: usize) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        (0..length).map(|_| rng.r#gen()).collect()
    }

    fn new_encryption_key_pair(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<u8>, Vec<u8>), CryptoError>>>>
    {
        Box::pin(async {
            let mut rng = OsRng;
            let private_key = RsaPrivateKey::new(&mut rng, 4096)
                .map_err(|_| CryptoError::KeyGeneratorFailed)?;

            let private_key_der = private_key
                .to_pkcs8_der()
                .map_err(|_| CryptoError::KeyExportFailed)?
                .as_bytes()
                .to_vec();
            let public_key = RsaPublicKey::from(&private_key);

            let public_key_der = public_key
                .to_public_key_der()
                .map_err(|_| CryptoError::KeyExportFailed)?
                .as_bytes()
                .to_vec();

            Ok((public_key_der, private_key_der))
        })
    }

    fn new_sign_key_pair(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<u8>, Vec<u8>), CryptoError>>>>
    {
        todo!("#168 Undescribed by author.")
    }
}

// TODO #169: reenable
/*#[cfg(test)]
mod tests {
    use super::*;
    static CRYPTO: CryptoNative = CryptoNative {};

    #[test]
    fn uuid() {
        let uuid = CRYPTO.create_uuid();
        assert_eq!(uuid.len(), 36);

        for _ in 0..100 {
            assert_ne!(CRYPTO.create_uuid(), uuid);
        }
    }

    #[test]
    fn random_bytes() {
        let random_bytes = CRYPTO.random_bytes(32);
        assert_eq!(random_bytes.len(), 32);
    }

    #[tokio::test]
    async fn test_enc_key_pair() {
        let key_pair = CRYPTO.new_encryption_key_pair().await.unwrap();
        assert_eq!(key_pair.0.len(), 550);
        // assert_eq!(key_pair.1.len(), 2375);
    }
}*/
