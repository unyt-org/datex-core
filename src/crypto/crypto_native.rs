use crate::stdlib::{future::Future, pin::Pin};
use rand::Rng;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use super::crypto::{CryptoError, CryptoTrait};
use uuid::Uuid;

use openssl::{
    aes::{AesKey, unwrap_key, wrap_key},
    derive::Deriver,
    pkey::PKey,
    sign::{Signer, Verifier},
    symm::{Cipher, Crypter, Mode},
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
impl CryptoTrait for CryptoNative {
    fn create_uuid(&self) -> String {
        // use pseudo-random UUID for testing
        cfg_if::cfg_if! {
            if #[cfg(feature = "debug")] {
                use crate::runtime::global_context::get_global_context;
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

    // EdDSA keygen
    fn gen_ed25519(
        &self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(Vec<u8>, Vec<u8>), CryptoError>>
                + 'static,
        >,
    > {
        Box::pin(async move {
            let key = PKey::generate_ed25519()
                .map_err(|_| CryptoError::KeyGeneratorFailed)?;

            let public_key: Vec<u8> = key
                .public_key_to_der()
                .map_err(|_| CryptoError::KeyGeneratorFailed)?;
            let private_key: Vec<u8> = key
                .private_key_to_pkcs8()
                .map_err(|_| CryptoError::KeyGeneratorFailed)?;
            Ok((public_key, private_key))
        })
    }

    // EdDSA signature
    fn sig_ed25519<'a>(
        &'a self,
        pri_key: &'a [u8],
        data: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<[u8; 64], CryptoError>> + 'a>> {
        Box::pin(async move {
            let sig_key = PKey::private_key_from_pkcs8(pri_key)
                .map_err(|_| CryptoError::KeyImportFailed)?;
            let mut signer = Signer::new_without_digest(&sig_key)
                .map_err(|_| CryptoError::SigningError)?;
            let signature = signer
                .sign_oneshot_to_vec(data)
                .map_err(|_| CryptoError::SigningError)?;
            let signature: [u8; 64] =
                signature.try_into().expect("Invalid signature length");
            Ok(signature)
        })
    }

    // EdDSA verification of signature
    fn ver_ed25519<'a>(
        &'a self,
        pub_key: &'a [u8],
        sig: &'a [u8],
        data: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<bool, CryptoError>> + 'a>> {
        Box::pin(async move {
            let public_key = PKey::public_key_from_der(pub_key)
                .map_err(|_| CryptoError::KeyImportFailed)?;
            let mut verifier = Verifier::new_without_digest(&public_key)
                .map_err(|_| CryptoError::KeyImportFailed)?;
            verifier
                .verify_oneshot(sig, data)
                .map_err(|_| CryptoError::VerificationError)
        })
    }

    // AES CTR
    fn aes_ctr_encrypt<'a>(
        &'a self,
        key: &'a [u8; 32],
        iv: &'a [u8; 16],
        plaintext: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>> + 'a>> {
        Box::pin(async move {
            let cipher = Cipher::aes_256_ctr();
            let mut enc = Crypter::new(cipher, Mode::Encrypt, key, Some(iv))
                .map_err(|_| CryptoError::EncryptionError)?;

            let mut out = vec![0u8; plaintext.len()];
            let count = enc
                .update(plaintext, &mut out)
                .map_err(|_| CryptoError::EncryptionError)?;
            out.truncate(count);
            Ok(out)
        })
    }

    fn aes_ctr_decrypt<'a>(
        &'a self,
        key: &'a [u8; 32],
        iv: &'a [u8; 16],
        ciphertext: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>> + 'a>> {
        self.aes_ctr_encrypt(key, iv, ciphertext)
    }

    // AES KW
    fn key_upwrap<'a>(
        &'a self,
        kek_bytes: &'a [u8; 32],
        rb: &'a [u8; 32],
    ) -> Pin<Box<dyn Future<Output = Result<[u8; 40], CryptoError>> + 'a>> {
        Box::pin(async move {
            // Key encryption key
            let kek = AesKey::new_encrypt(kek_bytes)
                .map_err(|_| CryptoError::EncryptionError)?;

            // Key wrap
            let mut wrapped = [0u8; 40];
            let _length = wrap_key(&kek, None, &mut wrapped, rb);

            Ok(wrapped)
        })
    }

    fn key_unwrap<'a>(
        &'a self,
        kek_bytes: &'a [u8; 32],
        cipher: &'a [u8; 40],
    ) -> Pin<Box<dyn Future<Output = Result<[u8; 32], CryptoError>> + 'a>> {
        Box::pin(async move {
            // Key encryption key
            let kek = AesKey::new_decrypt(kek_bytes)
                .map_err(|_| CryptoError::DecryptionError)?;

            // Unwrap key
            let mut unwrapped: [u8; 32] = [0u8; 32];
            let _length = unwrap_key(&kek, None, &mut unwrapped, cipher);
            Ok(unwrapped)
        })
    }

    // Generate encryption keypair
    fn gen_x25519(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<([u8; 44], [u8; 48]), CryptoError>>>>
    {
        Box::pin(async move {
            let key = PKey::generate_x25519()
                .map_err(|_| CryptoError::KeyGeneratorFailed)?;
            let public_key: [u8; 44] = key
                .public_key_to_der()
                .map_err(|_| CryptoError::KeyGeneratorFailed)?
                .try_into()
                .map_err(|_| CryptoError::KeyGeneratorFailed)?;
            let private_key: [u8; 48] = key
                .private_key_to_pkcs8()
                .map_err(|_| CryptoError::KeyGeneratorFailed)?
                .try_into()
                .map_err(|_| CryptoError::KeyGeneratorFailed)?;
            Ok((public_key, private_key))
        })
    }

    // Derive shared secret on x255109
    fn derive_x25519<'a>(
        &'a self,
        my_raw: &'a [u8; 48],
        peer_pub: &'a [u8; 44],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>> + 'a>> {
        Box::pin(async move {
            let peer_pub = PKey::public_key_from_der(peer_pub)
                .map_err(|_| CryptoError::KeyImportFailed)?;
            let my_priv = PKey::private_key_from_pkcs8(my_raw)
                .map_err(|_| CryptoError::KeyImportFailed)?;

            let mut deriver = Deriver::new(&my_priv)
                .map_err(|_| CryptoError::KeyGeneratorFailed)?;
            deriver
                .set_peer(&peer_pub)
                .map_err(|_| CryptoError::KeyGeneratorFailed)?;
            deriver
                .derive_to_vec()
                .map_err(|_| CryptoError::KeyGeneratorFailed)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static CRYPTO: CryptoNative = CryptoNative {};

    // Signatures
    #[tokio::test]
    pub async fn test_dsa_ed2519() {
        let data = b"Some message to sign".to_vec();

        let (pub_key, pri_key) = CRYPTO.gen_ed25519().await.unwrap();

        let sig: [u8; 64] = CRYPTO.sig_ed25519(&pri_key, &data).await.unwrap();
        assert!(CRYPTO.ver_ed25519(&pub_key, &sig, &data).await.unwrap());
    }

    // AES CTR
    #[tokio::test]
    pub async fn aes_ctr_roundtrip() {
        let key = [0u8; 32];
        let iv = [0u8; 16];

        let data = b"Some message to encrypt".to_vec();

        let ciphered = CRYPTO.aes_ctr_encrypt(&key, &iv, &data).await.unwrap();
        let deciphered =
            CRYPTO.aes_ctr_decrypt(&key, &iv, &ciphered).await.unwrap();

        assert_ne!(ciphered, data);
        assert_eq!(data, deciphered.to_vec());
    }

    #[tokio::test]
    pub async fn test_keywrapping() {
        let kek_bytes = [1u8; 32];
        let sym_key: [u8; 32] =
            CRYPTO.random_bytes(32_usize).try_into().unwrap();
        let arand = CRYPTO.key_upwrap(&kek_bytes, &sym_key).await.unwrap();
        let brand = CRYPTO.key_unwrap(&kek_bytes, &arand).await.unwrap();

        assert_ne!(arand.to_vec(), brand.to_vec());
        assert_eq!(arand.len(), brand.len() + 8);
    }

    #[tokio::test]
    pub async fn test_keywrapping_more() {
        // Copy pasta from Web Crypto implementation
        let kek: [u8; 32] = [
            176, 213, 29, 202, 131, 45, 220, 153, 250, 120, 219, 65, 177, 117,
            244, 172, 38, 107, 221, 109, 160, 134, 15, 195, 23, 22, 143, 238,
            242, 222, 38, 248,
        ];

        let web_wrapped: [u8; 40] = [
            140, 223, 207, 46, 9, 105, 205, 24, 174, 238, 109, 5, 96, 4, 51,
            132, 54, 187, 251, 167, 105, 131, 109, 246, 123, 238, 160, 139,
            180, 59, 185, 8, 191, 57, 139, 133, 19, 40, 15, 210,
        ];

        let wrapped = CRYPTO.key_upwrap(&kek, &kek).await.unwrap();

        let unwrapped = CRYPTO.key_unwrap(&kek, &wrapped).await.unwrap();
        let web_unwrapped =
            CRYPTO.key_unwrap(&kek, &web_wrapped).await.unwrap();

        assert_eq!(kek, unwrapped);
        assert_eq!(kek, web_unwrapped);
    }

    #[tokio::test]
    async fn test_dh_x25519() {
        let (ser_pub, ser_pri) = CRYPTO.gen_x25519().await.unwrap();
        let (cli_pub, cli_pri) = CRYPTO.gen_x25519().await.unwrap();

        let cli_shared =
            CRYPTO.derive_x25519(&cli_pri, &ser_pub).await.unwrap();
        let ser_shared =
            CRYPTO.derive_x25519(&ser_pri, &cli_pub).await.unwrap();

        assert_eq!(cli_shared, ser_shared);
        assert_eq!(cli_shared.len(), 32);
    }

    #[tokio::test]
    pub async fn test_multi_roundtrip() {
        // Given
        let mut client_list = Vec::new();

        // Generate symmetric random key
        let sym_key: [u8; 32] = CRYPTO.random_bytes(32).try_into().unwrap();

        for _ in 0..10 {
            let (cli_pub, cli_pri) = CRYPTO.gen_x25519().await.unwrap();
            client_list.push((cli_pri, cli_pub));
        }

        // Encrypt data with symmetric key
        let data = b"Some message to encrypt".to_vec();
        let iv = [0u8; 16];
        let cipher =
            CRYPTO.aes_ctr_encrypt(&sym_key, &iv, &data).await.unwrap();

        // Sender (server)
        let mut payloads = Vec::new();
        for (_, peer_pub) in client_list.iter().take(10) {
            let (ser_pub, ser_pri) = CRYPTO.gen_x25519().await.unwrap();
            let ser_kek_bytes: [u8; 32] = CRYPTO
                .derive_x25519(&ser_pri, peer_pub)
                .await
                .unwrap()
                .try_into()
                .unwrap();

            let wrapped =
                CRYPTO.key_upwrap(&ser_kek_bytes, &sym_key).await.unwrap();

            payloads.push((ser_pub, wrapped));
        }

        // Receiver (client)
        for i in 0..10 {
            // Unwraps key and decrypts
            let cli_kek_bytes: [u8; 32] = CRYPTO
                .derive_x25519(&client_list[i].0, &payloads[i].0)
                .await
                .unwrap()
                .try_into()
                .unwrap();
            let unwrapped = CRYPTO
                .key_unwrap(&cli_kek_bytes, &payloads[i].1)
                .await
                .unwrap();
            let plain = CRYPTO
                .aes_ctr_decrypt(&unwrapped, &iv, &cipher)
                .await
                .unwrap();

            // Check key wraps
            assert_ne!(payloads[i].1.to_vec(), unwrapped.to_vec());
            assert_eq!(payloads[i].1.len(), unwrapped.len() + 8);

            // Check data, cipher and deciphered
            assert_ne!(data, cipher);
            assert_eq!(plain, data);
        }
    }
}
