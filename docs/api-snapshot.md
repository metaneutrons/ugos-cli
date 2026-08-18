# Filesystem snapshots

The snapshot app (`com.ugreen.snapshot`) takes btrfs snapshots of shared
folders and user home directories. It has nothing to do with the KVM
snapshots in [api-kvm.md](api-kvm.md), which is why the CLI keeps them apart
as `snapshot` and `vm snapshot`.

Reconstructed from a HAR capture plus the app's lazily loaded bundles, and
verified against a live NAS on 2026-08-19.

## The only REST-shaped corner of UGOS

Everything else in UGOS posts to verb-named endpoints (`ShowLocalVirtualList`,
`GenerateSnapshot`). This app uses one path with four methods:

| Method | Path | Purpose |
|--------|------|---------|
| GET | `v1/snapshot/snapshot` | List snapshots of one folder |
| POST | `v1/snapshot/snapshot` | Create a snapshot |
| PUT | `v1/snapshot/snapshot` | Edit description or lock |
| DELETE | `v1/snapshot/snapshot` | Delete, **with the ids in the body** |
| POST | `v1/snapshot/snapshot/clone` | Copy a snapshot into a new folder |
| POST | `v1/snapshot/snapshot/restore` | Roll a folder back — not implemented |
| POST | `v1/snapshot/snapshot/restore/preview` | What a restore would change |
| GET | `v1/snapshot/folder/home` | Home folders that can hold snapshots |
| GET | `v1/snapshot/folder/share` | Shared folders likewise |
| GET | `v1/snapshot/preference` | UI preferences |

A DELETE that carries its payload in the body is unusual enough that the
client needed a dedicated method for it.

## Paging is mandatory

Both listings require `page_number` and `page_size`. Omitting either — or
passing zero — fails the whole call with `9999, Failed to operate`.

The parameter names cost some time. `limit`/`offset`, `page`/`size`,
`page_num`, `pageSize` and several others are all accepted silently and
ignored. What gave it away was the `debug` field UGOS puts in its error
envelope:

```json
{"code":9999,"msg":"Failed to operate","debug":"limit is 0, offset: 0, limit: 0"}
```

That field is worth remembering. The client discards it, but a raw `curl`
shows it, and it names the missing parameter where `msg` says nothing.

`snapshot/snapshot` also needs `folder_id` and `folder_type`, the latter
being `home` or `share`. A wrong value answers `unknown folder type: <value>`,
which is how the two valid ones were found.

## Success is code 200

Unlike the rest of UGOS, this app reports success as `code: 200` rather than
`0`. The shared envelope already treats 200 as success, so nothing special
was needed.

## The lock flag does not lock anything

Snapshots carry `is_locked`, and the web UI refuses to delete a locked one.
The API does not. Verified twice against a live NAS: a snapshot created with
`is_locked: true` is removed by `DELETE v1/snapshot/snapshot` without
complaint.

Anyone treating the flag as protection against accidental deletion is
mistaken, which is why the CLI help says so at the point where it is set.

## Restore is deliberately absent

`snapshot/snapshot/restore` rolls a folder back to an earlier state. It is
implemented in the API but not exposed by the CLI, for the same reason
`backup_restore/restore` is not (see [api-backup.md](api-backup.md)): it is
the most destructive operation available here, and it cannot be tested
without risking real data.

`snapshot clone` covers the common need without that risk. It materialises a
snapshot's contents as a new folder beside the original, leaving the original
untouched, so files can be recovered by copying them back.

## What was verified

Against a live NAS on 2026-08-19: folder listing for homes and shares,
snapshot listing, create, edit (description and lock), and delete — including
the finding above about locked snapshots. `clone` is implemented from the
bundle's call shape but was **not** run, because it creates a folder that the
snapshot API offers no way to remove.
