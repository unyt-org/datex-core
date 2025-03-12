use std::usize;

pub trait Crypto: Send + Sync {
    fn encrypt(&self, data: &[u8]) -> Vec<u8>;
    fn decrypt(&self, data: &[u8]) -> Vec<u8>;
    fn create_uuid(&self) -> String;
    fn random_bytes(&self, length: usize) -> Vec<u8>;
}

pub struct CryptoDefault;
impl Crypto for CryptoDefault {
    fn encrypt(&self, data: &[u8]) -> Vec<u8> {
        unreachable!()
    }

    fn decrypt(&self, data: &[u8]) -> Vec<u8> {
        unreachable!()
    }

    fn create_uuid(&self) -> String {
        unreachable!()
    }

    fn random_bytes(&self, length: usize) -> Vec<u8> {
        unreachable!()
    }
}
