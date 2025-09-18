use crate::stdlib::{future::Future, pin::Pin};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use super::crypto::{CryptoError, CryptoTrait};
use rand::{Rng, rngs::OsRng};
use rsa::{
    RsaPrivateKey, RsaPublicKey,
    pkcs8::{EncodePrivateKey, EncodePublicKey},
};
use uuid::Uuid;
#[cfg(not(target_arch = "wasm32"))]
use openssl::{
    derive::Deriver,
    md::Md,
    pkey::PKey,
    sign::{Signer, Verifier},
};


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

    fn create_uuid(&self) -> String {
        // use pseudo-random UUID for testing
        cfg_if::cfg_if! {
            if #[cfg(feature = "debug")] {
                use crate::runtime::global_context::get_global_context;
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

    // EdDSA keygen
    fn gen_ed25519(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<u8>, Vec<u8>), CryptoError>> + 'static>>
    {
        Box::pin(async move {
            let key = PKey::generate_ed25519().map_err(|_| CryptoError::KeyGeneratorFailed)?;

            let public_key: Vec<u8> = key
                .public_key_to_der()
                .map_err(|_| CryptoError::KeyGeneratorFailed)?
                .try_into()
                .map_err(|_| CryptoError::KeyGeneratorFailed)?;
            let private_key: Vec<u8>= key
                .private_key_to_pkcs8()
                .map_err(|_| CryptoError::KeyGeneratorFailed)?
                .try_into()
                .map_err(|_| CryptoError::KeyGeneratorFailed)?;
            Ok((public_key, private_key))
        })
    }

    // EdDSA signature
    fn sig_ed25519<'a>(
        &'a self,
        pri_key: &'a Vec<u8>,
        data: &'a Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>> + 'a>> {
        Box::pin(async move {
            let sig_key = PKey::private_key_from_pkcs8(pri_key)
                .map_err(|_| CryptoError::KeyImportFailed)?;
            let mut signer =
                Signer::new_without_digest(&sig_key).map_err(|_| CryptoError::SigningError)?;
            let signature = signer
                .sign_oneshot_to_vec(data)
                .map_err(|_| CryptoError::SigningError)?;
            Ok(signature)
        })
    }

    // EdDSA verification of signature
    fn ver_ed25519<'a>(
        &'a self,
        pub_key: &'a Vec<u8>,
        sig: &'a Vec<u8>,
        data: &'a Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, CryptoError>> + 'a>> {
        Box::pin(async move {
            let public_key = PKey::public_key_from_der(pub_key)
                .map_err(|_| CryptoError::KeyImportFailed)?;
            let mut verifier = Verifier::new_without_digest(&public_key)
                .map_err(|_| CryptoError::KeyImportFailed)?;
            Ok(verifier
                .verify_oneshot(sig, &data)
                .map_err(|_| CryptoError::VerificationError)?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static CRYPTO: CryptoNative = CryptoNative {};
    #[tokio::test]
    pub async fn test_dsa_ed2519() {
        let data = b"Some message to sign".to_vec();

        let (pub_key, pri_key) = CRYPTO.gen_ed25519().await.unwrap();

        let sig: Vec<u8> = CRYPTO
            .sig_ed25519(&pri_key, &data)
            .await
            .unwrap();

        assert_eq!(sig.len(), 64);

        assert!(CRYPTO.ver_ed25519(&pub_key, &sig, &data).await.unwrap());
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
