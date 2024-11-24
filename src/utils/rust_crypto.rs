
// default rust implementation for crypto

use super::crypto::Crypto;

pub struct RustCrypto {}

impl Crypto for RustCrypto {
    fn encrypt_aes(&self, _buffer:&[u8]) -> Vec<u8> {
        todo!()
    }
}