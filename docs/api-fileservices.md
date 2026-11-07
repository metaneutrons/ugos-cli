# File services: SMB, FTP, NFS, rsync, WebDAV

The file service pages of the control panel (`com.ugreen.ctlmgr`). Reconstructed
from the lazily loaded bundle, including the modal chunks, and checked against
a live NAS (UGOS 1.20) on 2026-11-08. Reads were checked for all five. Writes
were checked where they can be undone: settings flipped and restored, FTPS and
WebDAV started and stopped again, with the configuration compared before and
after. Everything else is built from the bundle with unit tests and is marked.

## One pattern for all five

There is no status endpoint. Whether a service runs is a field of
`file/{service}/config`: `status` for SMB, rsync and WebDAV, `enable_ftp` and
`enable_ftps` for FTP, `enableNfsServer` for NFS. A service is switched on, or
its settings are applied, by posting the **complete** configuration to
`file/{service}/start`. It is switched off by posting nothing to
`file/{service}/stop`. UGOS keeps settings only when they come with `start`,
so changing a stopped service is not possible: `ugos-cli` refuses it and says
to start it with the settings. The advanced page (`file/advanced`) has no
`stop`.

`ugos-cli` reads the configuration, edits a copy, posts it and reads it again.
Fields it does not model (SMB has about 40) go back unchanged, so a change
never drops them. A start that UGOS answers with success and does not apply
is reported.

| Method | Path | Wired as | Live |
|--------|------|----------|------|
| GET | `file/{smb,ftp,nfs,rsync,webdav}/config` | `{service} status` | all five |
| POST | `file/{service}/start` | `{service} start`, `{service} set` | SMB, NFS, FTPS, WebDAV |
| POST | `file/{service}/stop` | `{service} stop` | FTP, WebDAV |
| GET | `file/advanced/config` | `smb discovery` | yes |
| POST | `file/advanced/start` | `smb discovery --wsdd2 ...` | no-op only |
| DELETE | `file/smb/cache` | `smb clear-cache` | not called |
| GET | `file/checkport` | before a changed port | via FTP and WebDAV start |

`file_service/*` names are menu keys in the bundle, not calls.

## Settings

Each setting has an alias that maps to a field. `{service} settings` lists them
with what they take. A value is checked the way the web UI checks it before
anything is sent.

- **SMB**: `workgroup` (1 to 15 characters, the web UI strips `[ ] " ; : < > + =
  { } \ / * | ? , -`), `wins` (IP addresses of one family, comma separated),
  `protocols` (`1` SMB1, `2` SMB2 and 3), `encryption` and `signing` (`client`
  0, `off` 1, `force` 2), and the switches of the three tabs of the advanced
  dialog. The dialog's combinations are refused: forced encryption excludes
  multichannel, leases need oplocks, wide links need symbolic links, the veto
  list is `/a/b/` and needed for veto delete, macOS special characters need
  symbolic links. The web UI clears an invalid `wins` silently, `ugos-cli`
  refuses it.
- **FTP**: FTP and FTPS are one service with two switches. Starting without
  choosing turns on FTPS. `timeout` 1 to 7200 s, passive range 1024 to 65535
  (swapped if reversed, reset when the passive switch is off), `per-ip` 2 to
  10, rate limits in KB/s above 0. `enable_ascii`, `enable_fxp`,
  `enable_local_time` and the passive external address have no control in the
  web UI and are passed through.
- **NFS**: only `NFSv3` is offered. Custom statd and nlockmgr ports must
  differ. Every start carries `logs: true` and `enableAdvance: true`, which
  the web UI sends and never reads. Packet sizes, domain and default UNIX
  permissions have no control and are passed through.
- **rsync**: one port (873 when unset) and one account with one password.
  The account must exist and has no spaces. The password is sealed with the
  login key (see [api-users.md](api-users.md)) and sent in `password`; an
  empty `password` keeps the old one. UGOS never returns it. One RSA block
  holds 117 characters; the web UI allows 127 by splitting, and how UGOS reads
  that is not known, so `ugos-cli` stops at 117.
- **WebDAV**: HTTP and HTTPS ports default to 5005 and 5006. At least one
  protocol is needed and the ports must differ.

## Ports

Before a changed port is used, `ugos-cli` applies the web UI's rules (1 to
65535; reserved `80 20 22 23 443 3493` for FTP and the same plus `21` for the
others) and then asks `file/checkport?port=&module=` (`ftp`, `rsync`,
`webdav`, `nfs`, with `name=statd|nlockmgr` for NFS). Success means the port
is free; any API failure means it is taken.

## What needs `--force`

The web UI asks a confirmation for each of these, so `ugos-cli` and the MCP
tools refuse without `--force` or `force`:

| Change | Why |
|--------|-----|
| plain FTP on | names and passwords in the clear |
| NFS on | no login, the address rules of each share decide |
| SMB1 on, `NTLMv1` on | old and weak |
| SMB stop while Bonjour or WS-Discovery are on | the web UI asks first |
| Bonjour or WS-Discovery on while SMB is off | the web UI asks first |
| more than five Time Machine targets | a soft limit of the web UI |
| clear the SMB cache | the network service restarts afterwards |

## Discovery services and Time Machine

`file/advanced/start` takes `{enableWsdd2Service, enableBonjourService,
enableUPnPService, enableTimeMachine, shareFolders: [{fileId, fileName,
select}]}`, with every share in `shareFolders` and `select` for the Time
Machine targets. `enableTimeMachine` is true when at least one is selected.
The NAS tested kept `enableTimeMachine: true` with no target selected, so the
read-back checks the three switches and the targets only.

## Not tried live

rsync start and set (the old password could be lost if UGOS reads an empty
`password` differently), the SMB cache clear, NFS start from off, a changed
FTP port, custom NFS ports, discovery switches that change, Time Machine
targets, SMB protocol and encryption changes (refusals only), SMB stop. What
the codes `8202` and `8203` (SMB start errors) mean beyond "could not start"
is not known, and neither is the wording of the web UI's confirmations (only
their keys are in the bundle).
