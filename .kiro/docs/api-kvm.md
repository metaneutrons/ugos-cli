# UGOS KVM API Reference

App ID: `com.ugreen.kvm`
Base path: `/ugreen/v1/kvm/`

All endpoints require authentication (see api-auth.md).
All GET endpoints pass parameters as query strings.
All POST endpoints send JSON bodies.

## VM Manager (`kvm/manager/`)

### ShowLocalVirtualList
- **Method**: GET
- **Params**: none
- **Response**: `{result: [VmSummary]}`

```json
// VmSummary
{
  "virName": "797fb54f-...",        // UUID, used as identifier
  "virID": 4,
  "virDisplayName": "CachyOS",
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
    {"bus": "virtio", "size": 1048576000, "dev": "vda", "path": "...qcow2", "order": 1}
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
  "ovaPath": ""
}
```

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

### UpdateVirtualMachine
- **Method**: POST
- **Body**: VmDetail object

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
