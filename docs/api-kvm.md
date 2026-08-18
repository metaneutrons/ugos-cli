# UGOS KVM API Reference

App ID: `com.ugreen.kvm`
Base path: `/ugreen/v1/kvm/`

All endpoints require authentication (see api-auth.md).
All GET endpoints pass parameters as query strings.
All POST endpoints send JSON bodies.

The web UI additionally encrypts every call (`encrypt_query` on GET,
`encrypt_req_body` on POST). That layer is optional — plain query strings and
plain JSON bodies with `?token=` are accepted, which is what this client
sends. A HAR capture of the web UI therefore shows opaque bodies; the UI
bundle at `/kvm/assets/main-*.js` carries the field names in the clear.

## Images (`kvm/image/`)

### UploadUpk
- **Method**: POST, `multipart/form-data`
- **Fields**: `isoName` (display name), `fileName` (name on disk, must be
  free — a taken name fails with `9999`), `size` (total bytes), `chunks`
  (number of parts), `chunk` (0-based index), `file` (the part, sent as
  `filename="blob"`, `application/octet-stream`)
- **Chunk size**: 10 MiB, as used by the web UI
- **Response**: `{result: "successful"}` per chunk

Verified against a live NAS (2026-08-18) with a 25 MiB file in three parts.
The web UI generates a random `fileName`; a readable one works just as well
as long as it is unique.

### UploadPath
- **Method**: POST
- **Body**: `{path, imageName, fileName}` — registers a file that already
  sits on the NAS instead of uploading it.

Taken from the web UI bundle and not verified: the only files reachable for a
test were already registered, and a taken `fileName` answers `9999`.

### DeleteImage
- **Method**: GET
- **Params**: `fileName=<file>&imageName=<display name>`

`imageName`, not `name` — with the wrong key the call answers `9999`.

### RenameImage
- **Method**: POST

Exists, but every body tried answers `successful` and renames nothing, and
the web UI never calls it. The field names are unknown; not wrapped.

## Snapshots (`kvm/manager/`)

Verified against a live NAS (2026-08-18). Every one of these takes different
parameters than the naming suggests.

### GenerateSnapshot
- **Method**: GET
- **Params**: `name=<vm uuid>&virtualMachineDisplayName=<vm display name>`
- **Response**: `{snapshotDisplayName: "2026-08-18 08:40:50"}`

`name` is the **VM's** UUID, not a snapshot name. The caller does not name a
snapshot at all — UGOS names it after the creation time and reports it back.
Passing a snapshot name here fails with `3010`. Other codes seen in the web
UI: `3027` (a snapshot is already being taken) and `3012` (limit reached).

### ShowListSnapshot
- **Method**: GET
- **Params**: `name=<vm uuid>`
- **Response**: `{result: [{name, displayName, createTime, description, id, virName, screenshot}]}`

`createTime` is a **string** (`YYYY-MM-DD HH:MM:SS`), not a timestamp, and
`name` is `<vm-uuid>_<unix-time>`.

### DeleteSnapshot
- **Method**: GET
- **Params**: `name=<snapshot name>&virtualMachineDisplayName=<snapshot display name>`

Both parameters describe the **snapshot**; despite its name the second one is
not the VM.

### RevertSnapshot
- **Method**: GET
- **Params**: `virtualMachineName=<vm uuid>&snapshotName=<snapshot name>&createSnapshot=<bool>`

`createSnapshot` snapshots the current state before reverting.

### EditSnapshot
- **Method**: POST
- **Body**: `{name: "<snapshot name>", description: "<text>"}`

Sets the description. There is no rename: `RenameSnapshot` exists and answers
`successful`, but the web UI uses `EditSnapshot` and only for the description.

## VM Manager (`kvm/manager/`)

### ShowLocalVirtualList
- **Method**: GET
- **Params**: none
- **Response**: `{result: [VmSummary]}`

```json
// VmSummary
{
  "virName": "797fb54f-...",        // UUID, used as identifier
  "virDisplayName": "CachyOS",
  "virID": 4,                       // gone since app build 656 — treat as optional
  "graphic": "",                    // undocumented, present since build 656
  "storageName": "Volume 1",
  "systemType": "linux",            // "linux" | "windows" | "other"
  "systemVersion": "",              // "win11" | "" etc.
  "guestCpuPercent": 2,
  "guestTotalMemory": 25165824,     // KiB
  "guestUsedMemory": 18958724,
  "hostCpuPercent": 2,
  "hostUsedMemory": 19497880,
  "hostTotalMemory": 65581740,
  "upload": 0,                      // bytes/s
  "download": 590,
  "status": "running",              // "running" | "shutoff"
  "processStatus": "createSuccess",
  "progress": 0,
  "createTime": 1775925041          // unix timestamp
}
```

### ShowLocalVirtualMachine
- **Method**: GET
- **Params**: `name=<uuid>`
- **Response**: `{data: VmDetail}`

```json
// VmDetail
{
  "virtualMachineName": "797fb54f-...",
  "virtualMachineDisplayName": "CachyOS",
  "systemType": "linux",
  "systemVersion": "",
  "core": {"value": 12},
  "memory": {"value": 25165824},    // KiB
  "images": [
    {"path": "/volume1/@appstore/com.ugreen.kvm/iso/CachyOS.iso", "dev": "hda", "order": 2}
  ],
  "dists": [
    // size is KiB, like memory. usedBytes is read-only.
    {"bus": "virtio", "size": 1048576000, "dev": "vda", "path": "...qcow2",
     "order": 1, "usedBytes": 251928576}
  ],
  "networks": [
    {"name": "vnet-bridge0", "macAddress": "52:54:00:d7:38:b5", "type": "virtio"}
  ],
  "device": {
    "usbController": 2,
    "usbDevices": [],
    "graphicsCard": "virtio",
    "bootType": "uefi"              // "uefi" | "bios"
  },
  "otherConfig": {
    "autoMaticStartUp": true,
    "keyboardLanguage": "de",
    "shareDirectory": []
  },
  "storageName": "volume1",
  "storageUUID": "Csuzof-...",      // required by CreateVirtualMachine
  "ovaPath": "",
  "guestToolStatus": 0
}
```

The read endpoint also returns `device.pciPassthroughDevices` (null when
unused) and `images[].name`; neither is required in a write body.

### ShowNativeInfo
- **Method**: GET
- **Params**: none
- **Response**: `{data: {cores: 12, memory: 67155701760}}`

### CheckResource
- **Method**: GET
- **Params**: `memory=<bytes>`

### CheckVirName
- **Method**: POST
- **Body**: `{name: "<uuid>", virtualMachineDisplayName: "<displayName>"}`

### CreateVirtualMachine
- **Method**: POST
- **Body**: VmDetail object (same schema as ShowLocalVirtualMachine response)
- **Timeout**: unlimited

Verified against a live NAS (app build 656, 2026-08-18):

- `storageUUID` is **required**. Without it: `3000, Fail to create virtual
  machine`.
- `virtualMachineName` may be empty and is ignored either way — UGOS assigns
  the UUID. Read it back from `ShowLocalVirtualList`.
- `dists[].size` is **KiB**, not bytes. A body in bytes is accepted, and the
  domain then fails to start.
- `otherConfig.keyboardLanguage` must be a QEMU keymap (`en-us`, `de`, …).
  A plain `en` is accepted here and makes every later `PowerOn` fail with
  `3037`.
- The web UI omits `dists[].dev` and `dists[].path` and lets UGOS assign both.

The web UI form defaults, per OS type: Windows `ide` disk (60 GiB) plus an
`e1000e` NIC, Linux `virtio` (20 GiB), other `ide` (10 GiB) plus `e1000e`.

### UpdateVirtualMachine
- **Method**: POST
- **Body**: VmDetail object

- The VM must be shut off, otherwise `3002, Fail to edit virtual machine`.
- A **newly added** disk must carry neither `dev` nor `path` — with either of
  them the call fails with `3002`. Existing disks keep both.
- The web UI drops `storageUUID`, `systemVersion`, `ovaPath`,
  `guestToolStatus` and `macAddress` from the body here; sending them anyway
  works.

### DeleteVirtualMachine
- **Method**: GET
- **Params**: `name=<uuid>&virtualMachineDisplayName=<displayName>`

### PowerOn
- **Method**: GET
- **Params**: `name=<uuid>&virtualMachineDisplayName=<displayName>`

### Shutdown
- **Method**: GET
- **Params**: `name=<uuid>&virtualMachineDisplayName=<displayName>`

### ForcedShutdown
- **Method**: GET
- **Params**: `name=<uuid>&virtualMachineDisplayName=<displayName>`

### Reboot
- **Method**: GET
- **Params**: `name=<uuid>`

### ForcedRestart
- **Method**: GET
- **Params**: `name=<uuid>&virtualMachineDisplayName=<displayName>`

### ExportOVA
- **Method**: POST
- **Body**: `{virtualName: "<uuid>", storageName: "<name>", storageUUID: "<uuid>", ovaPath: "<path>"}`
- **Timeout**: unlimited

### ParseOvaFile
- **Method**: POST
- **Body**: `{ovaPath: "<path>"}`

## Snapshots (`kvm/manager/`)

### ShowListSnapshot
- **Method**: GET
- **Params**: `name=<vm-uuid>`
- **Response**: `{result: [Snapshot]}`

### GenerateSnapshot
- **Method**: GET
- **Params**: `name=<snapshotName>&virName=<vm-uuid>&virtualMachineDisplayName=<displayName>`
- **Timeout**: unlimited

### DeleteSnapshot
- **Method**: GET
- **Params**: `name=<snapshotName>&virName=<vm-uuid>`

### RevertSnapshot
- **Method**: GET
- **Params**: `name=<snapshotName>`
- **Timeout**: unlimited

### RenameSnapshot
- **Method**: POST
- **Body**: `{name: "<snapshotName>", displayName: "<newDisplayName>"}`

## Network (`kvm/network/`)

### ShowNetworkList
- **Method**: GET
- **Response**: `{result: [NetworkSummary]}`

```json
// NetworkSummary
{
  "networkName": "vnet-bridge0",
  "networkUUID": "8171cf35-...",
  "networkLabel": "VBR-LAN1",
  "networkValid": true,
  "networkType": "bridge",          // "bridge" | "nat" | "none"
  "networkMode": "bridge",
  "interfaceName": "bridge0",
  "virtualDisplayNames": ["CachyOS", "Worf"],
  "createTime": 0,
  "systemNetwork": true
}
```

### GetNetworkByName
- **Method**: GET
- **Params**: `name=<networkName>`
- **Response**: `{result: NetworkDetail}`

```json
// NetworkDetail
{
  "networkUUID": "...",
  "networkName": "vnet-bridge0",
  "networkType": "bridge",
  "networkMode": "bridge",
  "mappingNetwork": "bridge0",
  "enableIpv4": false,
  "allocateIpv4": false,
  "ipv4Subnet": "",
  "ipv4Gateway": "",
  "ipv4DHCPStartIp": "",
  "ipv4DHCPEndIp": "",
  "enableIpv6": false,
  "ipv6Subnet": "",
  "ipv6Gateway": "",
  "ipv6DHCPStartIp": "",
  "ipv6DHCPEndIp": "",
  "allocateIpv6": false
}
```

### CheckNetwork
- **Method**: GET
- **Params**: `name=<networkName>`

### CheckName
- **Method**: POST
- **Body**: `{networkName: "<name>"}`

### CreateNetwork
- **Method**: POST
- **Body**: NetworkDetail object

### UpdateNetwork
- **Method**: POST
- **Body**: NetworkDetail object

### DeleteNetwork
- **Method**: GET
- **Params**: `name=<networkName>`

### ChangeBridgeMode
- **Method**: POST
- **Body**: `{interface: "<interfaceName>"}`

## Storage (`kvm/storage/`)

### ShowStorageList
- **Method**: GET
- **Response**: `{result: [StorageInfo]}`

```json
// StorageInfo
{
  "name": "volume1",
  "label": "Volume 1",
  "health": 0,
  "status": 0,
  "totalCapacity": 23955125567488,
  "availableCapacity": 21626153811968,
  "uuid": "Csuzof-yZcf-...",
  "path": "/volume1",
  "filesystem": "btrfs",
  "describe": ""
}
```

### ShowLocalStorageList
- **Method**: GET
- **Response**: same as ShowStorageList

### CheckStorage
- **Method**: GET
- **Params**: `name=<name>&uuid=<uuid>`
- **Response**: `{result: ["VM1", "VM2"]}` (VMs using this storage)

### AddStorage
- **Method**: POST
- **Body**: `{storageName: "<name>", storageUUID: "<uuid>"}`

### DeleteStorage
- **Method**: GET
- **Params**: `name=<name>&uuid=<uuid>`

## Image (`kvm/image/`)

### ShowImageList
- **Method**: GET
- **Response**: `{result: [ImageInfo]}`

```json
// ImageInfo
{
  "id": 1,
  "fileName": "EndavourOS.iso",
  "imageName": "EndavourOS",
  "fileSize": 3498065920,
  "progress": 0,
  "state": "completed",
  "imageType": "iso",
  "path": "/volume1/@appstore/com.ugreen.kvm/iso/EndavourOS.iso",
  "virtualDiskSize": 0
}
```

### CheckImageName
- **Method**: GET
- **Params**: `name=<name>`

### CheckImageUsage
- **Method**: GET
- **Params**: `name=<name>`
- **Response**: `{result: []}` (VMs using this image)

### DeleteImage
- **Method**: GET
- **Params**: `fileName=<fileName>&name=<imageName>`

### UploadPath
- **Method**: POST
- **Body**: FormData

### UploadUpk
- **Method**: POST
- **Body**: FormData (Content-Type: application/x-www-form-urlencoded)

## USB (`kvm/usb/`)

### USBList
- **Method**: GET
- **Params**: `vmName=<vm-uuid>`
- **Response**: `{result: [UsbDevice]}`

```json
// UsbDevice
{
  "vendorID": "0x8087",
  "vendorName": "Intel Corp.",
  "productID": "0x0033",
  "productName": "AX211 Bluetooth",
  "busID": 3,
  "deviceID": 2,
  "usedBy": ""
}
```

## VNC (`kvm/vnc/`)

### ListAllLink
- **Method**: GET
- **Params**: `virName=<vm-uuid>`
- **Response**: `{result: [VncLink]}`

### GenerateNoVNClink
- **Method**: POST
- **Body**: `{virName: "<uuid>", type: 0, sourceUrl: "<baseURL>"}`

### CreateLink
- **Method**: POST
- **Body**: `{virName: "<uuid>", apiKey: "<key>"}`

### UpdateLink
- **Method**: POST
- **Body**: `{virName: "<uuid>", apiKey: "<key>", password: "<password>"}`

### CheckUgLinkStatus
- **Method**: GET
- **Params**: `virName=<uuid>` (likely)

### DeleteLink
- **Method**: GET
- **Params**: `virName=<uuid>` (likely)

## Logs (`kvm/logs/`)

### PageSearchLogs
- **Method**: POST
- **Body**:
```json
{
  "pageNum": 1,
  "pageSize": 20,
  "operator": "",
  "startTime": "",
  "endTime": "",
  "createTimeSort": "desc",
  "operatorSort": ""
}
```

### GetAllOperator
- **Method**: GET
- **Response**: `{result: ["fabian"]}`

### DeleteLogs
- **Method**: GET
- **Params**: (unknown, likely log IDs)

## User Preferences (`kvm/user/`)

### UserPreference
- **Method**: GET
- **Response**: `{data: {risk_warning: {usb_popup_accepted: false}}}`

### UpdateUserPreference
- **Method**: POST
- **Body**: `{risk_warning: {usb_popup_accepted: true}}`
