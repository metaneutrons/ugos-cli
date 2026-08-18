# UGOS Core API Reference

Base path: `/ugreen/v1/`

These endpoints live outside the KVM and Docker apps and need no app id. Two
things differ from the app APIs:

- Field names are **`snake_case`**, not camelCase.
- Responses are returned as the payload directly, without a `result` wrapper.

Verified against a live NAS (UGOS 1.18.1.0098, 2026-08-18).

## Machine info (`sysinfo/`)

### machine/common
- **Method**: GET
- **Params**: none
- **Response**: `{common: {...}, hardware: {...}}`

```json
{
  "common": {
    "nas_name": "nas1",
    "model": "DXP480T Plus",
    "model_series": "dxp",
    "product_series": "nasync",
    "serial": "…",
    "system_version": "1.18.1.0098",
    "beta": false,
    "last_turn_on_time": "2026-08-09 17:26:41",
    "run_time": 772730,               // seconds since boot
    "nas_owner": "-"
  },
  "hardware": {
    "cpu": [{"model": "…", "core": 10, "thread": 12, "ghz": 4400, "temperature": 60}],
    "mem": [{"manufacturer": "…", "model": "…", "size": 34359738368, "mhz": "5600 MHz", "is_ecc": false}],
    "net": [{"model": "bridge0", "ip": "…", "mac": "…", "mask": "…", "mtu": 1500, "speed": 10000}],
    "gpu": null, "ups": null, "usb": null
  }
}
```

`ghz` is MHz despite the name. Absent hardware arrives as `null`, not as an
empty list.

## Monitoring (`taskmgr/`)

### stat/overview
- **Method**: GET
- **Params**: none
- **Response**: one sample per subsystem, each wrapped in a list

Keys: `cpu` (`used_percent`, `temp`), `mem` (`used_percent`), `disk` and
`volume` (`read_rate`, `write_rate`, `used_percent`), `net` (`recv_rate`,
`send_rate`), `cpu_fan` and `device_fan` (`speed`, `status`), `gpu`.

**The `volume` figures are totals since boot, not rates**, despite the field
names: measured five seconds apart, `write_rate` grew by exactly the bytes
written in between. The `disk` figures read as rates, but that could not be
confirmed on an idle NAS. A GPU entry is present with empty values even when
no GPU is installed.

### processes
- **Method**: GET
- **Response**: `{list: [{pid, name, desc, status, process_status, can_be_operated, consume}], total_consume: {...}}`

`consume` and `total_consume` share a shape: `cpu_used_percent`, `mem_used`,
`mem_used_percent`, `disk_read_speed`, `disk_write_speed`, `net_recv_speed`,
`net_send_speed`, `gpu_used_percent`.

### services
- **Method**: GET
- **Response**: `{list: [{id, appid, name, icon_path, can_be_operated, consume}], total_consume: {...}}`

A "service" is an installed app, e.g. `snapshot_serv` / `com.ugreen.snapshot`.

### Not wrapped

`processes/stop` and `services/processes` exist but are not implemented:
stopping a process on a NAS deserves an explicit decision, not a convenience
wrapper.

## System log (`log/`)

### query
- **Method**: GET, plain
- **Params**: `page`, **`size`**, plus optional `module`, `level`,
  `operator`, `keyword`
- **Response**: `{log_list: [...] | null, total, cur_page}`

The page size parameter is **`size`**. `limit`, `page_size` and `pageSize`
are all accepted and silently ignored, leaving the default of 20 — which
looks like a working request returning the wrong number of rows.

Entries: `content`, `level`, `module`, `operator`, `create_time`, `log_id`.
The log spans every module, so a Download Center task and a login appear side
by side. This is a different log from the KVM app's own audit trail
(`kvm/logs/PageSearchLogs`), which the CLI exposes as `vm log`.

## Users (`user/`)

### list
- **Method**: GET, plain
- **Response**: `{list: [...]}`

### current/user
- **Method**: GET, plain
- **Response**: the account behind the session

`uid` is a **number**, not a string. Email addresses come back masked
(`f****n@example.org`). Note that `user/admin/list` exists but answers
`1008, Standard users do not have permission to access this interface` for a
non-admin account — `user/list` works for everyone.
