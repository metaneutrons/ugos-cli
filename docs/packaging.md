# Packaging and release channels

`ugos-cli` is published through four channels. Each is built from the same
release archives, and each is verified after publication rather than assumed
to have worked.

| Channel | Artefact | Verified by |
|---------|----------|-------------|
| GitHub release | `.tar.gz`, `.zip`, `.deb` | the build matrix itself |
| Homebrew | formula in `metaneutrons/homebrew-tap` | re-read from the tap after push |
| APT | signed repository on Cloudflare R2 | `gpgv` plus a byte compare of the published `.deb` |
| AUR | `ugos-cli-bin` | package built and compared against the release archive |

## Installing

```bash
# Homebrew (macOS, Linux)
brew install metaneutrons/tap/ugos-cli

# Debian, Ubuntu
curl -fsSL https://deb.metaneutrons.cc/ugos-cli-archive-keyring.asc \
  | sudo gpg --dearmor -o /usr/share/keyrings/ugos-cli.gpg
echo "deb [signed-by=/usr/share/keyrings/ugos-cli.gpg] https://deb.metaneutrons.cc stable main" \
  | sudo tee /etc/apt/sources.list.d/ugos-cli.list
sudo apt update && sudo apt install ugos-cli

# Arch Linux
yay -S ugos-cli-bin
```

## How the release runs

`release-please` tags a version, which calls `release.yml`. That workflow
runs in this order:

1. **preflight** — derives the version from the tag, checks it against
   `Cargo.toml`, confirms the Homebrew credential can actually write to the
   tap, and decides which package channels are configured. A channel that is
   only *partly* configured fails here rather than halfway through
   publication.
2. **build** — six targets, producing the release archives.
3. **package-deb** — builds `.deb` packages from those archives and installs
   the native one to confirm it works.
4. **upload-release-artifacts** — attaches everything to the GitHub release.
5. **publish-apt**, **update-homebrew**, **publish-aur** — publish and verify.

Both `.deb` packaging and the APT repository are built by scripts under
`scripts/release/`, so they can be run and tested locally rather than only
inside CI.

## Reproducibility

`SOURCE_DATE_EPOCH` is taken from the tagged commit and threaded through
`dpkg-deb`, `gzip -n` and the GPG signatures, so rebuilding a tag produces
byte-identical packages and metadata.

One consequence is worth knowing: signing pins the clock to that timestamp,
and GPG refuses to sign with a key created *after* it. Rotating the signing
key and then re-running an older tag therefore fails, deliberately and with
an explicit message (`UG7405`) rather than an opaque "Unusable secret key".

## Configuration for the release itself

The APT and AUR channels are skipped when their configuration is absent, so
the release still succeeds without them.

### Secrets

| Name | Channel | Notes |
|------|---------|-------|
| `HOMEBREW_TAP_TOKEN` | Homebrew | needs push rights on `metaneutrons/homebrew-tap` |
| `APT_GPG_PRIVATE_KEY` | APT | ASCII-armoured private key, `gpg --armor --export-secret-keys` |
| `APT_GPG_PASSPHRASE` | APT | passphrase for that key |
| `R2_ACCESS_KEY_ID` | APT | R2 token scoped to the bucket |
| `R2_SECRET_ACCESS_KEY` | APT | |
| `AUR_SSH_PRIVATE_KEY` | AUR | key registered with the AUR account |

### Variables

| Name | Channel | Example |
|------|---------|---------|
| `APT_GPG_FINGERPRINT` | APT | full 40-hex primary key fingerprint |
| `APT_PUBLIC_BASE_URL` | APT | `https://deb.metaneutrons.cc`, no trailing slash |
| `R2_ACCOUNT_ID` | APT | 32-hex Cloudflare account id |
| `R2_BUCKET_NAME` | APT | bucket serving that domain |
| `AUR_SSH_KNOWN_HOSTS` | AUR | output of `ssh-keyscan aur.archlinux.org` |

The bucket is served at the root of `APT_PUBLIC_BASE_URL`: `dists/` and
`pool/` sit directly beneath it, next to the public keyring.

`AUR_SSH_KNOWN_HOSTS` is pinned rather than trusted on first use, so the
push cannot be redirected by a changed host key.

## Testing the packaging locally

Both scripts run outside CI. On a machine without `dpkg-deb`, a container
works:

```bash
tar czf - archives scripts | docker run --rm -i debian:bookworm-slim bash -c '
  mkdir /work && cd /work && tar xzf -
  apt-get update -qq && apt-get install -y -qq dpkg-dev apt-utils gnupg
  scripts/release/build-deb.sh --archive archives/ugos-cli-x86_64-unknown-linux-gnu.tar.gz \
    --arch amd64 --version 0.10.1 --output-dir debs --source-date-epoch "$(date +%s)"
'
```

This is how the pipeline was validated before it first ran: the packages were
built, a throwaway key signed a repository, and `apt install ugos-cli`
installed it from a `file://` source.
