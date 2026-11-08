# File Version Explorer

The version manager app (`com.ugreen.versionmgr`): which folders keep older
versions of their files, the history of a file, and getting an older version
or a deleted file back. Reconstructed from the app's bundle and checked against
a live NAS (UGOS 1.20, on 2026-11-08), on throwaway shares that were deleted
again. Versioning was off on every folder of the NAS, and was switched on only
for those.

## Endpoints

| Call | Wired as |
|------|----------|
| GET `versionExplorer/folderInfo?type=share\|user\|domain&page&size&name` | `version folders` |
| POST `versionExplorer/folderInfo` | `version set` |
| GET `versionExplorer/dataCompatibilityProgress` | `version status` |
| POST `v2/versionExplorer/versionListInfo` | `version ls` |
| GET `versionExplorer/getVersionAllList?path&inode&uuid` | `version history` |
| POST `v2/versionExplorer/copyVersionToNAS` | `version copy` |
| POST `versionExplorer/restoreDelDir` | `version restore-deleted` |
| POST `versionExplorer/clearDelDirVersion` | `version clear-deleted` |
| GET `versionExplorer/getMonthVersion?year&month&current_time` | `version calendar` |

Not wired: the download (`downloadToken` and `downloadToPC` with the token in
the query), which `version copy` followed by `fs get` replaces; `getAllLevelsPath`,
`thumbnail`, `mediaPlayer`, `fileInfo`, `walkDir`, `getGuideStatus` and
`setGuideStatus` (the first-run guide). `getVersionLastList` and `versionStatus`
are in no file of the bundle.

## Folders

`folderInfo` answers `{folders, total, is_data_compatibility}`. A folder has
`enable`, `save_number` (1 to 32, `0` when off), `size` (bytes the versions
take), `fs_type`, and `is_available_volume`, `is_enable_folder`,
`is_generation_failure` and `is_repairing`. The write posts `{name, path,
save_number, enable, rescan}`.

Switching a folder off, and keeping fewer versions, delete the versions that no
longer fit, so both need `--force`. Error `5009` is no space, `50000` no such
folder, `50002` an invalid volume, `50003` a history migration in progress
(`dataCompatibilityProgress`, in percent) and `50001` old versions still being
deleted; `ugos-cli` turns them into refusals.

## Listing and history

`versionListInfo` is encrypted like the other `v2` file manager calls. Its body
is `{path, deadline_timestamp, show_deleted, data_type: 2, sort_type, reverse,
page, limit, left_count}`; the file list is `right_files.files`. The
`deadline_timestamp` (Unix seconds) is the time travel: UGOS lists the folder as
it was then. An item has `version_number` (the first 13 characters are a
millisecond timestamp), `is_deleted`, `uuid` and **`inode`, which is the path as
a text** on the NAS tested, and was documented as a number. `ugos-cli` keeps it
as it comes and sends it back unchanged. The first version of the client read it
as a number, got an empty list and said nothing: parse errors in these lists are
now errors.

`getVersionAllList` takes a file's `path`, `inode` and `uuid` and lists its
versions with `version_number`, `version_timestamp` and `last_modify_time`.
`ugos-cli history` finds the three in the folder listing, deleted files
included.

## Getting things back

`copyVersionToNAS` copies a version into another folder: `{paths: [{path,
file_type, version_number}], dst, deadline_timestamp (the version's own, as a
text), operation_type, contain_delete}`. `operation_type` is `0` skip, `1`
overwrite and `3` keep both, which gives the copy a name with the version's
time. The destination is checked first with `filemgr/determinePath`. The call is
asynchronous and answers nothing useful. Overwriting needs `--force`.

`restoreDelDir` takes an array `[{path, inode, uuid, is_dir}]` and puts
deleted items back where they were, `clearDelDirVersion` takes the same and
drops all their versions for good, which needs `--force`. A path that is not
deleted is refused first.

## Not known

What the web UI's confirmations say, the exact retention rule when
`save_number` shrinks, the content of the `ext4` hint, how the new client path
(`walkDir`, `downloadToPC` with `version_number`) is queried, and the effect of
versioning on a folder's recycle bin.
