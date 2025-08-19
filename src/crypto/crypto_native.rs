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
use openssl::{
    bn::BigNumContext,
    ec::{EcGroup, EcKey, PointConversionForm},
    hash::MessageDigest,
    nid::Nid,
    pkey::{PKey, Private, Public},
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
impl CryptoNative {

    pub fn ec_keypair() -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        // Curves: crypto-api=openssl::nid
        // P-256=X9_62_PRIME256V1, P-384=SECP384R1, P-512=SECP512R1
        let group = EcGroup::from_curve_name(Nid::BRAINPOOL_P384R1)
            .map_err(|_| CryptoError::KeyGeneratorFailed).unwrap();
        let ec = EcKey::generate(&group)
            .map_err(|_| CryptoError::KeyGeneratorFailed).unwrap();


        let mut ctx = BigNumContext::new().unwrap();

        let public_key = &ec.public_key().to_bytes(
            &group,
            PointConversionForm::COMPRESSED,
            &mut ctx,
        ).unwrap();
        let private_key = ec.private_key().to_vec();

        Ok((public_key.to_vec(), private_key))
    }

    // ECDSA
    pub fn gen_keypair() -> Result<PKey<Private>, CryptoError> {
        // Curves: crypto-api=openssl::nid
        // P-256=X9_62_PRIME256V1, P-384=SECP384R1, P-512=SECP512R1
        let group = EcGroup::from_curve_name(Nid::BRAINPOOL_P384R1)
            .map_err(|_| CryptoError::KeyGeneratorFailed).unwrap();
        let ec = EcKey::generate(&group)
            .map_err(|_| CryptoError::KeyGeneratorFailed).unwrap();
        Ok(PKey::from_ec_key(ec).unwrap())
    }

    pub fn sign(privkey: &PKey<Private>, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut signer = Signer::new(MessageDigest::sha256(), privkey)
            .map_err(|_| CryptoError::SigningError).unwrap();
        signer.update(data)
            .map_err(|_| CryptoError::SigningError).unwrap();
        Ok(signer.sign_to_vec().map_err(|_| CryptoError::SigningError).unwrap())
    }

    pub fn verify(pubkey: &PKey<Public>, data: &[u8], sign: &[u8]) -> Result<bool, CryptoError> {
        let mut verifier = Verifier::new(MessageDigest::sha256(), pubkey)
            .map_err(|_| CryptoError::VerificationError).unwrap();
        verifier.update(data)
            .map_err(|_| CryptoError::VerificationError).unwrap();
        Ok(verifier.verify(sign).map_err(|_| CryptoError::VerificationError).unwrap())
    }
}
impl CryptoTrait for CryptoNative {
    fn encrypt_rsa(
        &self,
        data: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Pin<Box<(dyn Future<Output = Result<Vec<u8>, CryptoError>> + 'static)>>
    {
        todo!()
    }

    fn decrypt_rsa(
        &self,
        data: Vec<u8>,
        private_key: Vec<u8>,
    ) -> Pin<Box<(dyn Future<Output = Result<Vec<u8>, CryptoError>> + 'static)>>
    {
        todo!()
    }

    fn sign_rsa(
        &self,
        data: Vec<u8>,
        private_key: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>>>> {
        todo!()
    }

    fn verify_rsa(
        &self,
        data: Vec<u8>,
        signature: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, CryptoError>>>> {
        todo!()
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
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static CRYPTO: CryptoNative = CryptoNative {};

    /*
    #[test]
    fn uuid() {
        let uuid = CRYPTO.create_uuid();
        assert_eq!(uuid.len(), 36);

        for _ in 0..100 {
            assert_ne!(CRYPTO.create_uuid(), uuid);
        }
    }
    */

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

    #[test]
    fn sign_verify() {
        let data = b"Datex-core";
        let fake_data = b"Datex-tractor";

        let server_pkey = CryptoNative::gen_keypair().unwrap();
        let server_pub_pem = server_pkey.public_key_to_pem().unwrap();
        let server_pub_key = PKey::public_key_from_pem(&server_pub_pem);

        let sig = CryptoNative::sign(&server_pkey, data).unwrap();

        let verified = CryptoNative::verify(&server_pub_key.as_ref().unwrap(), data, &sig).unwrap();
        let unverified = CryptoNative::verify(&server_pub_key.unwrap(), fake_data, &sig).unwrap();

        assert!(verified);
        assert!(!unverified);
    }

    #[test]
    fn ec_keygen() {
        let (pub_key, pri_key) = CryptoNative::ec_keypair()
            .map_err(|_| CryptoError::KeyGeneratorFailed).unwrap();
        // pub_key.len() = 33 for secp256, 49 for brainpool384
        assert_eq!(pub_key.len(), 49);
        assert_ne!(pub_key[0], 0x04);
        assert!(pri_key.len() >= 31);
    }
}
