# Hardware, power, time, language and terminal

The hardware panel of the control panel (`com.ugreen.ctlmgr`), plus the time,
language, SSH/Telnet, host name and finder settings. Reconstructed from the
lazily loaded bundle and verified against a live NAS (UGOS 1.20) on
2026-11-08. Reads were checked live. Writes were checked live only where
they could be undone (a no-op write of the value already set, read back).
Everything else was built from the bundle and unit tests, and is marked below.

UGOS answers success and does nothing for many malformed writes, so
`ugos-cli` reads every change back and says so when it did not take.

## Endpoints

| Area | Path | Wired as | Live |
|------|------|----------|------|
| Buzzer | `hardware/buzzer/config`, `start`, `testBuzzer` | `hw buzzer` | read, no-op write |
| CPU | `hardware/cpu/config`, `frequency` | `hw cpu` | read, no-op write |
| Fan | `hardware/fan/config`, `setfan`, `start` | `hw fan` | read, auto |
| LEDs | `hardware/led/config`, `start`, `stop` | `hw led` | read, on |
| Display | `hardware/ledscreen/settings`, `wallpaperlist` | `hw screen` | read, no-op write |
| Memory compression | `hardware/zram/config`, `start`, `stop` | `hw zram` | read, refusal |
| Power | `hardware/power/config`, `start` | `hw power` | read, schedule add and remove |
| UPS | `hardware/ups/config`, `apply`, `start`, `stop`, `connection/check`, `snmp/mibs`, `firmware/info` | `hw ups` | **read only** |
| GPU | `hardware/gpu/list`, `setmode`, `statement`, `statement/agree` | `hw gpu` | read (none present) |
| Time | `time`, `time/config`, `time/server`, `time/server/delete`, `time/verify/cloud/certificate` | `time` | read, server no-op |
| Language | `language`, `language/global`, `language/list` | `lang` | read, auto |
| Terminal | `terminal/get`, `terminal/set` | `terminal` | read, SSH on (already on) |
| Reboot | POST `desktop/reboot` | follows zram off and GPU mode | not called |

The host name and the finder switches are part of the time and system
endpoints and are wired as `system hostname` and `system finder`.

## What cost time

- **The fan curve** has 17 points from 20 to 100 percent at 0 to 80 °C. From
  55 °C the CPU fan is forced to 100, whatever is sent, so `ugos-cli` does the
  same before the read-back. A NAS without the feature answers a curve with
  success and ignores it; `ugos-cli` refuses first.
- **The power schedule** is a list of entries plus a flag. Removing the last
  entry leaves the flag on, which the web UI does not allow, so `ugos-cli`
  turns it off with the last entry. Changing the schedule sends 15 keys; the
  scopes the panel excludes are left out.
- **The UPS** is applied in three steps (stop, config, start) or by `apply`,
  depending on whether the service runs. The settings body depends on the
  source (USB, SNMP, network); fields of other sources are masked out. A
  network UPS needs a NUT server at the address, checked with
  `connection/check`. **Not tried live:** the NAS tested has a real UPS in
  use.
- **GPU mode** needs the passthrough disclaimer accepted and restarts the NAS.
- **Memory compression off** restarts the NAS.
- **Time:** setting the clock by hand or the zone may invalidate the cloud
  certificate. `time/verify/cloud/certificate` says so first, and `ugos-cli`
  refuses unless `--force` is given. The time is shown in UTC.
- **SSH and Telnet:** the reply of `terminal/get` has capitalised keys
  (`SshConfig`, `TelnetConfig`), the write uses camelCase. Switching a service
  on needs `--force`, Telnet sends passwords in the clear. A changed port is
  checked against the ports in use.

## Guards

| Change | Guard |
|--------|-------|
| zram off, GPU mode | `--force`, the NAS restarts |
| manual time, zone | `--force` if the cloud certificate would break |
| SSH or Telnet on | `--force` |
| fan curve | refused on a model without it |
| host name | validated as the web UI does |

The MCP tools do the same and take `force` where the CLI takes `--force`; the
text of the refusal tells the model to ask the user first.

## Not tried live

UPS set and connect, GPU mode, memory compression off, manual time, enabling
SSH from off and Telnet, a custom fan curve (the NAS has no support), and a
power schedule beyond adding and removing an entry.
