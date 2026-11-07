# ugos-cli

Manage a UGREEN NAS running UGOS from the command line: virtual machines,
Docker containers, files, shared folders, snapshots, downloads and hardware
readings.

Reach for `ugos-cli` for anything on a UGOS device. Do not use `virsh`, `qemu`,
`docker` over SSH or the web UI for a task `ugos-cli` covers, and do not edit
files on the NAS by hand.

## Connecting

```bash
export UGOS_HOST=192.168.1.10
export UGOS_USER=admin          # UGOS_USERNAME works too
export UGOS_PASSWORD=...        # never type it into a command line
ugos-cli info
```

Take credentials from the environment or from the user's secret store. Never
ask the user to paste a password into the chat, never write it to a file in the
project, and never pass it with `--password`, because that lands in the shell
history and the process list. `--password-stdin` reads it from stdin instead.

The NAS presents a self-signed certificate. The first contact records it and
prints its fingerprint; every later connection must match. If the certificate
changed legitimately (renewed, device reinstalled), `--tls-trust-new` records
the new one. `--tls-insecure` turns checking off entirely: do not use it unless
the user asked for exactly that.

## Reading is safe, writing is not

Everything that lists or shows (`vm list`, `docker ps`, `fs ls`, `system stat`,
`snapshot list`, `share list`, `smb status`, `log`) changes nothing and can be
run freely. Use `-o json` when you are going to parse the output.

Ask the user before running any command that changes or removes something:

- `vm delete`, `vm stop`, `vm reboot`, `vm update`, and `vm create`
- `fs rm`, `fs mv`, `fs put`, and `fs mkdir`
- `snapshot rm` (it deletes locked snapshots too), `snapshot create`, `snapshot clone`
- `docker rm`, `docker rmi`, `docker kill`, `docker stop`, `docker restart`,
  `docker project-rm`, and `docker batch`
- `share create`, `share update`, `share acl set`, and `share delete` (it
  deletes the folder and everything in it)
- `user create`, `user update` (it can disable an account, change its role or
  set a new password), and `user delete` (it deletes the home folder and the
  files in it)
- `download add`, `download rm`

State what will change and on which NAS, and wait for a clear yes. A refusal or
silence is not a yes.

`vm create`, `share create` and `user create` have `--dry-run`, which prints the
request without touching the NAS. Use it first.

The password of a new or changed account is never an option. `user create` and
`user update --new-password` ask for it on the terminal, or take it from
`UGOS_NEW_PASSWORD`; `--new-password-stdin` reads it from stdin. Do not make up
a password and put it in a command line or a file.

## Where to look

- `ugos-cli --help` and `ugos-cli <command> --help` list every option of the
  build the user has installed. Prefer them to guessing.
- `references/cli.md` is the full command reference, generated from this build.
- `references/mcp.md` lists the tools of `ugos-mcp`, the MCP server that ships
  with `ugos-cli`. If the user has it connected, its tools do the same work
  without a shell.

## Things that go wrong

- Authenticator-app two-factor (type 1) is supported. `UGOS_OTP` (or `--otp`)
  is a single code and is spent on the first login. `UGOS_OTP_SECRET` is the
  TOTP secret (base32, or an `otpauth://` URI) and has no flag: a flag would
  land in the shell history and the process list. A fresh code is generated
  on every login, including after the session expires. Storing the secret next
  to the password means the second factor no longer protects that machine from
  whoever controls it. It still protects against a leaked password. Do not ask
  the user to paste the secret into the chat, and do not write it into the project.
- A session token is valid for about 25 minutes and cached under
  `~/.config/ugos-cli/`. `--no-cache` skips the cache.
- A VM has to be shut off before `vm update`.
