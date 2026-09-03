//! Request encryption for the endpoints that require it.
//!
//! Most of UGOS accepts plain requests, but some — the file manager's v2 API,
//! `downloadCenter/download/addV2` — answer only when the payload is
//! encrypted. The scheme is the one the web UI implements:
//!
//! 1. Generate a fresh 32-character hex key per request, used as 32 raw ASCII
//!    bytes, which makes it an **AES-256** key despite the UI calling its
//!    helper `Aes128gcm`.
//! 2. Encrypt that key with the NAS's RSA public key (PKCS#1 v1.5) and send it
//!    as `X-Ugreen-Security-Code`.
//! 3. Encrypt query string and JSON body with AES-256-GCM, laid out as
//!    `base64(iv[12] || ciphertext || tag[16])`.
//! 4. Send the session token as `X-Ugreen-Token`, because the query string —
//!    where it normally rides — is now encrypted.
//! 5. Decrypt `encrypt_resp_body` with the same key.
//!
//! Verified against a live NAS on 2026-08-18, including the file manager's
//! v2 API, which never answers plain requests.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use md5::Md5;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};
use sha2::{Digest, Sha256};

use crate::error::{Result, UgosError};

/// Length of the random IV that prefixes every ciphertext.
const IV_LEN: usize = 12;
/// Length of the GCM authentication tag that suffixes it.
const TAG_LEN: usize = 16;

/// A per-request key plus the RSA-wrapped copy the NAS needs to read it.
#[derive(Debug, Clone)]
pub struct RequestKey {
    /// The 32 ASCII characters used as the AES key.
    key: [u8; 32],
    /// The same key, RSA-encrypted and base64-encoded.
    pub security_code: String,
}

impl RequestKey {
    /// Generate a key and wrap it for the given public key.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::Encryption`] if RSA encryption fails.
    pub fn generate(public_key: &RsaPublicKey) -> Result<Self> {
        let mut raw = [0u8; 16];
        rsa::rand_core::RngCore::fill_bytes(&mut rsa::rand_core::OsRng, &mut raw);
        let hex = hex::encode(raw);

        let mut key = [0u8; 32];
        key.copy_from_slice(hex.as_bytes());

        let mut rng = rsa::rand_core::OsRng;
        let wrapped = public_key
            .encrypt(&mut rng, Pkcs1v15Encrypt, hex.as_bytes())
            .map_err(|e| UgosError::Encryption(format!("wrapping request key: {e}")))?;

        Ok(Self {
            key,
            security_code: B64.encode(wrapped),
        })
    }

    /// Encrypt a string into `base64(iv || ciphertext || tag)`.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::Encryption`] if encryption fails.
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| UgosError::Encryption(format!("AES key: {e}")))?;

        let mut iv = [0u8; IV_LEN];
        rsa::rand_core::RngCore::fill_bytes(&mut rsa::rand_core::OsRng, &mut iv);
        let nonce = Nonce::from_slice(&iv);

        let sealed = cipher
            .encrypt(
                nonce,
                Payload {
                    msg: plaintext.as_bytes(),
                    aad: &[],
                },
            )
            .map_err(|e| UgosError::Encryption(format!("AES encrypt: {e}")))?;

        let mut out = Vec::with_capacity(IV_LEN + sealed.len());
        out.extend_from_slice(&iv);
        out.extend_from_slice(&sealed);
        Ok(B64.encode(out))
    }

    /// Decrypt a `base64(iv || ciphertext || tag)` payload.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::Encryption`] if the payload is malformed or the
    /// authentication tag does not verify.
    pub fn decrypt(&self, encoded: &str) -> Result<String> {
        let raw = B64
            .decode(encoded)
            .map_err(|e| UgosError::Encryption(format!("base64 decode: {e}")))?;
        if raw.len() < IV_LEN + TAG_LEN {
            return Err(UgosError::Encryption("encrypted payload too short".into()));
        }

        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| UgosError::Encryption(format!("AES key: {e}")))?;
        let nonce = Nonce::from_slice(&raw[..IV_LEN]);

        let plain = cipher
            .decrypt(
                nonce,
                Payload {
                    msg: &raw[IV_LEN..],
                    aad: &[],
                },
            )
            .map_err(|e| UgosError::Encryption(format!("AES decrypt: {e}")))?;

        String::from_utf8(plain).map_err(|e| UgosError::Encryption(format!("not UTF-8: {e}")))
    }
}

/// RSA-encrypt a value with the NAS's public key, base64-encoded.
///
/// Used for the session token, which travels encrypted in `X-Ugreen-Token`
/// rather than in the query string.
///
/// # Errors
///
/// Returns [`UgosError::Encryption`] if RSA encryption fails.
pub fn rsa_seal(public_key: &RsaPublicKey, value: &str) -> Result<String> {
    let mut rng = rsa::rand_core::OsRng;
    let sealed = public_key
        .encrypt(&mut rng, Pkcs1v15Encrypt, value.as_bytes())
        .map_err(|e| UgosError::Encryption(format!("RSA seal: {e}")))?;
    Ok(B64.encode(sealed))
}

/// Hex-encoded SHA-256, which UGOS sends alongside an encrypted body.
#[must_use]
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// Hex-encoded MD5, which UGOS wants in `X-Ugreen-Security-Key`.
///
/// # Why MD5
///
/// MD5 is broken for every purpose that needs collision resistance, and
/// static analysis flags this function accordingly. It is used here because
/// the protocol leaves no choice: UGOS derives `X-Ugreen-Security-Key` as
/// the MD5 of the session token and rejects the request otherwise. Sending
/// anything else fails with `1010, Token cannot be empty!`.
///
/// The value is not a security control on this side. It is a fixed
/// transformation of a token the server already knows, sent over TLS
/// alongside the same token in two other forms; nothing here relies on the
/// hash being hard to invert or collide. Replacing it would simply break
/// the connection.
// nosemgrep: rust.lang.security.insecure-hashes.insecure-hashes
#[must_use]
pub fn md5_hex(input: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// Build a query string the way the web UI's `qs.stringify` does.
#[must_use]
pub fn query_string(params: &[(&str, &str)]) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Percent-encode everything outside the unreserved set.
#[must_use]
pub fn urlencode_component(value: &str) -> String {
    urlencode(value)
}

/// Percent-encode everything outside the unreserved set.
fn urlencode(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                out.push('%');
                out.push(HEX[(byte >> 4) as usize] as char);
                out.push(HEX[(byte & 0x0f) as usize] as char);
            }
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let key = RequestKey {
            key: *b"0123456789abcdef0123456789abcdef",
            security_code: String::new(),
        };
        let sealed = key.encrypt(r#"{"path":"/volume1"}"#).unwrap();
        assert_eq!(key.decrypt(&sealed).unwrap(), r#"{"path":"/volume1"}"#);
    }

    #[test]
    fn layout_is_iv_ciphertext_tag() {
        let key = RequestKey {
            key: *b"0123456789abcdef0123456789abcdef",
            security_code: String::new(),
        };
        let sealed = B64.decode(key.encrypt("x").unwrap()).unwrap();
        // 12 byte IV + 1 byte payload + 16 byte tag
        assert_eq!(sealed.len(), IV_LEN + 1 + TAG_LEN);
    }

    #[test]
    fn sha256_matches_cryptojs_hex() {
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn md5_matches_reference_vectors() {
        // RFC 1321, Anhang A.5. Der Wert geht als X-Ugreen-Security-Key
        // ueber die Leitung, ein Wechsel der Hash-Bibliothek darf ihn also
        // nicht veraendern.
        assert_eq!(md5_hex(""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_hex("abc"), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn query_string_encodes_paths() {
        assert_eq!(
            query_string(&[("path", "/volume1/a b"), ("page", "1")]),
            "path=%2Fvolume1%2Fa%20b&page=1"
        );
    }
}
