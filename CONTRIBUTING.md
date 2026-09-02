# Contributing

## Commits

[Conventional Commits](https://www.conventionalcommits.org/) are mandatory.
release-please derives the next version from them, so a commit outside the
scheme produces a wrong version or a missing changelog entry.

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`,
`ci`, `chore`, `revert`. Scope optional and lowercase. Breaking changes carry
`!` after the type and `BREAKING CHANGE:` in the body. Subject at most 100
characters.

**The pull request title matters most.** Merges are squashed with the PR title
as the subject line on `main`, so that title is what release-please reads.

No AI attribution trailers: no `Co-authored-by:` naming Claude or Anthropic, no
`Generated with Claude Code` line, and never `noreply@anthropic.com` as author
or committer.

## Branches

`<type>/<short-description>`, for example `feat/snapshot-schedules` or
`fix/token-in-error-output`. Push to a branch and open a pull request; `main`
takes no direct pushes.

## Before you push

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo deny check
```

The hooks run the fast half of this automatically. If a hook is ever too slow
to live with, fix the hook rather than reaching for `--no-verify`; skipping it
also skips the checks that take milliseconds.

## Testing against real hardware

Most of this API was reverse-engineered from the UGOS web UI and verified
against a live NAS. Endpoints are not documented by the vendor and shift
between firmware builds, so a change to request shapes needs a run against real
hardware, not only unit tests. Say so in the pull request when you have done
that, and which UGOS version you used.
