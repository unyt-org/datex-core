use std::pin::Pin;

use super::crypto::{Crypto, CryptoError};
use rand::RngCore;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct CryptoNative;
impl Crypto for CryptoNative {
    fn create_uuid(&self) -> String {
        Uuid::new_v4().to_string()
    }

    fn random_bytes(&self, length: usize) -> Vec<u8> {
        let mut rng = rand::rng();
        let mut buffer = vec![0u8; length];
        rng.fill_bytes(&mut buffer);
        buffer
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
        todo!()
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
