# Recycle bins

The Recycle Bin app (`com.ugreen.filetrash`) and the file manager calls it
uses (`filemgr/...`). Reconstructed from the app's bundle and checked against a
live NAS (UGOS 1.20, on 2026-11-08). Everything was run live except restoring
over a name that is taken, on a throwaway share with a throwaway file. The bins
of the NAS's real shares were only read.

## What a bin is

A recycle bin is a `#recycle` folder inside a shared folder
(`/volume1/docs/#recycle`) or a home folder (`/home/NAME/#recycle`). Each share
and each user has a switch for it. A deleted file keeps its path below
`#recycle`, and has `dtime` (when it was deleted) and `remaining_del_time`
(until the automatic cleanup).

The three roots of the app are `root_type` 2 (the account's own bin), 3 (the
bins of the shared folders) and 4 (the bins of the user folders).

## Endpoints

| Call | Wired as |
|------|----------|
| GET `filemgr/getRecycleConf`, POST `setRecycleConf` | `recycle status`, `recycle config` |
| GET **`v2/`**`filemgr/getRecycleList` | `recycle list` |
| POST `filemgr/switchRecycle` | `recycle enable`, `recycle disable` |
| POST `v2/filemgr/getDirFileListV2` with `is_access_recycle` | `recycle ls` |
| POST `filemgr/restoreRecycle` | `recycle restore` |
| POST `filemgr/emptyRecycleBin` | `recycle empty` |
| POST `v2/filemgr/delPaths` with `forever: true` | `recycle delete` |
| GET `filemgr/taskPlanList`, `filemgr/shares` | `recycle plan list`, `plan choices` |
| POST `filemgr/addPlan`, `editScheduledTask`, DELETE `rmPlan?id=` | `recycle plan add`, `edit`, `remove` |

Not wired: the search (`createRecycleSearchTask` and its three companions), the
`v2` forms of restore and empty for a whole root (the v1 form with `root_type`
does the same), the per-user list preferences (`setRecycleWebConfig`) and the
size count of a bin.

**`getRecycleList` is a `v2` path.** The `v1` path answers success with an empty
object, and `ugos-cli` first listed no bins at all.

## Bins

`getRecycleList` answers `recycle_shares` and `recycle_users`, each with `name`,
`path`, `recycle_path`, `status` (on), `allow_only_admin_recycle`,
`recycle_not_empty`, `empty_recycle_task_id` (set while it is being emptied),
`share_id` or `user_id` and the retention of the automatic cleanup. It needs an
administrator.

`switchRecycle` takes `{path, status, allow_only_admin_recycle?}`; `path` is the
folder of the share or `/home` for all user folders. A new account's bin is on.
What UGOS does to files already in a bin that is switched off is not known, so
`ugos-cli` asks for `--force` for a bin that holds files.

## Listing, restoring, deleting

`getDirFileListV2` lists a folder with `path`, or a root with `root_type`, and
`is_access_recycle: true`, which is what makes UGOS show `#recycle`. The call is
encrypted like the other `v2` file manager calls.

`restoreRecycle` takes `{paths}` and puts the files back where they were. It has
no destination and no rule for a name that is taken; what happens then was not
tried.

`emptyRecycleBin` takes `{paths}` (the `#recycle` folders) or `{paths: [],
root_type}`, and works in the background. `delPaths` with `forever: true`
deletes the given entries for good, and is the same call as a normal delete
with `forever: false`, which is what puts a file in the bin.

**Both refuse a path that is not inside a `#recycle` folder.** `delPaths` with
`forever` on a live path deletes a live file. A path counts as inside a bin
when one of its components is exactly `#recycle`. Emptying and deleting need
`--force`.

## Cleanup plans

A plan is `{task_name, cleaning_type, interval, interval_unit, dynamic_task_time,
first_run_time, is_select_all, share_ids, user_ids}`. Type `1` deletes what is
older than `interval` days, every day at `dynamic_task_time`; its first run is
`interval` days from now. Type `2` empties the bins every `interval` days
(`interval_unit` 4) or months (5) from `first_run_time`, which has to be in the
future. `interval` is 1 to 365.

A plan for every bin (`is_select_all`) is made by UGOS itself, one per NAS:
the NAS tested has "after 30 days". **While it exists, UGOS answers `1359, A
system-wide cleanup plan already exists; new plans cannot be created` for any
new plan.** `ugos-cli` says so first. The plan can be edited, which was done
live (30, 31, back to 30 days). Deleting a plan keeps the bins and their files.

## Accounts that are deleted

See [api-users.md](api-users.md): the bin of a deleted account is gone with it,
and its files were not found in any bin.

## Not known

What disabling a bin does to the files in it, how a restore treats a name that
is taken, the wording of the confirmations, and the meaning of `date_type: 4`
in the search call.
