# Security Policy

## Reporting a vulnerability

Report privately through GitHub's [Private Vulnerability
Reporting](https://github.com/metaneutrons/ugos-cli/security/advisories/new).
Do not open a public issue for anything exploitable.

You can expect an acknowledgement within **7 days** and an assessment within
**30 days**. If a fix is warranted it ships in the next release, and the
advisory is published once users have had a chance to update.

## Scope

This project talks to a NAS over the network and handles credentials, so the
following are in scope:

- Leaking the session token, password or certificate material into logs,
  error messages, or files
- Weakening or bypassing the certificate pinning described in
  [docs/api-tls.md](docs/api-tls.md)
- Anything that lets a response from the NAS cause writes outside the paths
  the user asked for

Out of scope: the strength of the UGOS API itself, which this project only
consumes, and MD5 in `X-Ugreen-Security-Key`, which the protocol imposes — see
the note at `md5_hex` in `crates/ugos-client/src/crypto.rs`.

## Supported versions

The latest release only.
