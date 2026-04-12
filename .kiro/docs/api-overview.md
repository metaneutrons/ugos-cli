# UGOS API Overview

## Discovered API Modules

Base URL: `https://<nas-ip>:9443/ugreen/v1/`

### Fully Documented (from JS reverse-engineering)

| Module | App ID | Status | Doc |
|--------|--------|--------|-----|
| **Auth** | core | Complete | [api-auth.md](api-auth.md) |
| **KVM** | com.ugreen.kvm | Complete | [api-kvm.md](api-kvm.md) |

### Known but Undocumented (endpoints found in main JS bundle)

| Module | Path Prefix | Notes |
|--------|-------------|-------|
| File Manager | `filemgr/` | Upload, download, thumbnails, path checks |
| Docker | `docker/container/`, `docker/log/` | Container management, export, import, logs |
| Photo | `photo/` | Upload, stream, AI auto-learn |
| Video | `video/details/` | Metadata editing, actor info |
| Music | `music/` | Upload, playlist sharing |
| Backup | `backup/` | Backup jobs |
| Network | `network/iface/`, `network/auth/` | NAS network config (not KVM networks) |
| User | `user/` | User management, password, avatar, wallpaper |
| Firmware | `firmware/`, `upgrader/` | System updates, manual firmware upload |
| Security | `security/ssl/` | SSL certificate import |
| Download Center | `downloadCenter/` | Download management |
| Sync | `sync/syncthing/`, `web/sync/` | Syncthing integration |
| Hardware | `hardware/ups/` | UPS firmware |
| Time | `time/config` | Time configuration |
| Office | `office/` | Document editing (OnlyOffice) |
| Vault | `vault/` | Password vault |
| Assistant | `assistant/chat/` | AI chat assistant |
| Help Center | `helpcenter/` | Diagnostics, articles |
| Antivirus | `antivirus/` | Scan, logs |
| Editor | `editor/` | Text editor |
| Version Explorer | `versionExplorer/` | File versioning |
| Cloud Drives | `baidu/`, `onedrive/`, `netDisk/` | Cloud storage integration |
| Stream | `stream/transcode/` | Media transcoding |
| Connect | `connect/` | Remote access, wallpaper |
| Wizard | `wizard/` | Initial setup |

### Non-KVM APIs Used by KVM App

These system APIs are called from within the KVM app:

| Endpoint | Method | Params |
|----------|--------|--------|
| `network/iface/list?interface=vm` | GET | — |
| `network/iface/get_mode` | GET | `interface=<name>` |
| `time/config` | GET | — |
| `user/current/user` | GET | — |
| `filemgr/pathExist` | POST | `{path: "<path>"}` |

## Discovery Method

APIs were reverse-engineered from the UGOS web UI JavaScript bundles:
- Main desktop: `https://<ip>:9443/desktop/static/main-*.js` (3.3MB)
- KVM app: `https://<ip>:9443/kvm/js/689.*.js` (282KB)
- KVM framework: `https://<ip>:9443/kvm/js/ugos-framework.*.js` (252KB)

Each UGOS app is a separate Vue.js SPA served under its own path (`/kvm/`, `/docker/`, etc.) with lazy-loaded JS chunks. To document a new module, download its JS bundle and extract API calls.

## UGOS Version Tested

- **System**: 1.14.1.0107
- **KVM App**: build 899 (7418da87#282), 2026-03-11
- **Hardware**: DXP480T Plus (picard)
