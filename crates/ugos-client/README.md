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

## Licence

GPL-3.0-or-later.
