# UGOS Docker API Reference

App ID: `com.ugreen.docker`
Base path: `/ugreen/v1/docker/`
App version: 3284 (b0951efc#530), 2026-03-17

All endpoints require authentication (see api-auth.md).

## Overview (`docker/view/`)

### ObtainOverviewInfo
- **Method**: GET
- **Response**: `{data: OverviewInfo}`

```json
{
  "containerCount": 0,
  "runContainerCount": 0,
  "imageCount": 0,
  "memoryUsed": 22342995968,
  "totalMemory": 67155701760,
  "containerMemory": 0,
  "containerTotalMemory": 67155701760,
  "cpuUsed": 2,
  "containerCpuUsed": 0,
  "dataVolume": -1,
  "status": true,
  "overviewContainers": null,
  "projectCounr": 0,
  "runProjectCounr": 0
}
```

### GetEngineStatus
- **Method**: GET
- **Response**: `{result: "online"}`

### ShowContainerList (view)
- **Method**: GET
- **Params**: `form=<form>`

### RestoreEngine
- **Method**: GET

### GetTutorial
- **Method**: GET
- **Params**: `pageNum=0&pageSize=4`

## Container Management (`docker/container/`)

### ContainerListV2
- **Method**: POST
- **Body**: `{pageNum: 1, pageSize: 20}`
- **Response**: `{data: {originalTotal: N, result: [Container], total: N}}`

### ShowContainerList
- **Method**: POST
- **Body**: `{pageNum: 1, pageSize: 20}` (required)

### ShowContainerDetailListV2
- **Method**: POST
- **Body**: (container filter params)

### GetContainerById
- **Method**: GET
- **Params**: `containerId=<id>`

### ShowLocalContainer
- **Method**: GET
- **Params**: `containerId=<id>`

### ShowOfflineContainer
- **Method**: GET
- **Params**: `containerId=<id>`

### CreateContainer
- **Method**: POST
- **Body**: Container spec object

### UpdateContainer
- **Method**: POST
- **Body**: Container spec object

### UpdateContainerBase
- **Method**: POST
- **Body**: Container base config

### CloneContainer
- **Method**: POST
- **Body**: Clone spec

### RemoveContainer
- **Method**: POST
- **Body**: `{containerId: "<id>"}` (likely)

### StartContainer
- **Method**: GET
- **Params**: `containerId=<id>`

### StopContainer
- **Method**: GET
- **Params**: `containerId=<id>`

### RestartContainer
- **Method**: GET
- **Params**: `containerId=<id>`

### ContainerKill
- **Method**: GET
- **Params**: `containerId=<id>`

### BatchOperateContainer
- **Method**: POST
- **Body**: Batch operation spec

### CheckContainerName
- **Method**: GET
- **Params**: `containerName=<name>`

### GetContainerName
- **Method**: GET
- **Params**: `opType=<type>&name=<name>`

### CheckPort
- **Method**: POST
- **Body**: Port check spec

### AllocatePort
- **Method**: POST
- **Body**: Port allocation spec

### CheckSource
- **Method**: POST
- **Body**: Source check spec

### CheckNasPath
- **Method**: POST
- **Body**: Path check spec

### HasIntegratedGPU
- **Method**: GET
- **Response**: `{result: true}`

### ShowCpu
- **Method**: GET
- **Response**: `{data: CpuInfo}` (detailed CPU info with model, cores, flags)

### ShowMemory
- **Method**: GET
- **Response**: `{data: MemoryInfo}` (detailed memory stats)

## Container Logs (`docker/container/`)

### ShowContainerLogs
- **Method**: POST
- **Body**: Log query params

## Container Export/Import

### ExportContainer
- **Method**: POST
- **Body**: Export spec

### ContainerExportImage2Path
- **Method**: POST
- **Body**: Export image to path spec

### ParseFile
- **Method**: POST
- **Content-Type**: `application/x-www-form-urlencoded`
- **Body**: FormData with file

### ParseFileFromNas
- **Method**: POST
- **Body**: NAS path spec

### ContainerTemplate
- **Method**: POST
- **Body**: Template spec

## Container Upgrade

### UpgradeNeed
- **Method**: GET
- **Params**: `containerId=<id>`

### VersionUpgrade
- **Method**: POST
- **Body**: Upgrade spec

### ContainerUpgradeByTag
- **Method**: POST
- **Body**: Tag upgrade spec

### GetVersionUpgradePath
- **Method**: GET

### GetUpdateContainerCount
- **Method**: GET

### UpdateContainerImage
- **Method**: POST
- **Body**: Image update spec

### CancelUpdateContainerImage
- **Method**: POST
- **Body**: Cancel spec

## Container Terminal (`docker/container/`)

### GetTerminalList
- **Method**: GET
- **Params**: `containerId=<id>&processCheck=<bool>`

### AddTerminal
- **Method**: POST
- **Body**: Terminal spec

### DelTerminal
- **Method**: POST
- **Body**: Terminal spec

### ChangeTerminalClientID
- **Method**: POST
- **Body**: Client ID spec

### RecommendedTerminalCommands
- **Method**: GET

## Container Access

### GetContainerAccessLinkInfo
- **Method**: GET
- **Params**: `port=<port>`

### GetFilePathHistory
- **Method**: GET

### SetFilePathHistory
- **Method**: POST
- **Body**: Path history spec

## Image Management (`docker/image/`)

### ShowLocalImageV2
- **Method**: POST
- **Body**: `{pageNum: 1, pageSize: 20}` (likely)
- **Response**: `{data: {originalTotal: N, result: [Image]}}`

### SearchImage
- **Method**: GET
- **Params**: `name=<name>&pageNum=<n>&pageSize=<n>`

### QueryVersionNumber
- **Method**: GET
- **Params**: `name=<name>&tag=<tag>&page=<n>&pageSize=<n>`

### DownloadImage
- **Method**: POST
- **Body**: Download spec

### ObtainAllImages
- **Method**: POST
- **Body**: Filter params

### ObtainCommonImags
- **Method**: POST
- **Body**: Filter params

### ObtainPrivateIsOfficial
- **Method**: POST
- **Body**: Image check params

### GetCommonList
- **Method**: POST
- **Body**: Filter params

### QueryAllContaners
- **Method**: POST
- **Body**: Image filter (which containers use this image)

### DeleteImage
- **Method**: POST
- **Body**: `{id: "<imageId>"}` (likely)

### ForceDelete
- **Method**: GET
- **Params**: `id=<imageId>`

### BatchDeleteImage
- **Method**: POST
- **Body**: `{ids: ["<id1>", "<id2>"]}` (likely)

### ImageExport
- **Method**: POST
- **Body**: Export spec

### CheckPath
- **Method**: POST
- **Body**: Path check spec

### GetPathFileName
- **Method**: POST
- **Body**: Path spec

### LoadUrl
- **Method**: POST
- **Body**: URL load spec

### LoadPath
- **Method**: POST
- **Body**: Path load spec

### LoadPaths
- **Method**: POST
- **Body**: Multiple paths load spec

### DeleteFailInfo
- **Method**: POST
- **Body**: Fail info spec

### GetRecommendedImageAccelerators
- **Method**: GET

## Registry/Mirror Management (`docker/view/`)

### ShowMirrorList
- **Method**: GET
- **Response**: `{result: [Mirror]}`

```json
// Mirror
{
  "id": 0,
  "alias": "DockerHub",
  "address": "https://hub.docker.com/",
  "userName": "",
  "password": "",
  "isDockerhub": true,
  "status": true
}
```

### AddMirrorSource
- **Method**: POST
- **Body**: Mirror spec

### SwitchMirrorSource
- **Method**: GET
- **Params**: `id=<mirrorId>`

### DeleteMirror
- **Method**: GET
- **Params**: `id=<mirrorId>`

### CheckMirrorAliasOrAddr
- **Method**: GET
- **Params**: `alias=<alias>` or `addr=<addr>`

### GetRegistryMirrors
- **Method**: GET
- **Response**: `{result: [string]}` (mirror URLs)

### SetRegistryMirrors
- **Method**: POST
- **Body**: Mirror URLs

### GetHttpProxy
- **Method**: GET

### SetHttpProxy
- **Method**: POST
- **Body**: Proxy config

## Compose (`docker/compose/`)

### ShowOfflineContainers
- **Method**: GET
- **Params**: `projectName=<name>`

## Data Migration (`docker/migration/`)

### DataMigrate
- **Method**: POST
- **Body**: Migration spec

### GetProgress
- **Method**: POST

### GetMigrateStatus
- **Method**: GET

### GetInfo
- **Method**: GET

## User Preferences (`docker/user/`)

### GetPopup
- **Method**: GET

### SetPopup
- **Method**: GET

### GetUserSortConfig
- **Method**: GET
- **Params**: `source=<source>`

### SetUserSortConfig
- **Method**: POST
- **Body**: Sort config

### GetContainerImageUpdateConfig
- **Method**: GET

### SetContainerImageUpdateConfig
- **Method**: POST
- **Body**: Update config

## Logs (`docker/log/`)

### DeleteLogs
- **Method**: GET
- **Params**: `option=<option>`

## Non-Docker APIs Used by Docker App

| Endpoint | Method | Notes |
|----------|--------|-------|
| `user/current/user` | GET | Current user info |
| `filemgr/getHomeShare` | GET | Home share folders |
| `user/config` | GET | User config |
| `storage/volume/list` | GET | Storage volumes |
| `desktop/create` | POST | Create desktop shortcut |
