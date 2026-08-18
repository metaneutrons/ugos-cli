# Transport security

The NAS is reached over HTTPS on port 9443. This note records how its
certificate is handled and why the extra layer UGOS offers on top of TLS is
not used for everything.

## The certificate is X.509 version 1

UGOS serves a self-signed certificate issued by `C=CN, ST=GD, L=SZ,
O=UGREEN, OU=UGREEN, CN=UGREEN`, valid for one year, with a 2048-bit RSA
key — and **version 1**, which has not been current since 1996.

That has a concrete consequence. webpki, the certificate parser behind
rustls, rejects v1 outright, because v1 predates the extensions that carry
subject alternative names and key usage. Every ordinary verification path
therefore fails with `UnsupportedCertVersion` before any check happens.

## Trust on first use

Releases up to 0.8 set `danger_accept_invalid_certs(true)`, which accepts
any certificate at all. Since 0.9 the client instead remembers the
certificate it sees on first contact and refuses anything else afterwards,
as SSH does with host keys. Fingerprints live in the user's config
directory as `known_hosts.json`, keyed by `host:port`, and are shared by the
CLI and the MCP server.

- First contact prints the SHA-256 fingerprint and records it.
- Later connections must present the same certificate.
- `--tls-trust-new` records a changed certificate, for renewals and
  reinstalls.
- `--tls-insecure` skips the check entirely.

The first connection is only as trustworthy as the network it happens on.
That is inherent to the approach; the fingerprint is printed so it can be
compared against the device.

## Verifying the handshake by hand

Pinning a fingerprint alone would prove nothing. A certificate is public,
so anyone can present a copy of one. What separates the real host from an
impostor is the private key, which the peer proves it holds by signing the
handshake.

Because webpki will not parse a v1 certificate, that signature cannot be
checked through the usual helper. The client instead lifts the
`SubjectPublicKeyInfo` out of the certificate by walking its ASN.1 structure
and passes it to rustls' raw-key entry point,
`verify_tls13_signature_with_raw_key`.

Hand-written parsing is acceptable here only because of the order things
happen in. The fingerprint of the entire DER is checked first, so the bytes
the key is read from are already known to be exactly the pinned ones. A
mis-parse cannot weaken the check — it can only produce a key that fails to
verify, which aborts the connection.

Only TLS 1.3 is offered, since rustls has no raw-key equivalent on the TLS
1.2 signature path. The devices tested negotiate 1.3 regardless.

## Why requests are not all encrypted

UGOS can wrap request bodies in AES-GCM under an RSA-wrapped key (see
[api-encryption.md](api-encryption.md)). Measured against a live NAS, every
JSON endpoint accepts that wrapping transparently — 14 endpoints across
KVM, Docker, sysinfo, log and user behaved identically encrypted and in the
clear, errors included, at a cost of roughly 11 ms per call.

It is nonetheless used only where an endpoint demands it. The wrapping sits
*inside* TLS, so it adds nothing against a passive eavesdropper that TLS
does not already handle. Against an active man-in-the-middle it also adds
nothing, because the same attacker answers `verify/check` and thereby
chooses the RSA key that gets used. Certificate pinning is what closes that
gap, which is why the effort went there.

Three things cannot be wrapped at all:

- `verify/check` hands out the RSA key the scheme needs, so it cannot use it.
- `verify/login` has no session token yet, which the scheme requires in three
  places at once. The password is separately RSA-encrypted there regardless.
- Multipart uploads and binary downloads carry byte streams, and the scheme
  operates on query strings and JSON envelopes.
