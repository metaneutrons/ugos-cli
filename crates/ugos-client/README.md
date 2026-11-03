# ugos-client

Rust client library for the API that the UGREEN NAS web interface uses (UGOS).

Covers authentication, KVM virtual machines, snapshots, networks, storage,
images, Docker, the file manager, the download centre, filesystem snapshots,
system monitoring, logs and users.

Two things about UGOS shape this client and are worth knowing before you use it:

- The device serves an **X.509 version 1 certificate**, which every ordinary
  verifier rejects. The library therefore offers trust on first use with a
  pinned certificate fingerprint, SSH style, alongside an explicit insecure
  mode. TLS 1.3 only.
- Request encryption exists in UGOS, but the web interface uses it **only when
  the connection is not HTTPS**. It is a substitute for TLS, not an addition to
  it, and this client treats it as such.

```rust
use ugos_client::{Credentials, TlsPolicy, UgosClient};
```

Part of [ugos-cli](https://github.com/metaneutrons/ugos-cli). See that
repository for the command line tool, the MCP server and the full
documentation.

## Two-factor authentication

Put an authenticator code on `Credentials::otp`, or a TOTP secret on
`Credentials::otp_secret` (base32, or an `otpauth://totp/` URI). Only
authenticator apps (type 1) are supported. The secret is turned into a fresh
code on every login, which is what re-authentication after the ~25 minute
session needs. The one-shot code is spent on the first login.

The library does not read environment variables. The CLI and `ugos-mcp` copy
`UGOS_OTP` and `UGOS_OTP_SECRET` onto `Credentials`.

With the password and the secret in one process, the second factor no longer
protects that machine against whoever controls it. It still protects against a
leaked password. UGOS shows the secret only once, as a QR code when
two-factor authentication is enabled. The `/v1/verify/code/login` request
follows the web client and has not been confirmed against a live capture.

## Errors

`UgosError` is `#[non_exhaustive]`. Match the variants you handle and keep a
`_` arm for the rest, so a new error kind does not break your build. `Io`
covers filesystem failures and is deliberately separate from `Encryption`.

## Licence

GPL-3.0-or-later, deliberately, where a library would usually carry the LGPL.
The crate serves the programs of its own project, so the weaker copyleft would
add nothing, and every release so far has been GPL.
