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
5. **publish-release** — makes the draft visible. This has to happen before
   the package channels, not after: Homebrew and AUR download from the release
   URL in their own tests, and a draft release's assets answer 404 without
   authentication.
6. **update-homebrew**, **publish-apt**, **publish-aur** — publish and verify.
   The Homebrew step opens a pull request on the tap, waits for its
   `Formula qualification` check and merges it; `main` there is protected and
   rejects a direct push. The APT step dispatches to the archive and then waits
   for `deb.metaneutrons.cc/ugos-cli` to actually serve the version, because a
   dispatch returns no run id and the served archive is the stronger statement
   anyway. It finishes by comparing the published `.deb` byte for byte against
   the release asset.

`.deb` packaging is done by a script under `scripts/release/`, so it can be run
and tested locally rather than only inside CI.

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
