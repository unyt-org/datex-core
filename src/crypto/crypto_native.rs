use super::crypto::Crypto;
use rand::RngCore;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct CryptoNative;
impl Crypto for CryptoNative {
    fn encrypt(&self, data: &[u8]) -> Vec<u8> {
        todo!()
    }

    fn decrypt(&self, data: &[u8]) -> Vec<u8> {
        todo!()
    }

    fn create_uuid(&self) -> String {
        Uuid::new_v4().to_string()
    }

    fn random_bytes(&self, length: usize) -> Vec<u8> {
        let mut rng = rand::rng();
        let mut buffer = vec![0u8; length];
        rng.fill_bytes(&mut buffer);
        buffer
    }
}
