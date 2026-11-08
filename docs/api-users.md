# User accounts

Accounts, their roles and groups, and the password. They live on the core API
under `user/`, with no app of their own. `user list` and `user me` were
documented in [api-system.md](api-system.md); this page covers creating,
changing and deleting.

Reconstructed from the lazily loaded bundle of the control panel
(`com.ugreen.ctlmgr`) and verified against a live NAS on UGOS 1.20 on
2026-11-07, with throwaway accounts that were created and deleted again.

## Endpoints

| Method | Path | Transport | Wired |
|--------|------|-----------|-------|
| GET | `user/list` | plain | `user list`, `user show` |
| GET | `user/uid/{uid}` | plain | not wired, the list says as much |
| POST | `user/name/check` | plain | `user create` |
| POST | `user/create` | plain, **sealed password** | `user create` |
| POST | `user/edit` | plain, **sealed password** | `user update` |
| POST | `user/check/delete` | plain | `user delete` |
| DELETE | `user/delete` | plain, **password header** | `user delete` |
| GET, POST | `user/config/password` | plain | not wired (password rules) |
| POST | `user/rule/password` | plain, **sealed password** | `UsersApi::user_check_password`, not used by the CLI (see below) |
| GET, POST | `user/quota/{uid}` | plain | `user quota`, `user quota-set` |
| GET | `user/check/quota` | plain | `user quota-set` (refuses when `false`) |
| GET, POST | `user/permission/{uid}` | plain | `user permissions`, `user permit` |
| GET, POST | `user/authorized/{uid}` | plain | not wired (app rights of domain users) |
| | `user/search`, `user/import*`, `user/invite*`, `user/headpic/*`, `otp/disable` | plain | not wired |
| GET | `group/list` | plain | `group list` |
| POST | `group/create`, `group/edit` | plain | `group create`, `group edit` |
| POST | `group/name/check` | plain | not wired (the rules are checked locally and `group/create` refuses a taken name) |
| DELETE | `group/delete` | plain | `group delete` |
| GET | `group/info/{gid}` | plain | `group edit` |
| GET, POST | `group/permission/{gid}` | plain | `group permissions`, `group permit` |

All of them are `v1`.

## The account

`user/list` answers `{list: [...]}`, and `user/uid/{uid}` answers one entry of
the same shape. An entry carries 40 or so fields, among them:

| Field | Meaning |
|-------|---------|
| `username`, `uid` | login name, numeric id (a number, not a string) |
| `role` | `admin` or `users`. `account_type` is `0` for both and does not tell them apart |
| `groups` | `[{gid, name}]`, or `null` |
| `description`, `email` | the email is **masked** (`a****a@example.org`) |
| `email_encrypted` | the real address, see below |
| `enable_home_dir`, `deny_change_pwd` | home folder, and whether the account may not change its own password |
| `expire_time`, `expire_date` | `-1` never, `1` disabled, else a Unix time |
| `is_expired` | whether the expiry has passed |
| `dir_quota`, `unit` | home quota in units of 0 MB, 1 GB, 2 TB |

### `email_encrypted`

The masked address cannot be sent back, and an edit has to send the address
(see below). The real one is in `email_encrypted`, which the web UI undoes in
the browser: the value is Base64, its **last 16 bytes are an AES-128 key**, and
the rest is AES-128-ECB with PKCS#7 padding under that key. The key travels
with the data, so this hides the address from a glance and from nothing else.
`ugos_client::crypto::decrypt_email` reads it back.

## Passwords

A password is never sent in the clear. UGOS wants it RSA-sealed (PKCS#1 v1.5)
with the **login key**, the one `verify/check` returns in its `X-Rsa-Token`
header. It is the key the login seals the password with, and not the session
key of the encrypted transport. `user create` and `user edit` carry the sealed
password in the `password` field of the body; `user edit` leaves the field out
to keep the old one.

## Creating

### user/name/check
- **Body**: `{name}`
- **Response**: `{}`, or `1007, Username already exists, please enter another name!`

The web UI also refuses `root`, `admin` and `administrator` in any case, a
leading `-`, and leading or trailing spaces. `ugos-cli` checks the same.

### user/create
- **Body**: `{username, password, email, description, role, groups: [{name}],
  enable_home_dir, deny_change_pwd, dir_quota, unit}`
- **Response**: not used; `ugos-cli` reads the account back from the list

The web UI's defaults are `role: users`, a home folder, no quota, `unit: 1`,
no groups. It clears `deny_change_pwd` for an administrator, and so does
`ugos-cli`.

An unknown group is answered `9999, Failed to operate`. Neither the account
nor anything else is created then.

## Changing

### user/edit
- **Body**: `{uid, username, email, description, role, groups: [{name}],
  enable_home_dir, deny_change_pwd, dir_quota, unit, expire_time,
  expire_date, password?}`
- **Response**: `{}`

The request replaces the account, it does not patch it. Three details cost
time:

- **The email must be in the body.** A request without `email` is answered
  success and **blanks the stored address**. Every edit sends the address back,
  which is why `email_encrypted` is read (see above). `ugos-cli` refuses an
  edit if it cannot read the address and `--email` is not given.
- **Groups follow the role and the home folder.** UGOS adds `admin` to an
  administrator and `ughomeusers` to an account with a home folder, and drops
  them again. The built-in groups `users`, `admin` and `ughomeusers` cannot be
  left: a request without one of them is answered success and the account
  stays in it. `ugos-cli` checks the groups that were named and no others.
- **`dir_quota` and `unit` had no effect on an edit.** The request was
  answered success and the quota stayed. The web UI sets a quota per volume
  through `user/quota/{uid}`, which is not wired.

`expire_time` is `-1` for an account that never expires and `1` for one that
is disabled; `ugos-cli user update --disable` and `--enable` set those two.

An expiry date goes in two fields, as the web UI's date picker sends it:
`expire_date` is the text `YYYY-MM-DD`, and `expire_time` is midnight UTC of
that day in seconds. UGOS keeps the date and **moves the time to midnight of
its own time zone** (on a NAS in Central European Time, an hour earlier), so
the date is the field to compare. A date in the past is refused by the web UI,
and so by `ugos-cli`; one beyond 2280 is too. `ugos-cli user update --expires
DATE` sets it, and `--no-expiry` lifts it.

## Deleting

### user/check/delete
- **Body**: `[uid, ...]`
- **Response**: `{snapshot, safespace}`

`snapshot` says that an account has snapshots that would go with it, and
`safespace` that it has a safe space. The web UI sends a user with a safe space
to the safe space app first, and `ugos-cli` refuses it. Snapshots are refused
unless `--force` is given.

### user/delete
- **Method**: DELETE, **JSON body** `[uid, ...]`
- **Header**: `X-CurPass-Token`, the password of the account that makes the
  request, sealed with the login key as above
- **Response**: `{}`

An account that is not an administrator is answered
`1008, Standard users do not have permission to access this interface`, here
and on every other call that changes an account.

**The home folder goes with the account, and so do the files in it.** A
throwaway account with one file in `/home/NAME` left nothing behind: the folder
was gone (`1302, Path does not exist`) and so was its place in
`/volume1/@home`. Whether a recycle bin keeps a copy could not be seen (see
[The recycle bin](#the-recycle-bin)). `ugos-cli user delete` reads the folder first and refuses a non-empty one unless `--force` is given,
as it does for snapshots.

`ugos-cli user delete` also refuses the account it is logged in with, and the
only administrator. After the call it reads the list again and says so if the
account is still there.

## Groups

### group/list
- **Response**: `{list: [{gid, groupname, description, users, utime}]}`

`users` is `null` for a group with no members and `[{name, uid}]` otherwise.
Only the groups an administrator made are listed, from `gid` 1000. The built-in
`admin`, `users` and `ughomeusers` are not, though an account's own `groups`
names them. The web UI's account form offers only the listed groups, and it
sends only those back in `groups`. UGOS accepts the full list as well (the
built-in groups follow the role and the home folder anyway), which is what
`ugos-cli` sends.

### Groups: create, edit, delete

- **Create**: POST `{groupname, description, users: [{name}]}`, answered
  `{result: gid}`. A group name is 1 to 64 characters, not only digits, not
  `root`, `admin`, `administrator` or `none` (any case), without a leading `-`
  or spaces at its ends. A description is up to 64.
- **Info**: GET `group/info/{gid}` answers `{groupname, description, users:
  [{name}]}`.
- **Edit**: POST `{gid, groupname, description, users: [{name}]}`. The
  member list is the **whole** list, so `ugos-cli` reads it, edits it and sends
  it back. A group **can be renamed**, which was verified live, and so can the
  members be changed.
- **Delete**: DELETE with the JSON body `[gid]`, answered `{}`. `ugos-cli`
  asks for `--force` when the group has members.

Joining a group is also a field of the account: an account created with
`groups: [{name}]` shows up in the group's `users` at once, and leaves it when
the account is deleted.

## Share permissions

`user/permission/{uid}` and `group/permission/{gid}` answer `{result: [{id,
name, path, permission, permission_mask, ...}]}`, one row per share.
`permission` is what was set (`-1` nothing, `0` none, `4`/`5` read, `6`/`7`
write; the web UI writes only `0`, `5`, `7` and `-1`), `permission_mask` is what
counts after the groups. The write is a POST of `{list: [{path, permission}]}`
and takes any subset of the shares: the web UI sends the changed rows for an
account and all rows for a group, and both were verified live to accept one row.
An administrator always gets read and write, which the web UI enforces by
turning read into write; `ugos-cli` refuses read for an administrator instead.
`inherit` is `-1`.

## Quotas

`user/check/quota` says whether this NAS offers quotas per account at all.
**The NAS tested answers `false`**, and the web UI hides the tab. `user/quota/{uid}`
still answered, with `btrfs` and `ext4` rows `{path, hard, max_value, used}`;
`max_value` was the volume's size in bytes. The write is a POST with a **bare
array** `[{path, hard}]`, with `hard` in **KiB** according to the web UI's own
conversion (1 MB is 1024) and `0` for none. `ugos-cli` sends KiB and refuses
when the NAS offers no quotas. **The write was not run against a NAS**, so the
unit of `hard` is the reading of the bundle, not a measurement. The limit of the
home folder (`dir_quota`, `unit` of `user/create`) is another thing and was
found to be ignored on edit.

## Password rules

### user/rule/password
- **Body**: `{username, password}`, the password sealed as above. Plain text is
  answered `1006, Password decryption failed`.
- **Response**: `{}`, or one of `1028, Insufficient password length: 6`,
  `1030, It cannot be a simple common password`, `1032, It must contain at
  least one number`

`user/create` and `user/edit` apply **the same rules** and answer with the same
codes, so the check is a pre-check and not a stricter one. Passwords of three
kinds (too short, common, without a number) were refused alike by all three
calls, and one that passed the check was accepted by `create` and `edit`.
`ugos-cli` does not pre-check: it would only move the same message forward, and
the library offers `UsersApi::user_check_password` for a caller that wants to
ask before it collects anything else.

The settings in `user/config/password` (`enable`, `force_change`, `max_days`,
`warn_age`) read `enable: false` on the NAS tested, and the three rules
applied nevertheless. What a policy that is switched on adds was not tried,
since that changes a setting of the NAS.

## The recycle bin

The home folder of a deleted account does **not** end up in a recycle bin.
With the recycle bin API (see [api-recycle.md](api-recycle.md)) this was
checked on 2026-11-08: a new account has a recycle bin of its own, switched on,
under `recycle_users`. A throwaway account with one file in `/home/NAME` was
deleted with `--force`. Afterwards its bin was gone from `getRecycleList`, the
users root (`recycle ls --root users`) listed nothing, and the folder and its
file were gone from `/home`. `/volume1/@recycle` and `/volume1/@users` exist,
but the file manager answers `1301, Access not allowed` for them, an
administrator included, so a copy there can be neither found nor ruled out.
Treat the deletion as final.

## Not known

- What the web UI's delete dialog says about the account's data, and what
  `delHomeFolder` does. The text of the dialog is not in the bundle that was
  read.
- What a password policy that is switched on adds to the three rules.
