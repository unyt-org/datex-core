use super::crypto::{CryptoError, CryptoTrait, MaybeAsync};
use crate::stdlib::sync::OnceLock;
use crate::stdlib::sync::atomic::{AtomicU64, Ordering};
use crate::stdlib::{future::Future, pin::Pin};
use core::prelude::rust_2024::*;
use core::result::Result;
use openssl::{
    aes::{AesKey, unwrap_key, wrap_key},
    derive::Deriver,
    md::Md,
    pkey::{Id, PKey},
    pkey_ctx::{HkdfMode, PkeyCtx},
    sha::sha256,
    sign::{Signer, Verifier},
    symm::{Cipher, Crypter, Mode},
};
use rand::Rng;
use uuid::Uuid;

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

    fn hash<'a>(
        &'a self,
        to_digest: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<[u8; 32], CryptoError>> + 'a>> {
        Box::pin(async move {
            let hash = sha256(to_digest);
            Ok(hash)
        })
    }

    fn hkdf<'a>(
        &'a self,
        ikm: &'a [u8],
        salt: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<[u8; 32], CryptoError>> + 'a>> {
        Box::pin(async move {
            let info = b"";
            let mut ctx = PkeyCtx::new_id(Id::HKDF)
                .map_err(|_| CryptoError::KeyGenerationError)?;
            ctx.derive_init()
                .map_err(|_| CryptoError::KeyGenerationError)?;
            ctx.set_hkdf_mode(HkdfMode::EXTRACT_THEN_EXPAND)
                .map_err(|_| CryptoError::KeyGenerationError)?;
            ctx.set_hkdf_md(Md::sha256())
                .map_err(|_| CryptoError::KeyGenerationError)?;
            ctx.set_hkdf_salt(salt)
                .map_err(|_| CryptoError::KeyGenerationError)?;
            ctx.set_hkdf_key(ikm)
                .map_err(|_| CryptoError::KeyGenerationError)?;
            ctx.add_hkdf_info(info)
                .map_err(|_| CryptoError::KeyGenerationError)?;
            let mut okm = [0u8; 32_usize];
            ctx.derive(Some(&mut okm))
                .map_err(|_| CryptoError::KeyGenerationError)?;
            Ok(okm)
        })
    }
    // EdDSA keygen
    fn gen_ed25519<'a>(
        &'a self,
    ) -> Pin<
        Box<dyn Future<Output = Result<(Vec<u8>, Vec<u8>), CryptoError>> + 'a>,
    > {
        Box::pin(async move {
            let key = PKey::generate_ed25519()
                .map_err(|_| CryptoError::KeyGenerationError)?;

            let public_key: Vec<u8> = key
                .public_key_to_der()
                .map_err(|_| CryptoError::KeyGenerationError)?;
            let private_key: Vec<u8> = key
                .private_key_to_pkcs8()
                .map_err(|_| CryptoError::KeyGenerationError)?;
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
                .map_err(|_| CryptoError::KeyImportError)?;
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
                .map_err(|_| CryptoError::KeyImportError)?;
            let mut verifier = Verifier::new_without_digest(&public_key)
                .map_err(|_| CryptoError::KeyImportError)?;
            let verification = verifier
                .verify_oneshot(sig, data)
                .map_err(|_| CryptoError::VerificationError)?;
            Ok(verification)
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
    fn gen_x25519<'a>(
        &'a self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<([u8; 44], [u8; 48]), CryptoError>> + 'a,
        >,
    > {
        Box::pin(async move {
            let key = PKey::generate_x25519()
                .map_err(|_| CryptoError::KeyGenerationError)?;
            let public_key: [u8; 44] = key
                .public_key_to_der()
                .map_err(|_| CryptoError::KeyGenerationError)?
                .try_into()
                .map_err(|_| CryptoError::KeyGenerationError)?;
            let private_key: [u8; 48] = key
                .private_key_to_pkcs8()
                .map_err(|_| CryptoError::KeyGenerationError)?
                .try_into()
                .map_err(|_| CryptoError::KeyGenerationError)?;
            Ok((public_key, private_key))
        })
    }

    // Derive shared secret on x255109
    fn derive_x25519<'a>(
        &'a self,
        pri_key: &'a [u8; 48],
        peer_pub: &'a [u8; 44],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>> + 'a>> {
        Box::pin(async move {
            let peer_pub = PKey::public_key_from_der(peer_pub)
                .map_err(|_| CryptoError::KeyImportError)?;
            let my_priv = PKey::private_key_from_pkcs8(pri_key)
                .map_err(|_| CryptoError::KeyImportError)?;

            let mut deriver = Deriver::new(&my_priv)
                .map_err(|_| CryptoError::KeyGenerationError)?;
            deriver
                .set_peer(&peer_pub)
                .map_err(|_| CryptoError::KeyGenerationError)?;
            let derived = deriver
                .derive_to_vec()
                .map_err(|_| CryptoError::KeyGenerationError)?;
            Ok(derived)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static CRYPTO: CryptoNative = CryptoNative {};

    #[tokio::test]
    pub async fn hash_derivation() {
        let ikm = Vec::from([0u8; 32]);
        let hash = CRYPTO.hash(&ikm).await.unwrap();
        assert_eq!(
            hash,
            [
                102, 104, 122, 173, 248, 98, 189, 119, 108, 143, 193, 139, 142,
                159, 142, 32, 8, 151, 20, 133, 110, 226, 51, 179, 144, 42, 89,
                29, 13, 95, 41, 37
            ]
        );
    }

    #[tokio::test]
    pub async fn hash_key_derivation() {
        let mut ikm = Vec::from([0u8; 32]);
        let salt = Vec::from([0u8; 16]);
        let hash_a = CRYPTO.hkdf(&ikm, &salt).await.unwrap();
        ikm[0] = 1u8;
        let hash_b = CRYPTO.hkdf(&ikm, &salt).await.unwrap();
        assert_ne!(hash_a, hash_b);
        assert_ne!(hash_a.to_vec(), ikm);
        assert_eq!(
            hash_a,
            [
                223, 114, 4, 84, 111, 27, 238, 120, 184, 83, 36, 167, 137, 140,
                161, 25, 179, 135, 224, 19, 134, 209, 174, 240, 55, 120, 29,
                74, 138, 3, 106, 238
            ]
        );
    }

    // Signatures
    #[tokio::test]
    pub async fn asy_dsa_ed2519() {
        let data = b"Some message to sign".to_vec();

        let (pub_key, pri_key) = CRYPTO.gen_ed25519().await.unwrap();
        let sig = CRYPTO.sig_ed25519(&pri_key, &data).await.unwrap();
        let ver = CRYPTO.ver_ed25519(&pub_key, &sig, &data).await.unwrap();

        assert!(ver)
    }

    #[tokio::test]
    pub async fn asy_aes_ctr() {
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
    pub async fn asy_key_wrap() {
        let kek_bytes = [1u8; 32];
        let sym_key: [u8; 32] =
            CRYPTO.random_bytes(32_usize).try_into().unwrap();
        let arand = CRYPTO.key_upwrap(&kek_bytes, &sym_key).await.unwrap();
        let brand = CRYPTO.key_unwrap(&kek_bytes, &arand).await.unwrap();

        assert_ne!(arand.to_vec(), brand.to_vec());
        assert_eq!(arand.len(), brand.len() + 8);
    }

    #[tokio::test]
    async fn asy_key_gen_x25519() {
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
    pub async fn crypto_multi_roundtrip() {
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
