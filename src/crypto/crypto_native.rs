use crate::stdlib::pin::Pin;

use super::crypto::{Crypto, CryptoError};
use rand::{rngs::OsRng, Rng};
use rsa::{
    pkcs8::{EncodePrivateKey, EncodePublicKey},
    RsaPrivateKey, RsaPublicKey,
};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct CryptoNative;
impl Crypto for CryptoNative {
    fn create_uuid(&self) -> String {
        Uuid::new_v4().to_string()
    }

    fn random_bytes(&self, length: usize) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        (0..length).map(|_| rng.gen()).collect()
    }

    fn new_encryption_key_pair(
        &self,
    ) -> Pin<
        Box<
            dyn std::prelude::rust_2024::Future<
                Output = Result<(Vec<u8>, Vec<u8>), super::crypto::CryptoError>,
            >,
        >,
    > {
        Box::pin(async {
            let mut rng = OsRng;
            let private_key = RsaPrivateKey::new(&mut rng, 4096)
                .map_err(|_| super::crypto::CryptoError::KeyGeneratorFailed)?;

            let private_key_der = private_key
                .to_pkcs8_der()
                .map_err(|_| super::crypto::CryptoError::KeyExportFailed)?
                .as_bytes()
                .to_vec();
            let public_key = RsaPublicKey::from(&private_key);

            let public_key_der = public_key
                .to_public_key_der()
                .map_err(|_| super::crypto::CryptoError::KeyExportFailed)?
                .as_bytes()
                .to_vec();

            Ok((public_key_der, private_key_der))
        })
    }

    fn new_sign_key_pair(
        &self,
    ) -> Pin<
        Box<
            dyn std::prelude::rust_2024::Future<
                Output = Result<(Vec<u8>, Vec<u8>), super::crypto::CryptoError>,
            >,
        >,
    > {
        todo!()
    }

    fn encrypt_rsa(
        &self,
        data: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Pin<
        Box<
            (dyn std::future::Future<Output = Result<Vec<u8>, CryptoError>>
                 + 'static),
        >,
    > {
        todo!()
    }

    fn decrypt_rsa(
        &self,
        data: Vec<u8>,
        private_key: Vec<u8>,
    ) -> Pin<
        Box<
            (dyn std::future::Future<Output = Result<Vec<u8>, CryptoError>>
                 + 'static),
        >,
    > {
        todo!()
    }

    fn sign_rsa(
        &self,
        data: Vec<u8>,
        private_key: Vec<u8>,
    ) -> Pin<
        Box<
            dyn std::prelude::rust_2024::Future<
                Output = Result<Vec<u8>, CryptoError>,
            >,
        >,
    > {
        todo!()
    }

    fn verify_rsa(
        &self,
        data: Vec<u8>,
        signature: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Pin<
        Box<
            dyn std::prelude::rust_2024::Future<
                Output = Result<bool, CryptoError>,
            >,
        >,
    > {
        todo!()
    }
}

// TODO: reenable
/*#[cfg(test)]
mod test {
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
