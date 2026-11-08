# Log Center

The Log Center app (`logmgr`) and the `log/` calls of the core API. Reconstructed
from the app's bundle and checked against a live NAS (UGOS 1.20, on 2026-11-08).
Reads were checked live, and so were the writes that can be undone. Clearing
entries was only run for a time long before the first log entry, so it removed
nothing.

All calls are `v1/log/...`.

## Endpoints

| Call | Wired as |
|------|----------|
| GET `log/query` | `log`, `ugos_syslog` |
| GET `log/usersandmodules` | `logcenter modules` |
| GET `log/export` | `logcenter export` |
| DELETE `log/delete?log_type=&to_time=` | `logcenter clear` |
| GET, POST `log/filemgr/config` | `logcenter filemgr` |
| GET, POST `log/archive/config`, GET `log/archive/record` | `logcenter archive show`, `set`, `records` |
| GET, POST, DELETE `log/archive/visualizer/files`, POST `.../visualizer/open?archive_id=` | `logcenter archive files`, `add`, `remove`, `open` |
| GET, POST `log/notify/config`, GET `log/notify/record` | `logcenter notify show`, `set`, `records` |

Not wired: `log/sharefolders` and `log/dir/files`, which the bundle defines and
never calls.

## Querying

`log/query` takes `page` and **`size`** (see [api-system.md](api-system.md)),
`module`, `level` (`info`, `warning`, `critical`), `operator`, `keyword`,
`from_time` and `to_time` in Unix seconds, `order` (`down` newest first, `up`
oldest first), `visualizer` (`false` for the live log, `true` for the archive
that is open in the viewer) and `log_type`. **`log_type=1` is the transfer log**
(Samba, FTP, WebDAV, file manager); without it the event log answers. The web UI
repeats `level`, `module` and `operator` to filter on several, and `ugos-cli`
takes one of each. The keyword takes logical statements with parentheses; the
grammar is not known.

`usersandmodules` lists the users and modules that appear in the log. The
modules of the event log are `control_panel`, `file_manager`, `app_center`,
`storage_manager`, `snapshot`, `login`, `vault`, `iscsi`, `camera`, `voicememo`
and `other`; Samba, FTP and WebDAV belong to the transfer log.

## Export

`log/export` answers the file itself (CSV with a byte order mark, or HTML) and
takes `export_type`, `visualizer`, `lang`, and the same filters as the query.
A CSV of 227 kB came from the NAS tested.

## Clearing

`DELETE log/delete` has **no body**; everything is in the query. `log_type` is
**empty for the event log** and `1` for the transfer log. Without `to_time` it
clears everything, with it the entries older than that time. The web UI offers 3,
6 or 12 months; `ugos-cli` takes days or a time. Cleared entries cannot be
retrieved, so it needs `--force`. Whether archived records are touched is not
known.

## Archive policy

`archive/config` is `{status, data_bytes, num, remain_time, path, path_exist}`.
Archiving runs when the log reaches `data_bytes` MB (0, 500 or 1024), has `num`
entries (0, 1 000 000, 5 000 000 or 10 000 000) or holds entries older than
`remain_time` months (0, 3, 6 or 12), and puts them into a `.ulog` file in
`path`, **removing them from the current log**. It needs a folder and at least
one trigger, and `--force` while it is on. **UGOS keeps the triggers only while
archiving is on**: setting them with archiving off read back as 0.

The viewer opens `.ulog` archives as a log source. Adding takes the path on the
NAS, opening makes it the active source (`in_use`), and removing only drops it
from the list.

## Notifications

`notify/config` is `{status, keywords, levels, modules, operators}`. The web UI
always sends `keywords` as a list of one text, even when empty. A policy needs a
filter, which the web UI's own check never catches; `ugos-cli` does. The records
list what was sent, with the id of the log entry.

## Not known

The keyword grammar, whether clearing touches archived records, what UGOS does
with a transfer-log `log_type` of `0`, and the wording of the confirmations.
