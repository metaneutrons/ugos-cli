# Backup

No backup commands are implemented, and this note records why, so the question
does not have to be researched twice.

## There is no backup app

UGOS ships its apps as separate web bundles under `http://<nas>:9999/<app>/`.
Probing the plausible names — `backup`, `backupmgr`, `backup_restore`,
`backupcenter` — returns 404 for all of them. The only neighbouring app that
exists is `snapshot` (`com.ugreen.snapshot`), which manages filesystem
snapshots rather than backups.

## The three `backup_restore` paths are not an API surface

A search across the bundles turns up exactly three paths containing
`backup_restore`:

- `ugreen/v1/backup_restore/backupnow`
- `ugreen/v1/backup_restore/import`
- `ugreen/v1/backup_restore/restore`

All three appear in a single place: the `WHITE_LIST_FORM_ENCRYPT` array in the
desktop bundle (see [api-encryption.md](api-encryption.md)). They are listed
there as paths exempt from request encryption, next to unrelated entries such
as `music/songlist/share/download` and `security/ssl/import`. Nothing in the
bundles calls them.

That places their consumer outside the web UI — most likely UGREEN's desktop
sync client, whose sync-and-backup strings do live in the same bundle. Their
parameters, their semantics, and what they operate on are therefore unknown.

## Why nothing was implemented

There is no listing, status, or progress endpoint anywhere in this group. A
`backup run` command would trigger a potentially long-running operation on a
production NAS with no way to observe it, built from a path whose parameters
would have to be guessed, and it could not be tested without risking the data
it claims to protect. An untested backup command is worse than no command,
and an untested restore command is worse still. The same reasoning kept
`DeleteLogs` and `processes/stop` out of the CLI.

## What would settle it

A HAR capture from the snapshot app. Its endpoints sit in lazily loaded
chunks that could not be resolved from the entry bundle alone, so the paths
are not obtainable by reading `app-snapshot.js`. Snapshots are also the closer
match to what a NAS user means by backup, and listing and creating them are
operations that can be verified safely, unlike `backup_restore/restore`.
