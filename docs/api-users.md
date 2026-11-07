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
| POST | `user/rule/password` | plain | not wired (checks a password against the rules) |
| GET, POST | `user/quota/{uid}` | plain | not wired (quota per volume) |
| GET, POST | `user/permission/{uid}`, `user/authorized/{uid}` | plain | not wired |
| | `user/search`, `user/import*`, `user/invite*`, `user/headpic/*`, `otp/disable` | plain | not wired |
| | `group/list`, `group/info/{gid}`, `group/create`, `group/edit`, `group/delete` | plain | not wired |

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
A Unix time sets an expiry, and the web UI sends the date as text in
`expire_date`. Neither was tried.

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

`ugos-cli user delete` refuses the account it is logged in with, and the only
administrator. After the call it reads the list again and says so if the
account is still there.

## Not known

- What happens to the home folder of a deleted account and to the files in it.
- The format of `expire_date`, and whether a Unix time in `expire_time` expires
  an account as the web UI says.
- What a custom group looks like in `group/list`. The test NAS had none.
- Whether `user/rule/password` rejects a password that `user/create` accepts.
  The web UI asks first, `ugos-cli` does not.
