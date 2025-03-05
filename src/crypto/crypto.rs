pub trait Crypto {
  fn encrypt(&self, data: &[u8]) -> Vec<u8>;
  fn decrypt(&self, data: &[u8]) -> Vec<u8>;
  fn create_uuid(&self) -> String;
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
}
