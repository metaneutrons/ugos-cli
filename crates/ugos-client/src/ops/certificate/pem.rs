//! PEM parsing and validation for certificate imports.

use base64::Engine;
use der::Decode;
use rustls::pki_types::{
    CertificateDer, PrivateKeyDer, PrivatePkcs1KeyDer, PrivatePkcs8KeyDer, PrivateSec1KeyDer,
};
use rustls::sign::{CertifiedKey, SigningKey};
use x509_cert::Certificate;

use crate::error::{Result, UgosError};

const INVALID_INPUT: &str = "certificate or private key is invalid or unsupported";

/// Parsed certificate data used by the upload operation.
pub(super) struct ValidatedChain {
    pub(super) subject: String,
    pub(super) expires: u64,
    pub(super) leaf_pem: Vec<u8>,
    pub(super) intermediates_pem: Vec<u8>,
    pub(super) fingerprint: crate::CertFingerprint,
}

struct PemBlock {
    label: String,
    der: Vec<u8>,
}

struct ParsedCertificate {
    der: Vec<u8>,
    certificate: Certificate,
}

impl ParsedCertificate {
    fn identity_and_validity(&self) -> Result<(String, std::time::Duration, std::time::Duration)> {
        let tbs = self.certificate.tbs_certificate();
        let subject = tbs
            .subject()
            .common_name()
            .map_err(|_| invalid_input())?
            .map(String::from)
            .filter(|name| !name.is_empty())
            .ok_or_else(invalid_input)?;
        Ok((
            subject,
            tbs.validity().not_before.to_unix_duration(),
            tbs.validity().not_after.to_unix_duration(),
        ))
    }
}

/// Return the subject and SHA-256 fingerprint of the first certificate.
///
/// This intentionally does not require the certificate to match a private key;
/// it is also used to identify known public certificates in local archives.
pub(super) fn identity(pem: &[u8]) -> Result<(String, crate::CertFingerprint)> {
    let certificates = parse_certificates(pem)?;
    let leaf = certificates.first().ok_or_else(invalid_input)?;
    let (subject, _, _) = leaf.identity_and_validity()?;
    Ok((subject, crate::CertFingerprint::of(&leaf.der)))
}

/// Reject a leaf certificate that has expired or is not valid yet.
pub(super) fn require_current(pem: &[u8]) -> Result<()> {
    let certificates = parse_certificates(pem)?;
    let leaf = certificates.first().ok_or_else(invalid_input)?;
    let (_, not_before, not_after) = leaf.identity_and_validity()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| UgosError::Refused("certificate validity cannot be checked".to_owned()))?;
    if now < not_before || now >= not_after {
        return Err(UgosError::Refused(
            "certificate is expired or not yet valid".to_owned(),
        ));
    }
    Ok(())
}

/// Validate a leaf-first certificate chain and its one unencrypted RSA or EC key.
pub(super) fn validate(chain_pem: &[u8], key_pem: &[u8]) -> Result<ValidatedChain> {
    let certificates = parse_certificates(chain_pem)?;
    let leaf = certificates.first().ok_or_else(invalid_input)?;
    let (subject, _, expires) = leaf.identity_and_validity()?;
    let key = load_private_key(key_pem)?;

    let rustls_chain = certificates
        .iter()
        .map(|certificate| CertificateDer::from(certificate.der.clone()))
        .collect();
    let certified_key = CertifiedKey::new(rustls_chain, key);
    certified_key.keys_match().map_err(|_| invalid_input())?;

    let leaf_pem = encode_certificate(&leaf.der);
    let intermediates_pem = certificates
        .iter()
        .skip(1)
        .flat_map(|certificate| encode_certificate(&certificate.der))
        .collect();

    Ok(ValidatedChain {
        subject,
        expires: expires.as_secs(),
        leaf_pem,
        intermediates_pem,
        fingerprint: crate::CertFingerprint::of(&leaf.der),
    })
}

fn invalid_input() -> UgosError {
    UgosError::Refused(INVALID_INPUT.to_owned())
}

fn parse_certificates(pem: &[u8]) -> Result<Vec<ParsedCertificate>> {
    let blocks = parse_pem(pem, &["CERTIFICATE"])?;
    let mut certificates = Vec::with_capacity(blocks.len());

    for block in blocks {
        let certificate = Certificate::from_der(&block.der).map_err(|_| invalid_input())?;
        certificates.push(ParsedCertificate {
            der: block.der,
            certificate,
        });
    }

    Ok(certificates)
}

fn load_private_key(pem: &[u8]) -> Result<std::sync::Arc<dyn SigningKey>> {
    let mut blocks = parse_pem(pem, &["PRIVATE KEY", "RSA PRIVATE KEY", "EC PRIVATE KEY"])?;
    if blocks.len() != 1 {
        return Err(invalid_input());
    }

    let block = blocks.pop().ok_or_else(invalid_input)?;
    let key_der = match block.label.as_str() {
        "PRIVATE KEY" => PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(block.der)),
        "RSA PRIVATE KEY" => PrivateKeyDer::Pkcs1(PrivatePkcs1KeyDer::from(block.der)),
        "EC PRIVATE KEY" => PrivateKeyDer::Sec1(PrivateSec1KeyDer::from(block.der)),
        _ => return Err(invalid_input()),
    };

    let provider = rustls::crypto::ring::default_provider();
    let key = provider
        .key_provider
        .load_private_key(key_der)
        .map_err(|_| invalid_input())?;
    if !matches!(
        key.algorithm(),
        rustls::SignatureAlgorithm::RSA | rustls::SignatureAlgorithm::ECDSA
    ) {
        return Err(invalid_input());
    }

    Ok(key)
}

fn parse_pem(pem: &[u8], allowed_labels: &[&str]) -> Result<Vec<PemBlock>> {
    let text = std::str::from_utf8(pem).map_err(|_| invalid_input())?;
    let mut lines = text.lines();
    let mut blocks = Vec::new();

    while let Some(line) = next_nonblank_line(&mut lines) {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let label = line
            .strip_prefix("-----BEGIN ")
            .and_then(|line| line.strip_suffix("-----"))
            .filter(|label| allowed_labels.contains(label))
            .ok_or_else(invalid_input)?;
        let footer = format!("-----END {label}-----");
        let mut encoded = Vec::new();
        let mut found_footer = false;

        for line in lines.by_ref() {
            let line = line.strip_suffix('\r').unwrap_or(line);
            if line == footer {
                found_footer = true;
                break;
            }
            if line.starts_with("-----END ") || line.starts_with("-----BEGIN ") {
                return Err(invalid_input());
            }
            for byte in line.bytes() {
                if !byte.is_ascii_whitespace() {
                    if !byte.is_ascii() {
                        return Err(invalid_input());
                    }
                    encoded.push(byte);
                }
            }
        }

        if !found_footer || encoded.is_empty() {
            return Err(invalid_input());
        }
        let der = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| invalid_input())?;
        if der.is_empty() {
            return Err(invalid_input());
        }
        blocks.push(PemBlock {
            label: label.to_owned(),
            der,
        });
    }

    if blocks.is_empty() {
        return Err(invalid_input());
    }
    Ok(blocks)
}

fn next_nonblank_line<'a>(lines: &mut impl Iterator<Item = &'a str>) -> Option<&'a str> {
    lines.find(|line| {
        let line = line.strip_suffix('\r').unwrap_or(line);
        !line.bytes().all(|byte| byte.is_ascii_whitespace())
    })
}

fn encode_certificate(der: &[u8]) -> Vec<u8> {
    let encoded = base64::engine::general_purpose::STANDARD.encode(der);
    let mut pem = b"-----BEGIN CERTIFICATE-----\n".to_vec();
    for line in encoded.as_bytes().chunks(64) {
        pem.extend_from_slice(line);
        pem.push(b'\n');
    }
    pem.extend_from_slice(b"-----END CERTIFICATE-----\n");
    pem
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET_MARKER: &str = "SENSITIVE_PRIVATE_KEY_CONTENT";

    fn assert_generic_error<T>(result: Result<T>, secret: &str) {
        let error = result
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
        assert_eq!(error, INVALID_INPUT);
        assert!(!error.contains(secret));
    }

    fn fixture(label: &str, payload: &str) -> String {
        format!("-----BEGIN {label}-----\n{payload}\n-----END {label}-----\n")
    }

    #[test]
    fn rejects_malformed_certificate_without_echoing_input() {
        let pem = fixture("CERTIFICATE", SECRET_MARKER);
        assert_generic_error(identity(pem.as_bytes()), SECRET_MARKER);
    }

    #[test]
    fn rejects_malformed_private_key_without_echoing_input() {
        let pem = fixture("PRIVATE KEY", SECRET_MARKER);
        assert_generic_error(load_private_key(pem.as_bytes()), SECRET_MARKER);
    }

    #[test]
    fn rejects_invalid_private_key_der_without_echoing_input() {
        let pem = fixture("PRIVATE KEY", "U0VDUkVU");
        assert_generic_error(load_private_key(pem.as_bytes()), "SECRET");
    }

    #[test]
    fn rejects_private_key_bytes_with_certificate_label() {
        let pem = fixture("CERTIFICATE", SECRET_MARKER);
        assert_generic_error(load_private_key(pem.as_bytes()), SECRET_MARKER);
    }

    #[test]
    fn rejects_private_key_pem_as_certificate_input() {
        let pem = fixture("PRIVATE KEY", SECRET_MARKER);
        assert_generic_error(identity(pem.as_bytes()), SECRET_MARKER);
    }

    #[test]
    fn rejects_multiple_private_keys() {
        let one = fixture("PRIVATE KEY", "U0VDUkVU");
        let pem = format!("{one}{one}");
        assert_generic_error(load_private_key(pem.as_bytes()), "SECRET");
    }
}
