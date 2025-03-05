pub trait Crypto {
	fn encrypt(&self, data: &[u8]) -> Vec<u8>;
	fn decrypt(&self, data: &[u8]) -> Vec<u8>;
	fn create_uuid(&self) -> String;
}