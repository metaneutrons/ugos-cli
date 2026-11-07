# SAN Manager: iSCSI

The SAN Manager app (`com.ugreen.iscsi`, app version 856): iSCSI targets,
LUNs, initiator groups and the global setting. Reconstructed from the app's
bundle and checked against a live NAS (UGOS 1.20, on 2026-11-08). The NAS had
no targets, LUNs or groups, so the whole life cycle was run live with test
objects that were deleted again, with one exception noted at the end.

All calls are `v1/iscsi/...`.

## Endpoints

| Area | Path | Wired as |
|------|------|----------|
| Setting | GET, POST `iscsi/setting`, GET `iscsi/setting/portCheck` | `san setting show`, `san setting set` |
| Target | GET, POST, PUT `iscsi/target`, GET `target/exist`, `predict`, `conn` | `san target ...` |
| | POST `target/enable?id=`, `target/disable?id=` | `san target enable`, `disable` |
| | DELETE `iscsi/target` (body), POST `target/deleteWithLun` | `san target delete` |
| LUN | GET, POST, PUT, DELETE `iscsi/lun` | `san lun ...` |
| Group | GET, POST, PUT, DELETE `iscsi/group`, `group/check`, `group/bind` | `san group ...` |
| Volumes | GET `storage/volume/list` | via the volume list of the Storage Manager |

Not wired: `iscsi/popup` (the first-run guide), the `target/page`, `lun/page`
calls (deep-link paging), `checkCustomPassword` (the two CHAP passwords are
compared locally, which is what it decides).

## Conventions

- **Lists** take `page` and `page_size` and answer `{results, total}` with
  `results: null` when empty. `ugos-cli` asks for everything at once.
- **Updates** are `PUT` with the id in the query (`iscsi/lun?id=3`). **Deletes**
  are `DELETE` with a JSON body of ids and `exclude: false`.
- **Sizes** go as the number the user typed and `size_type` `GB` or `TB`, 1024
  based, whole numbers. Responses carry bytes.
- **Names** of LUNs, groups and targets are letters, digits, `_` and `-` up to
  32. A description is up to 32 characters without `! , \` : ' " / \ < > ? #
  @ $ % * ; { } [ ] ( ) |`. A target IQN is up to 128 characters of letters,
  digits, `:`, `.` and `-`. An initiator IQN is up to 223 of letters, digits,
  `.`, `:` and `-`, and a group holds 1 to 256 of them. A CHAP user is up to 12
  letters and digits, a CHAP password 12 to 16 letters and digits.
- **Passwords** (CHAP, and the confirmation of a delete) are RSA-sealed with the
  login key, as in [api-users.md](api-users.md).
- UGOS works asynchronously: a new LUN is `creating` (status 5) for a few
  seconds, a deleted object stays listed while it is removed. `ugos-cli` waits
  for a group to settle and polls up to ten seconds for a deletion.

## Setting

`{iqn_prefix, port, type, userid, mutual_userid}`. `type` is the default CHAP
(`0` off, `1` one-way, `2` mutual). The write posts `{port, iqn_prefix,
chap_info: {type, userid?, password?, mutual_userid?, mutual_password?}}`; the
port is 1001 to 65535 and checked with `portCheck` (`is_available`). A new port
restarts the service and new credentials affect initiators that are connected,
so both need `--force`.

## LUNs

`status`: `0` normal, `1` damaged, `2` read only, `3` not enough space,
`4` protected, `5` creating, `6` editing, `7` removing, `8` expanding. Create
posts `{name, desc, size, size_type, volume_name, type: thin|thick}`. A thick
LUN may not exceed the volume's free space; a thin one larger than that needs
`--force`.

Expanding is the same `PUT` as any edit: raise `size`. A LUN only grows, and a
thick one by at most the free space. Edit posts `{name, desc, size, size_type,
enable_fua, access_type: default|custom, group_access: [{group_id, group_name,
access_mode: rw|ro, desc}]}`; `default` sends an empty list. A larger size and
an access change both need `--force`: the first cannot be undone, the second
changes what initiators see.

Delete posts `{lun_ids, exclude: false}` with the account's sealed password in
`X-CurPass-Token` and destroys the data.

## Targets

Create posts `{name, iqn, lun_id, chap_type: close|default|custom, chap_info}`.
The LUN has to be unmapped and `status` 0, 3 or 4. `default` needs the default
CHAP of the setting to be on. Edit posts `{name, iqn, multi_conn, chap_type,
chap_info: {..., apply_now}, crc_info: {enable_head_crc, enable_data_crc,
apply_now}}`; `apply_now: false` applies CHAP and digest changes at the next
connection, `true` drops sessions. `ugos-cli` asks for `--force` when initiators
are connected and the IQN, CHAP or digest changes.

`disable` cuts the connections, so it needs `--force` while initiators are
connected. A plain delete leaves the LUN, unmapped. `deleteWithLun` is a `POST`
with the same sealed password header and deletes the LUN with its data, so it
always needs `--force`. Target `status` `0` and `1` are on, `2` is off and `3`
to `7` are busy.

## Groups

Create posts `{name, desc, iqns}` after `group/check` (`{iqns}` answers which
belong to a group already). Changing the initiators of a group that LUNs are
bound to (`group/bind?group_id=`), and deleting such a group, need `--force`.

## What was run live

Everything above except the connection list with initiators (none exist), a
changed port (refused without `--force`; not run with it) and CHAP credentials
(sealed and sent, but no initiator to check against). Test objects were
deleted, including a LUN together with its target through `deleteWithLun`.

## Not known

The meaning of target `status` 3 to 7 individually, the formats of the LUN list
filters `lun_types`, `volume_names`, `volume_status`, the effect of
`exclude: true`, and the wording of the confirmations.
