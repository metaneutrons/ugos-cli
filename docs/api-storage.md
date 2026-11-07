# Storage Manager

Pools, volumes and disks of the NAS itself (`com.ugreen.storagemgr`). This is
not the KVM storage of `ugos-cli storage`, which lists the places VMs keep
disk images.

Reconstructed from the app's bundle and read against a live NAS on UGOS 1.20 on
2026-11-07. **Only reading is wired.** Creating, extending, repairing and
deleting pools and volumes, replacing and erasing disks and starting tests
change data or can destroy it, and are left to the web UI for now.

## Wired

| Method | Path | Transport | Command |
|--------|------|-----------|---------|
| GET | `storage/pool/list` | plain | `pool list`, `pool show` |
| GET | `storage/volume/list` | plain | `volume list`, `volume show` |
| GET | `v2/storage/disk/list` | encrypted | `disk list`, `disk show` |
| GET | `storage/disk/attribute?disk=` | plain | `disk show` |
| GET | `storage/disk/smart/info?disk=` | plain | `disk smart` |
| GET | `storage/disk/detection/list?disk=` | plain | `disk history` |
| GET | `storage/disk/cron/list` | plain | `disk plans` |

`disk` is the device path, e.g. `/dev/nvme0n1`. The commands accept the short
name, the slot (`nvme1`) or the label as well and look the path up.

## Pools

`{result: [pool]}`. A pool has `name` (`pool1`), `label`, `level` (`raid5`),
`status` (`0` when healthy; other codes were not seen), `total`, `used`, `free`
in bytes, `disks` (`dev_name`, `slot`, `size`, `status`), `volumes`, `spare`
(`null` for none), and the settings of the jobs on it:

- `trim`: `enable`, `is_support`, `next_start_time`, `scheduled`.
- `data_scrub`: `is_cron`, `last_time` and `next_start_time` (`--` for never),
  `process`, `status`.
- `support_delete`, `support_extend`, `support_hot_spare`, `support_ssd_cache`,
  `support_replace`, `support_pool_fix`, `support_pv_extend` and
  `support_upgrade_raid` say what the web UI would offer.

`used` of a pool that holds volumes equals its `total`: the pool is allocated
to them. The usage that matters is on the volume.

## Volumes

`{result: [volume]}` with `name` (`volume1`), `label`, `poolname`, `device`,
`filesystem` (`btrfs`), `mntpath` and `mntpaths` (`/volume1` and `/home` for the
volume that holds the home folders), `total`, `used`, `available`, `status`,
`health`, `is_home`, `ready_to_use`, `space_alert_percent`, and the same kind of
`support_*` switches.

## Disks

`v2/storage/disk/list` answers `{result: [disk]}` with `name`, `dev_name`,
`label`, `slot`, `brand`, `model`, `serial`, `size`, `interface_type`, `status`,
`temperature`, `power_on_hours`, `used_for` (the pool), `activate`,
`is_standby`, `is_support_smart`. `disk/attribute` adds `firmware`,
`standard_agreement` (`NVMe`), `power_on_count`.

`disk/smart/info` answers `is_testing`, `progress`, `remain_time`,
`short_test_cost` and `long_test_cost` (minutes), `last` (`last_time`,
`next_time`, `status`) and `report`, a list of `{id, label, name, value}`. For an
NVMe disk the report has temperature, available spare and its threshold, data
units read and written, host reads and writes, controller busy time, power
cycles, power-on hours, unsafe shutdowns, error log entries and the
temperature sensors as a list.

`disk/detection/list` is the test history: `time`, `smart_type` (`0` short, `1`
long), `status` (`1` for a pass), `err_code`. `disk/cron/list` is the test
plans: `cron_id`, `name`, `next_time`, `disks`, `schedule` (`interval` and a unit
code that the API does not name) and `smart.enable`.

## Probed, not wired

| Path | What it answers |
|------|-----------------|
| `disk/runningState?disk=` | per month: unsafe shutdowns, bad blocks, reconnects |
| `disk/vars?disk=` | temperature of every disk |
| `disk/canUseFor?disk=` | what the disk could be used for (add to a pool, cache, spare, import) |
| `disk/slotnum`, `disk/setting/get`, `pool/setting/get` | slot counts, bad block check, RAID sync speed limit and bitmap |
| `volume/spaceUsage?poolName=&volumeName=` | timed out after 30 s on a 22 TiB volume: it scans |
| `cache/list?name=`, `extStorage/list` | empty on the NAS tested (no cache, no external storage) |
| `disk/iops`, `volume/iops`, `volume/setting`, `pool/operate/disks` | `9999, Failed to operate` without the parameters the web UI sends |

## Not wired, known from the bundle

Changing calls, all `POST`: `pool/create`, `pool/delete`, `pool/ext`,
`pool/up`, `pool/fix`, `pool/import`, `pool/scrub`, `pool/scrub/edit`,
`pool/scrub/stop`, `pool/trim/set`, `pool/setting/set`, `spare/create`,
`spare/delete`, `cache/create`, `cache/createWithPool`, `cache/delete`,
`volume/createWithPool`, `volume/createWithDisk`, `volume/delete`,
`volume/grow`, `volume/balance`, `volume/defrag`, `volume/fsrepair`,
`volume/setting`, `disk/erase`, `disk/activate`, `disk/deactivate`,
`disk/unlock`, `disk/smart/start`, `disk/smart/stop`, `disk/benchmark/*`,
`disk/cron/add`, `disk/cron/edit`, `disk/cron/del`, `disk/setting/set`,
`extStorage/formatter`, `extStorage/remove`.

## Not known

- What the status and health codes other than `0` mean.
- The unit codes of a schedule's `interval_unit`.
