# Shared folders and the SMB service

A share is a top-level folder on a volume that UGOS offers to the network. Its
settings and its access rules live on the file manager API
(`com.ugreen.filemgr`); the SMB service that publishes it lives under `file/`
and belongs to the control panel (`com.ugreen.ctlmgr`).

Reconstructed from the lazily loaded bundles of both apps and verified against
a live NAS on UGOS 1.20 on 2026-11-07, with a throwaway share that was created
and deleted again. `docs/api-files.md` covers the rest of the file manager and
`docs/api-encryption.md` the encrypted transport.

## Endpoints

| Method | Path | Transport | Wired |
|--------|------|-----------|-------|
| GET | `filemgr/getShareList` | plain | `share list` |
| GET | `filemgr/getShareFolderInfo?id=` | plain | `share show` |
| GET | `v2/filemgr/pathAcl?path=` | encrypted | `share acl show` |
| POST | `v2/filemgr/createShareFolder` | encrypted | `share create` |
| POST | `filemgr/updateShareFolderInfo` | plain | `share update` |
| POST | `filemgr/modifyPathAlc` | plain | `share acl set` |
| DELETE | `v2/filemgr/delSharedFolder?path=` | encrypted, **password** | `share delete` |
| GET | `file/smb/config` | plain | `smb status` |
| GET | `file/advanced/config` | plain | `smb status` |
| POST | `file/smb/start`, `file/smb/stop` | plain | `smb start`, `smb set`, `smb stop` |
| POST | `file/advanced/start` | plain | `smb discovery` |
| DELETE | `file/smb/cache` | plain | `smb clear-cache` |
| GET | `filemgr/getSharedFolderList` | plain | not wired |

## Listing and settings

### filemgr/getShareList
- **Response**: `{result: [{id, name, path, filesystem, owner, time_backup, ctime, mtime, ...}]}`

Entries carry 30 fields (icons, tags, mount state, quick-access flags).
`time_backup` says whether the share is a Time Machine target.

### filemgr/getShareFolderInfo
- **Params**: `id`, the share id from the list
- **Response**: `{id, name, path, volume, quota, unit, data_check, network_places_hide, no_permission_hide, encrypt_status, owner, ...}`

`quota` is in `unit`s, and `unit` indexes `["MB", "GB", "TB"]`. A quota of 0
means no limit, and the unit is then still reported.

### filemgr/getSharedFolderList

Answers `{data: [], total: 0}` without parameters. The web UI passes some, which
were not found, so this is not used. `getShareList` serves.

## Creating

### v2/filemgr/createShareFolder
- **Method**: POST, encrypted
- **Body**: `{volume, name, quota, unit, data_check, network_places_hide,
  no_permission_hide, permissions, enable_recycle, allow_only_admin_recycle,
  open_encrypt, encrypt_session_id}`
- **Response**: the new folder, with `id`, `name` and `path` among many more fields

The web UI's defaults are `quota: 0`, `unit: 1`, `data_check: false`,
`network_places_hide: false`, `no_permission_hide: true`, `permissions: []`,
`enable_recycle: true`, `allow_only_admin_recycle: true`, `open_encrypt: false`.
With empty `permissions` only administrators have access at first.

The web UI refuses the names `personal_folder`, `homes` and `home`, names that
start with `@`, empty names, and names over 80 characters. `ugos-cli` checks
the same and adds a slash.

Encryption (`open_encrypt`) needs a key dialog and is never asked for.

## Changing settings

### filemgr/updateShareFolderInfo
- **Method**: POST, plain
- **Body**: `{id, set_share_folder_info: {data_check, name, network_places_hide,
  quota, volume, unit}, permissions: [], no_permission_hide}`
- **Response**: `{result: ""}`

Two details cost time:

- `no_permission_hide` goes **beside** `set_share_folder_info`, not inside it.
- A body with the settings at the top level is answered `{result: ""}`, which
  looks like success, and **changes nothing**. `ugos-cli share update` reads
  the settings back for that reason.

The web UI always sends `data_check: false` here. `ugos-cli` sends the value
the share has, so that an update does not switch checksums off. Whether UGOS
would honour a change of it was not tried. The name and the volume are not
changed by `ugos-cli`.

## Access rules

### v2/filemgr/pathAcl
- **Method**: GET, encrypted
- **Params**: `path`
- **Response**: `{acls: [group], no_permission_hide, total_users}`

Each group has `name`, `gid`, `permission_mask`, `select_permissions`,
`parent_permissions` and its `users`; each user has `username`, `uid`,
`is_admin` and the same three fields.

- `select_permissions` is what is set on this path. `-1` means nothing is set.
- `permission_mask` is what applies in the end. A user whose own entry is
  unset gets the access of the groups they are in.
- `parent_permissions` is what comes from the parent. `-1` means none.

A group with no members is **not listed**.

### The masks

| Mask | Meaning | Written by the web UI |
|------|---------|-----------------------|
| `0` | No access | yes |
| `4`, `5` | Read | `4` |
| `6`, `7` | Read and write | `7` |
| `-1` | Nothing set | yes, to clear an entry |

The web UI treats `4` and `5` alike, and `6` and `7`. Setting a user to `4`
stored `5`, which the list then shows. `ugos-cli` compares by meaning, not by
number.

### filemgr/modifyPathAlc
- **Method**: POST, plain
- **Body**: `{path, permissions: [{type, name, permission_mask, is_admin}]}`
- **Response**: `{}`

`type` is `user` or `group`. Only the entries that change are sent; the rest
keep their access. For a user, `name` is the login name (it equals the display
name on the NAS tested, so which of the two UGOS reads was not told apart).

**UGOS keeps the administrators group at read and write.** Asking for less
answers `{}` and changes nothing, and the web UI disables those rows.
`ugos-cli share acl set` reads the list back and names every rule UGOS did not
apply.

A rule for a group with no members is accepted without error. Whether it has
an effect could not be seen, since the group is not listed. `ugos-cli` only
accepts names that the list shows.

## Deleting

### v2/filemgr/delSharedFolder
- **Method**: DELETE, encrypted
- **Params**: `path`, the share's path
- **Header**: `X-CurPass-Token`, the account's password, RSA-sealed

UGOS asks for the password here, as the web UI does with a dialog. The request
without the header is answered `1101, Password verification failed`. The key
to seal it with is the **login key**, the one `verify/check` returns in its
`X-Rsa-Token` header, and not the session key that the encrypted transport
uses: the latter is answered `1006, Password decryption failed`.

**The folder and everything in it is deleted.** The path was gone afterwards
(`1302, Path does not exist`). Whether UGOS keeps a copy somewhere was not
looked for. `ugos-cli share delete` refuses a folder that holds files unless
`--force` is given.

The web UI handles two outcomes that were not reproduced: a `result` in the
response that names a background task for a large deletion, and an error code
for folders that hold snapshots, which lists them. `ugos-cli` reads the share
list afterwards and says so if the share is still there.

## The SMB service

### file/smb/config
- **Method**: GET
- **Response**: 36 fields

The useful ones: `status` (running), `nas_name`, `work_group`, `wins_server`,
`is_joined_domain`, `hide_share_dir_for_no_permission`, and `enable_ntlmv1`.
Protocol selection (`smb_protocols`, `min_protocol`, `max_protocol`), transport
encryption (`encrypt_mode`) and signing (`server_sign_mode`) are numeric codes
that UGOS does not name; `smb status` shows them as they come.

### file/advanced/config
- **Method**: GET
- **Response**: `{enableBonjourService, enableWsdd2Service, enableUPnPService,
  enableTimeMachine, sharedFileInfos: [{id, name, path, time_backup, type}]}`

Bonjour is how Macs find the NAS, WS-Discovery how Windows does.

### Starting, stopping, the cache

These are wired now, with the other file services. See
[api-fileservices.md](api-fileservices.md) for how they work, what was tried
live and what needs `--force`.

## Not known

- Whether a rule for a group without members has an effect.
- What a share with snapshots answers when it is deleted, and what a long
  deletion returns.
- Which of login name and display name the ACL reads for a user.
- Whether `data_check` can be changed after creation.
- What `min_protocol` and `max_protocol` stand for. The web UI never touches them; `smb_protocols` (`1` SMB1, `2` SMB2 and 3) is what it edits. Transport encryption and signing are `0` by client, `1` forbidden, `2` forced.
