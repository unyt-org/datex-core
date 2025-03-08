use super::crypto::Crypto;
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
}
