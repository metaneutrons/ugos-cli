//! Certificate pinning on first use.
//!
//! UGOS devices serve a self-signed certificate, so ordinary chain
//! validation cannot succeed. Accepting any certificate unconditionally
//! would leave the connection open to an active man-in-the-middle, who
//! could also answer `verify/check` and hand over an RSA key of their own
//! choosing — which would expose the password despite it being encrypted.
//!
//! This module takes the approach SSH takes: remember the certificate the
//! first time a host is seen and refuse anything else afterwards. The first
//! connection remains trust-on-first-use and is only as safe as the network
//! it happens on; every later one is authenticated.
//!
//! # Why the public key is handled by hand
//!
//! UGOS serves an **X.509 version 1** certificate. webpki rejects v1
//! outright, so the usual `verify_tls*_signature` helpers cannot be used —
//! they parse the certificate before checking anything and fail with
//! `UnsupportedCertVersion`. The handshake signature still has to be
//! verified, otherwise pinning would prove nothing: a certificate is public,
//! and anyone could replay one they copied. So the public key is lifted out
//! of the certificate directly and passed to rustls' raw-key entry point.
//!
//! Extracting it is safe despite being hand-rolled, because the fingerprint
//! of the whole DER is checked first. A mis-parse cannot weaken the check;
//! it can only produce a key that fails to verify, which aborts the
//! connection.
//!
//! Only TLS 1.3 is offered. rustls has no raw-key equivalent for the TLS 1.2
//! signature path, and the devices tested negotiate 1.3 anyway.

pub mod known_hosts;

use std::sync::{Arc, Mutex};

use der::{Reader, SliceReader, Tag, TagNumber};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, ring, verify_tls13_signature_with_raw_key};
use rustls::pki_types::{CertificateDer, ServerName, SubjectPublicKeyInfoDer, UnixTime};
use rustls::{DigitallySignedStruct, Error as RustlsError, SignatureScheme};
use sha2::{Digest, Sha256};

use crate::error::{Result, UgosError};

/// SHA-256 fingerprint of a server's leaf certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertFingerprint([u8; 32]);

impl CertFingerprint {
    /// Compute the fingerprint of a DER-encoded certificate.
    #[must_use]
    pub fn of(der: &[u8]) -> Self {
        let digest = Sha256::digest(der);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&digest);
        Self(bytes)
    }

    /// Render as lowercase hex, the form used in the on-disk store.
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse from the lowercase hex form.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::Encryption`] if the input is not 64 hex digits.
    pub fn from_hex(input: &str) -> Result<Self> {
        let decoded = hex::decode(input)
            .map_err(|e| UgosError::Encryption(format!("bad fingerprint: {e}")))?;
        let bytes: [u8; 32] = decoded
            .try_into()
            .map_err(|_| UgosError::Encryption("fingerprint must be 32 bytes".into()))?;
        Ok(Self(bytes))
    }
}

impl std::fmt::Display for CertFingerprint {
    /// Groups bytes in colon-separated pairs, as OpenSSH prints them.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hex = hex::encode(self.0);
        let pairs: Vec<&str> = (0..hex.len()).step_by(2).map(|i| &hex[i..i + 2]).collect();
        write!(f, "{}", pairs.join(":"))
    }
}

/// How the TLS certificate of the NAS is treated.
#[derive(Debug, Clone)]
pub enum TlsPolicy {
    /// Accept any certificate without checking it.
    ///
    /// Offers no protection against an active man-in-the-middle. Reserved
    /// for the case where a user knowingly opts out.
    Insecure,
    /// Accept only a certificate with this exact fingerprint.
    Pinned(CertFingerprint),
}

/// Verifier that accepts one specific certificate.
#[derive(Debug)]
struct PinnedVerifier {
    expected: CertFingerprint,
    provider: Arc<CryptoProvider>,
}

/// Verifier that accepts anything and records what it saw.
#[derive(Debug)]
struct LearningVerifier {
    seen: Arc<Mutex<Option<CertFingerprint>>>,
    provider: Arc<CryptoProvider>,
}

/// Lift the `SubjectPublicKeyInfo` out of a DER-encoded certificate.
///
/// Walks the ASN.1 structure rather than using a certificate parser, because
/// every such parser in reach rejects the version 1 certificates UGOS
/// serves. Handles both v1 (no version field) and v3.
///
/// # Errors
///
/// Returns [`UgosError::Encryption`] if the DER does not have the shape of a
/// certificate.
fn extract_spki(cert_der: &[u8]) -> Result<Vec<u8>> {
    fn parse(cert_der: &[u8]) -> std::result::Result<Vec<u8>, der::Error> {
        let mut outer = SliceReader::new(cert_der)?;
        outer.sequence(|certificate| {
            let spki = certificate.sequence(|tbs| {
                // A v3 certificate opens with [0] EXPLICIT version; v1 omits
                // it and starts at the serial number.
                let version_tag = Tag::ContextSpecific {
                    constructed: true,
                    number: TagNumber::N0,
                };
                if tbs.peek_tag()? == version_tag {
                    let _version = tbs.tlv_bytes()?;
                }
                let _serial = tbs.tlv_bytes()?;
                let _signature_alg = tbs.tlv_bytes()?;
                let _issuer = tbs.tlv_bytes()?;
                let _validity = tbs.tlv_bytes()?;
                let _subject = tbs.tlv_bytes()?;
                let spki = tbs.tlv_bytes()?.to_vec();
                // v3 carries optional fields after the key; drain them so the
                // reader finishes cleanly.
                while !tbs.is_finished() {
                    let _rest = tbs.tlv_bytes()?;
                }
                Ok(spki)
            })?;
            // signatureAlgorithm and signatureValue follow the TBS block.
            while !certificate.is_finished() {
                let _rest = certificate.tlv_bytes()?;
            }
            Ok(spki)
        })
    }

    parse(cert_der)
        .map_err(|e| UgosError::Encryption(format!("cannot read certificate public key: {e}")))
}

/// Verify a TLS 1.3 handshake signature against a certificate's own key.
fn verify_with_cert_key(
    cert: &CertificateDer<'_>,
    message: &[u8],
    dss: &DigitallySignedStruct,
    provider: &CryptoProvider,
) -> std::result::Result<HandshakeSignatureValid, RustlsError> {
    let spki = extract_spki(cert.as_ref()).map_err(|e| RustlsError::General(format!("{e}")))?;
    verify_tls13_signature_with_raw_key(
        message,
        &SubjectPublicKeyInfoDer::from(spki.as_slice()),
        dss,
        &provider.signature_verification_algorithms,
    )
}

/// The TLS 1.2 path, which rustls offers no raw-key equivalent for.
fn tls12_unsupported() -> RustlsError {
    RustlsError::General(
        "TLS 1.2 is not supported against a version 1 certificate; the client offers TLS 1.3 only"
            .into(),
    )
}

impl ServerCertVerifier for PinnedVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, RustlsError> {
        let actual = CertFingerprint::of(end_entity.as_ref());
        if actual == self.expected {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(RustlsError::General(format!(
                "certificate fingerprint mismatch: expected {}, got {actual}",
                self.expected
            )))
        }
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, RustlsError> {
        Err(tls12_unsupported())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, RustlsError> {
        // Checked again here rather than trusting the earlier call: this is
        // what binds the key used below to the certificate that was pinned.
        let actual = CertFingerprint::of(cert.as_ref());
        if actual != self.expected {
            return Err(RustlsError::General(
                "certificate changed mid-handshake".into(),
            ));
        }
        verify_with_cert_key(cert, message, dss, &self.provider)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

impl ServerCertVerifier for LearningVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, RustlsError> {
        // Auch bei vergiftetem Mutex eintragen. Wurde die Beobachtung sonst
        // verworfen, meldete der Aufrufer "no certificate seen during
        // handshake" und zeigte damit auf den Handshake statt auf den Panic
        // in einem anderen Thread, der die eigentliche Ursache war.
        {
            let mut slot = self
                .seen
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *slot = Some(CertFingerprint::of(end_entity.as_ref()));
        }
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, RustlsError> {
        Err(tls12_unsupported())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, RustlsError> {
        // Verified even while learning: pinning a certificate whose private
        // key the peer does not hold would record the wrong thing.
        verify_with_cert_key(cert, message, dss, &self.provider)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Build a rustls configuration from a verifier.
///
/// Restricted to TLS 1.3, the only version whose signature check can be done
/// against a raw key.
///
/// # Errors
///
/// Returns [`UgosError::Encryption`] if the provider rejects the version.
fn config_with(verifier: Arc<dyn ServerCertVerifier>) -> Result<rustls::ClientConfig> {
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(ring::default_provider()))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| UgosError::Encryption(format!("TLS 1.3 unavailable: {e}")))?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    Ok(config)
}

/// Build an HTTP client that enforces the given TLS policy.
///
/// # Errors
///
/// Returns [`UgosError::Encryption`] if the client cannot be built.
pub fn http_client(policy: &TlsPolicy) -> Result<reqwest::Client> {
    let builder = reqwest::Client::builder()
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(30));

    let builder = match policy {
        TlsPolicy::Insecure => builder.danger_accept_invalid_certs(true),
        TlsPolicy::Pinned(expected) => {
            let verifier = Arc::new(PinnedVerifier {
                expected: *expected,
                provider: Arc::new(ring::default_provider()),
            });
            builder.use_preconfigured_tls(config_with(verifier)?)
        }
    };

    builder
        .build()
        .map_err(|e| UgosError::Encryption(format!("HTTP client build: {e}")))
}

/// Connect once and report the certificate the host presented.
///
/// Used on first contact, before anything is pinned. The result is only
/// trustworthy insofar as the network at that moment is.
///
/// # Errors
///
/// Returns [`UgosError::Http`] if the host cannot be reached, and
/// [`UgosError::Encryption`] if no certificate was observed.
pub async fn probe_fingerprint(host: &str, port: u16) -> Result<CertFingerprint> {
    let seen = Arc::new(Mutex::new(None));
    let verifier = Arc::new(LearningVerifier {
        seen: Arc::clone(&seen),
        provider: Arc::new(ring::default_provider()),
    });
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .use_preconfigured_tls(config_with(verifier)?)
        .build()
        .map_err(|e| UgosError::Encryption(format!("HTTP client build: {e}")))?;

    // Any endpoint will do; only the handshake matters. A 404 is fine.
    let _ = client.get(format!("https://{host}:{port}/")).send().await?;

    let observed = seen
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    observed.ok_or_else(|| UgosError::Encryption("no certificate seen during handshake".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_round_trips_through_hex() {
        let fp = CertFingerprint::of(b"some certificate bytes");
        let parsed = CertFingerprint::from_hex(&fp.to_hex());
        assert_eq!(parsed.ok(), Some(fp));
    }

    #[test]
    fn hex_form_is_64_lowercase_digits() {
        let hex = CertFingerprint::of(b"x").to_hex();
        assert_eq!(hex.len(), 64);
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
        );
    }

    #[test]
    fn display_form_is_colon_separated() {
        let shown = CertFingerprint::of(b"x").to_string();
        assert_eq!(shown.matches(':').count(), 31);
        assert_eq!(shown.len(), 95);
    }

    #[test]
    fn differing_certificates_differ() {
        assert_ne!(CertFingerprint::of(b"a"), CertFingerprint::of(b"b"));
    }

    #[test]
    fn from_hex_rejects_wrong_length() {
        assert!(CertFingerprint::from_hex("abcd").is_err());
    }

    #[test]
    fn from_hex_rejects_non_hex() {
        assert!(CertFingerprint::from_hex(&"z".repeat(64)).is_err());
    }

    // Minimal synthetic certificates. Only the ASN.1 shape matters to
    // `extract_spki`, so these carry the structure without being valid
    // certificates — and without tying the tests to a real device.
    const V1_CERT: &str = "302d30250201013000300030003000301830032a8648031100ababababababab                           ababababababababab3000030200ff";
    const V3_CERT: &str = "3036302ea0030201020201013000300030003000301830032a8648031100abab                           ababababababababababababababa30230003000030200ff";
    const SPKI: &str = "301830032a8648031100abababababababababababababababab";

    fn der(hex_str: &str) -> Vec<u8> {
        hex::decode(hex_str.replace(' ', "")).unwrap_or_default()
    }

    #[test]
    fn extracts_key_from_a_version_1_certificate() {
        // The case that matters: UGOS serves v1, which has no version field.
        let spki = extract_spki(&der(V1_CERT));
        assert_eq!(spki.map(hex::encode).ok(), Some(SPKI.to_string()));
    }

    #[test]
    fn extracts_key_from_a_version_3_certificate() {
        // v3 puts an explicit [0] version first and extensions after the key.
        let spki = extract_spki(&der(V3_CERT));
        assert_eq!(spki.map(hex::encode).ok(), Some(SPKI.to_string()));
    }

    #[test]
    fn both_versions_yield_the_same_key() {
        assert_eq!(
            extract_spki(&der(V1_CERT)).ok(),
            extract_spki(&der(V3_CERT)).ok()
        );
    }

    #[test]
    fn rejects_input_that_is_not_a_certificate() {
        assert!(extract_spki(b"not der at all").is_err());
        assert!(extract_spki(&[]).is_err());
        assert!(extract_spki(&der("3000")).is_err());
    }

    #[test]
    fn rejects_a_truncated_certificate() {
        let full = der(V1_CERT);
        assert!(extract_spki(&full[..full.len() / 2]).is_err());
    }
}
