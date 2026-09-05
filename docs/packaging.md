# Packaging and release channels

`ugos-cli` is published through several channels, all built from the same
release archives and verified after publication rather than assumed to have
worked. Two of them are live today.

| Channel | Artefact | State | Verified by |
|---------|----------|-------|-------------|
| GitHub release | `.tar.gz`, `.zip`, `.deb` | live | the build matrix itself |
| Homebrew | formula in `metaneutrons/homebrew-tap` | live | pull request, tap qualification, re-read after merge |
| APT | signed repository under `deb.metaneutrons.cc` | waiting on a token | the archive verifies our attestation, we compare the published `.deb` against the release asset |
| AUR | `ugos-cli-bin` | package does not exist | package built and compared against the release archive |

The APT channel is served by `metaneutrons/apt-archive`, the central repository
that renders and signs `deb.metaneutrons.cc`. This repository holds no signing
key and no R2 write access: an R2 token can be scoped to a bucket but not to a
prefix, so write access for one project would be write access to the whole
archive.

Instead the `.deb` packages are attested with `actions/attest-build-provenance`
and attached to the release, and the pipeline asks the archive to publish. The
archive fetches them, verifies the attestation with `gh attestation verify` and
aborts without it, deliberately: an account that may upload an asset is no
statement about what built the file. Signing happens there, with the domain
subkey; the certify-only primary key stays offline.

## Installing

```bash
# Homebrew (macOS, Linux)
brew install metaneutrons/tap/ugos-cli

# Debian, Ubuntu: the .deb from the release, until the APT repository serves
sudo apt install ./ugos-cli_<version>_amd64.deb
```

## How the release runs

`release-please` tags a version, which calls `release.yml`. Nine stages, each a
precondition of the next:

1. **metadata** — derives the release core from the tag, checks it against
   `Cargo.toml`, verifies the tag points at the commit the run builds from, and
   aborts when the tag already carries a published release. No environment and
   no secrets: this stage has to run for a prerelease tag too.
2. **build** — six targets, producing the release archives.
3. **package-deb** — builds `.deb` packages from those archives and installs the
   native one to confirm it works.
4. **sign** — one SPDX SBOM and one cosign bundle per payload, keyless through
   the run's OIDC identity, plus a GitHub build attestation for every payload.
5. **aggregate** — requires payload, SBOM and bundle for each of the eight
   payloads and verifies every signature and attestation. An intermediate gate.
6. **channel-metadata** — generates the Homebrew formula, the PKGBUILD and the
   `.SRCINFO` from the qualified payloads, builds the Arch package and compares
   it against the release archive, then signs and attests the three files.
7. **candidate** — forms the final asset list from the payloads and the channel
   metadata actually produced, verifies everything once more, and only then
   writes `SHA256SUMS` over exactly that list. No later stage adds or changes an
   asset.
8. **channel-preflight** — checks the channel accesses without publishing
   anything. A prerelease tag skips this deliberately, since it never reaches
   the channels.
9. **stage**, then the channels, then **promote** — the assets go onto the
   draft, are read back and compared byte for byte in both directions, and the
   release becomes visible as a **prerelease**, without `latest`. Visible it
   must be, because Homebrew and AUR download from the release URL in their own
   tests and a draft's assets answer 404. As a prerelease nobody installs it
   automatically. Only once every configured channel has succeeded does
   `promote` set `latest`, and the assets are not touched in the process.

That order is the point: a channel that fails leaves a prerelease behind that
nobody installs, instead of a half-published stable release.

### What a release carries

Per payload the payload itself, `<payload>.spdx.json` and
`<payload>.sigstore.json`, keeping the full payload name so archive and package
of one architecture cannot collide. Per channel metadata file the file and its
bundle. Plus `SHA256SUMS` over all of them. Eight payloads and three metadata
files make 31 assets.

## Reproducibility

`SOURCE_DATE_EPOCH` is taken from the tagged commit and threaded through
`dpkg-deb` and `gzip -n`, so rebuilding a tag produces byte-identical packages.
That is what lets the APT step compare the published file against the release
asset with `cmp` instead of trusting a version string.

Archive metadata is no longer built here, so the old key-age trap (`UG7405`)
is gone with it: signing pinned the clock to `SOURCE_DATE_EPOCH`, and GPG
refuses to sign with a key created after that moment. The archive sets its own
publication time, and its `Valid-Until` follows from that rather than from a
tag's age.

## Configuration for the release itself

The APT and AUR channels are skipped when their configuration is absent, so
the release still succeeds without them.

### Secrets

| Name | Channel | Notes |
|------|---------|-------|
| `HOMEBREW_TAP_TOKEN` | Homebrew | needs push rights on `metaneutrons/homebrew-tap` |
| `APT_ARCHIVE_DISPATCH_TOKEN` | APT | fine-grained token for `metaneutrons/apt-archive` alone, `contents: write`, which is what `repository_dispatch` requires |
| `AUR_SSH_PRIVATE_KEY` | AUR | key registered with the AUR account |

### Variables

| Name | Channel | Example |
|------|---------|---------|
| `AUR_SSH_KNOWN_HOSTS` | AUR | output of `ssh-keyscan aur.archlinux.org` |

`AUR_SSH_KNOWN_HOSTS` is pinned rather than trusted on first use, so the
push cannot be redirected by a changed host key.

There is deliberately no APT signing key and no R2 credential here. Everything
the archive needs sits in its own protected environment, and the only thing
this repository may do is knock.

## Testing the packaging locally

`build-deb.sh` runs outside CI. On a machine without `dpkg-deb`, a container
works:

```bash
tar czf - archives scripts | docker run --rm -i debian:bookworm-slim bash -c '
  mkdir /work && cd /work && tar xzf -
  apt-get update -qq && apt-get install -y -qq dpkg-dev
  scripts/release/build-deb.sh --archive archives/ugos-cli-x86_64-unknown-linux-gnu.tar.gz \
    --arch amd64 --version 0.11.3 --output-dir debs --source-date-epoch "$(date +%s)"
'
```

Archive rendering and signing are not testable from here any more; they live in
`metaneutrons/apt-archive` and are covered by its own contract suites.
