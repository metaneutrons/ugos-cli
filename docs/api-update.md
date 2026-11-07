# Firmware, configuration backup, resets, about and thumbnails

The control panel pages **Update & Restore** (`/updateAndReduction`), **About
this machine** (`/aboutMachine`) and **Index service** (`/indexService`) of
`com.ugreen.ctlmgr`. Reconstructed from the lazily loaded bundle and checked
against a live NAS (UGOS 1.20, on 2026-11-08). Reads were checked live. The
writes that install, restore or reset were **not** run: they are built from
the bundle and unit tests, need `--force`, and are marked below.

## Endpoints

| Area | Path | Wired as | Live |
|------|------|----------|------|
| Firmware | GET `firmware/status`, `firmware/version/is_new` | `update status` | read |
| | GET, POST `firmware/config` | `update config` | read, write and restore |
| | POST `firmware/install` | `update install` | refusal only |
| | POST `firmware/manual` (multipart), GET `firmware/manual/status` | `update upload`, `update progress` | refusal only |
| | POST `firmware/pubkey/import` | `update key` | not run |
| | GET `firmware/status/clean` | `update clean` | not run |
| Backup | GET `backup_restore/backupnow?uid=&backup_local=1` | `backup save` | read (saved and deleted) |
| | GET `backup_restore/status`, `backup/info`, `progress` | `backup info`, `backup progress` | read |
| | GET `backup_restore/services/list` | `backup services` | read |
| | POST `backup_restore/import` (multipart), `restore` | `backup restore` | not run |
| | GET `backup_restore/network/reset` | `backup reset-network` | not run |
| | GET `backup_restore/reset` | `backup reset-factory` | not run |
| Power | POST `desktop/reboot`, `desktop/poweroff` | `system reboot`, `system poweroff` | refusal only |
| About | GET `sysinfo/machine/common`, `machine/repair/status`, `collect/getInfo` | `system about`, `system ports` | read |
| Thumbnails | `api/ugreen.thumb/getThumbStatus`, `getPregenList`, `setThumbStatus`, `regenerate` | `index ...` | read, resume |

The thumbnail calls sit under `/ugreen/api/`, not `/ugreen/v1/`. The client
leaves a path that starts with `api/` as it is.

## Firmware

`firmware/status` answers the installed version, a state (`current_stat`: `0`
check failed, `1` up to date, `2` update available, `3` checking, `4` manual
upload) and, under `remote_base`, the update that was found with its version,
size, release time and changelog in the language of the NAS. `is_new` says
whether a newer firmware exists. The UI shows an install button when
`current_stat` is `2` and nothing runs.

`firmware/config` holds `auto_update` (`0` notify, `1` install important
updates, `2` install any update), a custom check interval (`interval_time` 1
to 72 whatever the unit, `unit` `1` hours, `2` days, `3` months) and the last
check. The write posts all four fields. The NAS tested was set to **install any
update by itself**, and had an update (1.21.1.0016) downloaded and waiting.

`firmware/install` takes `{}` and downloads, installs and **restarts the NAS**.
`ugos-cli` refuses it unless an update is ready, and without `--force`. Errors
`1509` (the signing key is not trusted), `9000`, `9001` and `1505` are turned
into refusals; `firmware/pubkey/import` takes the text of a `.pub` file as
`{data, pkgName: "import"}`.

`firmware/manual` takes a `.img` in chunks of 10 MiB, as `multipart/form-data`
with `file`, `chunks`, `chunk` (from 0) and `md5` of the whole file, one
request after another, and then reports through `firmware/manual/status`
(`current_stage` `install` or `complete`, `process`, `error_code` `1500` to
`1503`, `1509`). `ugos-cli update upload` sends the chunks and then polls with
a growing pause and a 45 minute deadline; a dropped connection after the
install began is taken as the restart. **Not run.**

## Configuration backup and the two resets

`backup_restore` is a backup of the **configuration** (connections, settings,
system), not of data.

- `backupnow?uid=<uid>&backup_local=1` answers the `.ugb` file itself. A file
  of 218 kB came from the NAS tested. UGOS restores only a `.ugb` whose name
  contains `backup_restore` and that is at most 100 MiB, so `backup save`
  insists on the name.
- `services/list` is a tree of categories and items with `service_name` leaves.
  A restore names the services to bring back.
- `restore` is `import` (the file, with `uid` and `is_local=1`), a look at
  `backup/info/local` and then `POST restore {services, is_local}`. It
  overwrites those services' configuration and restarts them. **Not run.**
- `progress?type=0|1` answers `1109, No task in the queue` when nothing runs.
- `network/reset` returns the addresses the NAS may have afterwards.
- `reset` is the **factory reset**, a GET that wipes data and configuration.
  UGOS wants the account's password sealed in `X-CurPass-Token`, as for deleting
  an account. `403` means the NAS is already going down. **Not run.**

## About and ports

`sysinfo/machine/common` holds `common` (name, model, serial, version, last
start, run time) and `hardware` (cpu, memory, network, ...). `machine/repair/status`
gives the warranty period in Unix milliseconds. `collect/getInfo` lists each
service with its TCP and UDP ports and a type (`1` system, `2` application).

## Thumbnails

`getThumbStatus` answers `status` (`1` generating, `2` complete, `3` paused,
`4` failed, `5` scanning), progress, counts and the seconds until a pause ends.
`setThumbStatus` takes `{target_status: 3, count_down_time: 3600|7200|18000}` to
pause and `{target_status: 1}` to resume. With nothing to do, a pause is
answered `1109, No task in the queue`, and `ugos-cli` says so. `regenerate`
takes `{app_id, regen_all, only_for_missing, ids}`. `setPregenList`, which
changes the ranges and the image and video quality, is **not wired**: the
values the quality may take are not in the bundle.

## What needs `--force`

| Change | Why |
|--------|-----|
| install, upload | flashes the firmware and restarts the NAS |
| auto-update set to `important` or `any` | the NAS then installs and restarts by itself |
| key import | decides which firmware is trusted |
| restore | overwrites configuration, restarts services |
| reset network, reset factory | address change, data loss |
| reboot, poweroff | restarts or powers off |

## Not known

The wording of the web UI's confirmations (keys only in the bundle), whether
`desktop/poweroff` wants a password, what `firmware/checking` does (defined,
never called), the allowed thumbnail quality values, and the `apiVersion`
parameter the web UI sends with `sysinfo/machine/common` (the call works
without it).
