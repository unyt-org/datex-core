pub trait Crypto {
  fn encrypt_aes(&self, buffer: &[u8]) -> Vec<u8>;
}
