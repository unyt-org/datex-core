use crate::stdlib::{future::Future, pin::Pin, usize};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use super::crypto::{CryptoError, CryptoTrait};
use crate::runtime::global_context::get_global_context;
use openssl::{
    derive::Deriver,
    md::Md,
    pkey::{Id, PKey},
    pkey_ctx::{HkdfMode, PkeyCtx},
    rand::rand_bytes,
    sign::{Signer, Verifier},
    symm::{Cipher, Crypter, Mode},
};
use rand::{Rng, rngs::OsRng};
use rsa::{
    RsaPrivateKey, RsaPublicKey,
    pkcs8::{EncodePrivateKey, EncodePublicKey},
};
use uuid::Uuid;

static UUID_COUNTER: OnceLock<AtomicU64> = OnceLock::new();

pub const KEY_LEN: usize = 32;
pub const IV_LEN: usize = 12;
pub const TAG_LEN: usize = 16;
pub const SALT_LEN: usize = 16;
pub const SIG_LEN: usize = 64;

// ECIES Cryptographic Message Syntax
#[derive(Debug, Clone)]
pub struct Crypt {
    // Senders eph EC pub key (PEM)
    pub pub_key: [u8; KEY_LEN],
    // HKDF salt
    pub salt: [u8; SALT_LEN],
    // IV/nonce for AES-GCM
    pub iv: [u8; IV_LEN],
    // ciphertext
    pub ct: Vec<u8>,
    // AES-GCM tag (128-bit)
    pub tag: [u8; TAG_LEN],
}

fn init_counter() -> &'static AtomicU64 {
    UUID_COUNTER.get_or_init(|| AtomicU64::new(1))
}
fn generate_pseudo_uuid() -> String {
    let counter = init_counter();
    let count = counter.fetch_add(1, Ordering::Relaxed);

    // Encode counter into last segment, keeping UUID-like structure
    format!("00000000-0000-0000-0000-{count:012x}")
}

// HKDF (hash)
pub fn hkdf(ikm: &[u8], salt: &[u8], out_len: usize) -> Result<Vec<u8>, CryptoError> {
    let mut ctx = PkeyCtx::new_id(Id::HKDF).map_err(|_| CryptoError::KeyDerivationFailed)?;
    ctx.derive_init()
        .map_err(|_| CryptoError::KeyDerivationFailed)?;
    ctx.set_hkdf_mode(HkdfMode::EXTRACT_THEN_EXPAND)
        .map_err(|_| CryptoError::KeyDerivationFailed)?;
    ctx.set_hkdf_md(&Md::sha256())
        .map_err(|_| CryptoError::KeyDerivationFailed)?;
    ctx.set_hkdf_salt(salt)
        .map_err(|_| CryptoError::KeyDerivationFailed)?;
    ctx.set_hkdf_key(ikm)
        .map_err(|_| CryptoError::KeyDerivationFailed)?;
    /*
    ctx.add_hkdf_info(info)
        .map_err(|_| CryptoError::KeyDerivationFailed)?;
    */
    let mut okm = vec![0u8; out_len];
    ctx.derive(Some(&mut okm))
        .map_err(|_| CryptoError::KeyDerivationFailed)?;
    Ok(okm)
}

// AES CTR
pub fn aes_ctr_encrypt(
    key: &[u8; KEY_LEN],
    iv: &[u8; 16],
    plaintext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = Cipher::aes_256_ctr();
    let mut enc = Crypter::new(cipher, Mode::Encrypt, key, Some(iv))
        .map_err(|_| CryptoError::EncryptionError)?;

    let mut out = vec![0u8; plaintext.len()];
    let count = enc
        .update(plaintext, &mut out)
        .map_err(|_| CryptoError::EncryptionError)?;
    out.truncate(count);
    Ok(out)
}

pub fn aes_ctr_decrypt(
    key: &[u8; KEY_LEN],
    iv: &[u8; 16],
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    Ok(aes_ctr_encrypt(key, iv, ciphertext).unwrap())
}

// AES GCM
pub fn aes_gcm_encrypt(
    key: &[u8; KEY_LEN],
    iv: &[u8; IV_LEN],
    aad: &[u8],
    plaintext: &[u8],
) -> Result<(Vec<u8>, [u8; TAG_LEN]), CryptoError> {
    let cipher = Cipher::aes_256_gcm();
    let mut enc = Crypter::new(cipher, Mode::Encrypt, key, Some(iv))
        .map_err(|_| CryptoError::EncryptionError)?;
    enc.aad_update(aad)
        .map_err(|_| CryptoError::EncryptionError)?;

    let mut out = vec![0u8; plaintext.len() + cipher.block_size()];
    let mut count = enc
        .update(plaintext, &mut out)
        .map_err(|_| CryptoError::EncryptionError)?;
    count += enc
        .finalize(&mut out[count..])
        .map_err(|_| CryptoError::EncryptionError)?;
    out.truncate(count);

    let mut tag = [0u8; TAG_LEN];
    enc.get_tag(&mut tag)
        .map_err(|_| CryptoError::EncryptionError)?;
    Ok((out, tag))
}

pub fn aes_gcm_decrypt(
    key: &[u8; KEY_LEN],
    iv: &[u8; IV_LEN],
    aad: &[u8],
    ciphertext: &[u8],
    tag: &[u8; TAG_LEN],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = Cipher::aes_256_gcm();
    let mut dec = Crypter::new(cipher, Mode::Decrypt, key, Some(iv))
        .map_err(|_| CryptoError::DecryptionError)?;
    dec.aad_update(aad)
        .map_err(|_| CryptoError::DecryptionError)?;
    dec.set_tag(tag).map_err(|_| CryptoError::DecryptionError)?;

    let mut out = vec![0u8; ciphertext.len() + cipher.block_size()];
    let mut count = dec
        .update(ciphertext, &mut out)
        .map_err(|_| CryptoError::DecryptionError)?;
    count += dec
        .finalize(&mut out[count..])
        .map_err(|_| CryptoError::DecryptionError)?;
    out.truncate(count);
    Ok(out)
}

// Derive shared secret on x255109
pub fn derive_x25519(
    my_raw: &[u8; KEY_LEN],
    peer_pub: &[u8; KEY_LEN],
) -> Result<Vec<u8>, CryptoError> {
    let peer_pub = PKey::public_key_from_raw_bytes(peer_pub, Id::X25519)
        .map_err(|_| CryptoError::KeyImportFailed)?;
    let my_priv = PKey::private_key_from_raw_bytes(my_raw, Id::X25519)
        .map_err(|_| CryptoError::KeyImportFailed)?;

    let mut deriver = Deriver::new(&my_priv).map_err(|_| CryptoError::KeyDerivationFailed)?;
    deriver
        .set_peer(&peer_pub)
        .map_err(|_| CryptoError::KeyDerivationFailed)?;
    deriver
        .derive_to_vec()
        .map_err(|_| CryptoError::KeyDerivationFailed)
}

#[derive(Debug, Clone, PartialEq)]
pub struct CryptoNative;
impl CryptoTrait for CryptoNative {
    fn encrypt_rsa(
        &self,
        data: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Pin<Box<(dyn Future<Output = Result<Vec<u8>, CryptoError>> + 'static)>>
    {
        todo!()
    }

    fn decrypt_rsa(
        &self,
        data: Vec<u8>,
        private_key: Vec<u8>,
    ) -> Pin<Box<(dyn Future<Output = Result<Vec<u8>, CryptoError>> + 'static)>>
    {
        todo!()
    }

    fn sign_rsa(
        &self,
        data: Vec<u8>,
        private_key: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>>>> {
        todo!()
    }

    fn verify_rsa(
        &self,
        data: Vec<u8>,
        signature: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, CryptoError>>>> {
        todo!()
    }

    fn create_uuid(&self) -> String {
        // use pseudo-random UUID for testing
        cfg_if::cfg_if! {
            if #[cfg(feature = "debug")] {
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

    fn new_encryption_key_pair(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<u8>, Vec<u8>), CryptoError>>>>
    {
        Box::pin(async {
            let mut rng = OsRng;
            let private_key = RsaPrivateKey::new(&mut rng, 4096)
                .map_err(|_| CryptoError::KeyGeneratorFailed)?;

            let private_key_der = private_key
                .to_pkcs8_der()
                .map_err(|_| CryptoError::KeyExportFailed)?
                .as_bytes()
                .to_vec();
            let public_key = RsaPublicKey::from(&private_key);

            let public_key_der = public_key
                .to_public_key_der()
                .map_err(|_| CryptoError::KeyExportFailed)?
                .as_bytes()
                .to_vec();

            Ok((public_key_der, private_key_der))
        })
    }

    fn new_sign_key_pair(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<u8>, Vec<u8>), CryptoError>>>>
    {
        todo!()
    }
    // EdDSA keygen
    fn gen_ed25519(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<u8>, Vec<u8>), CryptoError>> + 'static>>
    {
        Box::pin(async move {
            let key = PKey::generate_ed25519().map_err(|_| CryptoError::KeyGeneratorFailed)?;

            let public_key: Vec<u8> = key
                .public_key_to_der()
                .map_err(|_| CryptoError::KeyGeneratorFailed)?
                .try_into()
                .map_err(|_| CryptoError::KeyGeneratorFailed)?;
            let private_key: Vec<u8>= key
                .private_key_to_pkcs8()
                .map_err(|_| CryptoError::KeyGeneratorFailed)?
                .try_into()
                .map_err(|_| CryptoError::KeyGeneratorFailed)?;
            Ok((public_key, private_key))
        })
    }

    // EdDSA signature
    fn sig_ed25519<'a>(
        &'a self,
        pri_key: &'a Vec<u8>,
        data: &'a Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, CryptoError>> + Send + 'a>> {
        Box::pin(async move {
            let sig_key = PKey::private_key_from_pkcs8(pri_key)
                .map_err(|_| CryptoError::KeyImportFailed)?;
            let mut signer =
                Signer::new_without_digest(&sig_key).map_err(|_| CryptoError::SigningError)?;
            let signature = signer
                .sign_oneshot_to_vec(data)
                .map_err(|_| CryptoError::SigningError)?;
            Ok(signature)
        })
    }

    // EdDSA verification of signature
    fn ver_ed25519<'a>(
        &'a self,
        pub_key: &'a Vec<u8>,
        sig: &'a [u8; SIG_LEN],
        data: &'a Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, CryptoError>> + Send + 'a>> {
        Box::pin(async move {
            let public_key = PKey::public_key_from_der(pub_key)
                .map_err(|_| CryptoError::KeyImportFailed)?;
            let mut verifier = Verifier::new_without_digest(&public_key)
                .map_err(|_| CryptoError::KeyImportFailed)?;
            Ok(verifier
                .verify_oneshot(sig, &data)
                .map_err(|_| CryptoError::VerificationError)?)
        })
    }

    // Generate encryption keypair
    fn gen_x25519(&self) -> Result<([u8; KEY_LEN], [u8; KEY_LEN]), CryptoError> {
        // ) -> Pin<Box<dyn Future<Output = Result<([u8; KEY_LEN], [u8; KEY_LEN]), CryptoError>> + 'static>>
        let key = PKey::generate_x25519().map_err(|_| CryptoError::KeyGeneratorFailed)?;
        let public_key: [u8; KEY_LEN] = key
            .raw_public_key()
            .map_err(|_| CryptoError::KeyGeneratorFailed)?
            .try_into()
            .map_err(|_| CryptoError::KeyGeneratorFailed)?;
        let private_key: [u8; KEY_LEN] = key
            .raw_private_key()
            .map_err(|_| CryptoError::KeyGeneratorFailed)?
            .try_into()
            .map_err(|_| CryptoError::KeyGeneratorFailed)?;
        Ok((public_key, private_key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static CRYPTO: CryptoNative = CryptoNative {};

    /*
    #[test]
    fn uuid() {
        let uuid = CRYPTO.create_uuid();
        assert_eq!(uuid.len(), 36);

        for _ in 0..100 {
            assert_ne!(CRYPTO.create_uuid(), uuid);
        }
    }
    */

    #[test]
    fn random_bytes() {
        let random_bytes = CRYPTO.random_bytes(32);
        assert_eq!(random_bytes.len(), 32);
    }

    #[tokio::test]
    async fn test_enc_key_pair() {
        let key_pair = CRYPTO.new_encryption_key_pair().await.unwrap();
        assert_eq!(key_pair.0.len(), 550);
        // assert_eq!(key_pair.1.len(), 2375);
    }

    #[test]
    fn test_hkdf() {
        let ikm = vec![0u8; 32];
        let salt = vec![0u8; 16];

        let hash = hkdf(&ikm, &salt, 32).unwrap();

        assert_eq!(hash.len(), 32);
    }
    #[tokio::test]
    async fn test_dsa_ed2519() {
        let data = b"Some message to sign".to_vec();

        let (pub_key, pri_key) = CRYPTO.gen_ed25519().await.unwrap();

        let sig: [u8; 64] = CRYPTO
            .sig_ed25519(&pri_key, &data)
            .await
            .unwrap()
            .try_into()
            .unwrap();

        assert_eq!(sig.len(), 64);

        assert!(CRYPTO.ver_ed25519(&pub_key, &sig, &data).await.unwrap());
    }
    #[test]
    fn aes_ctr_roundtrip() {
        let key = [0u8; 32];
        let iv = [0u8; 16];

        let data = b"Some message to encrypt".to_vec();

        let ciphered = aes_ctr_encrypt(&key, &iv, &data).unwrap();
        let deciphered = aes_ctr_decrypt(&key, &iv, &ciphered).unwrap();

        assert_ne!(ciphered, data);
        assert_eq!(data, deciphered.to_vec());
    }
    #[test]
    fn aes_gcm_roundtrip() {
        let aad: &[u8] = b"Some additionally verified data by tag.";
        let key = [0u8; 32];
        let iv = [0u8; 12];

        let data = b"Some message to encrypt".to_vec();

        let (ciphered, tag) = aes_gcm_encrypt(&key, &iv, &aad, &data).unwrap();
        let deciphered = aes_gcm_decrypt(&key, &iv, &aad , &ciphered, &tag).unwrap();

        assert_ne!(ciphered, data);
        assert_eq!(data, deciphered.to_vec());
    }
    #[test]
    fn test_dh_x25519() {
        let (ser_pub, ser_pri) = CRYPTO.gen_x25519().unwrap();
        let (cli_pub, cli_pri) = CRYPTO.gen_x25519().unwrap();

        let cli_shared = derive_x25519(&cli_pri, &ser_pub).unwrap();
        let ser_shared = derive_x25519(&ser_pri, &cli_pub).unwrap();

        assert_eq!(cli_shared, ser_shared);
        assert_eq!(cli_shared.len(), 32);
    }
}
