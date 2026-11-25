# Deploy HTTPS certificates to UGOS

Use an administrator account and connect directly to the NAS HTTPS port. Start with `ugos-cli certificate list` to see certificate common names, expiry times and service assignments. On first contact, verify the certificate fingerprint against the NAS.

To install a new certificate, pass the leaf first, followed by its intermediate certificates in PEM format. Supply the matching unencrypted RSA or EC private key as a separate file.

```sh
ugos-cli certificate import --cert fullchain.pem --key privkey.pem
```

A successful import makes the certificate available in Control Panel. To replace an **unassigned** custom entry with the same common name, add `--replace-unused`. The CLI exports the existing certificate, chain and key into memory before deleting that entry. If the upload fails and the slot is empty, it attempts to restore the previous bundle.

## Renew the active HTTPS certificate

Install a standby with a different common name and keep a local copy of its public certificate. Choose a currently valid standby whose subject alternative names cover the NAS hostname and whose chain your HTTPS clients trust. The CLI authenticates the selected standby by its leaf fingerprint. Other clients apply their own hostname, trust and pinning rules.

UGOS must already be serving either the renewal target or the selected standby. The target must be a custom certificate used only for HTTPS or currently unassigned. The command also supports initial deployment when the standby is serving and the target has yet to be imported.

```sh
ugos-cli certificate rotate \
  --cert fullchain.pem \
  --key privkey.pem \
  --fallback-cert /etc/ugos/standby-cert.pem
```

The CLI switches the web service to the standby and verifies its live TLS fingerprint and UGOS assignment. It then replaces the target, checks the imported leaf and chain, and switches HTTPS back. Each verified handover updates the CLI's local pin. Other machines with saved pins may need an update. A reload can briefly interrupt connections.

Run certificate changes serially across your automation hosts and Control Panel. If the process is interrupted, inspect the web assignment and rerun the same command with the same files. Once the standby is serving, it can remain active until the renewal succeeds. If UGOS presents an unexpected certificate, check its fingerprint and the assignment directly on the NAS.

### Certbot deploy hook

Set `UGOS_HOST`, `UGOS_USER` and `UGOS_PASSWORD` in the hook's environment. Use the account and configuration directory holding the verified NAS pin. Limit the hook to the relevant Certbot lineage.

```sh
#!/bin/sh
set -eu
[ "$RENEWED_LINEAGE" = /etc/letsencrypt/live/nas.example.org ] || exit 0
exec ugos-cli certificate rotate \
  --cert "$RENEWED_LINEAGE/fullchain.pem" \
  --key "$RENEWED_LINEAGE/privkey.pem" \
  --fallback-cert /etc/ugos/standby-cert.pem
```

## Control Panel API

All paths are relative to `/ugreen/v1/` and require an authenticated session.

| Method | Path | Wire format |
| --- | --- | --- |
| GET | `security/ssl/list` | `result` contains the subject, issuer, Unix expiry, default flag and assigned services. |
| POST | `security/ssl/import?cover=0` | Repeated multipart field `file` in order: private key, leaf, optional chain. |
| GET | `security/ssl/export?cert=<subject>` | ZIP containing `cert.pem`, `privkey.pem` and optionally `chain.pem`. |
| DELETE | `security/ssl/cert/delete?cert=<subject>` | Removes the named, unassigned custom certificate. |
| GET | `security/ssl/getsettings` | Maps service names to assigned certificate names. |
| POST | `security/ssl/settings` | Saves scalar service assignments. The CLI changes `webserver` and forwards other settings using the Control Panel format. |

On UGOS Pro 1.21.1.0016, importing an existing common name returned code 8306, including with `cover=1`. Inventory, leaf-only and full-chain imports, export, same-name renewal and cleanup were verified on hardware using an unassigned certificate. The HTTPS handovers, rollback after a rejected upload and resumption were tested against a local server that switches its TLS certificate.
