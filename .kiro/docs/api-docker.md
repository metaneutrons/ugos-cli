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
- **Body**: Container spec object — live-captured 2026-08-06 via a browser-side
  `XMLHttpRequest.prototype.send` patch injected into the Docker app's iframe
  (`window.fetch` alone did **not** catch it — the app makes its API calls
  from inside an iframe with its own JS realm, `<iframe name="docker">`, not
  the top-level `window`). Real request body for `nginx:latest` with one port
  mapping:
  ```json
  {
    "imageName": "nginx:latest",
    "containerName": "nginx-2",
    "cpuLimit": 0,
    "memLimit": 0,
    "abnormalReset": false,
    "hardwareAcceleration": false,
    "gpuIds": [],
    "privilegedMode": false,
    "networkMode": "bridge",
    "subnetSettings": [{"networkName": "bridge", "subnet": "172.17.0.0/16"}],
    "volumes": [],
    "environmentVariables": [{"variable": "PATH", "price": "..."}],
    "portMapping": [{"nasPort": 34817, "containerPort": 80, "portType": "tcp"}],
    "containerRunCommand": ["nginx", "-g", "daemon off;"],
    "permAndFunc": null,
    "runContainer": true,
    "imageId": "..."
  }
  ```
  Notable quirks confirmed against the live API:
  - `environmentVariables` items use `{variable, price}` — `price` is the
    value, not a typo worth "fixing" on the wire.
  - `cpuLimit`/`memLimit` of `0` means unlimited, not "field absent".
  - `containerRunCommand` is a string array (Docker `Cmd` style), not a
    single command string.
  - `subnetSettings` is sent even for the default `bridge` network — not
    tested whether omitting it is accepted.
  - `permAndFunc` was `null` here, not `[]` — treat as optional/nullable
    when deserializing.
  - `portMapping` keys are `nasPort`/`containerPort`/`portType` — **not**
    `hostPort`/`protocol`. The CLI's `build_container_spec` originally used
    the wrong names here (fixed 2026-08-06); a container created via the CLI
    before that fix would have had its `--port` mappings silently dropped or
    defaulted by the backend.

### UpdateContainer
- **Method**: POST
- **Body**: Container spec object (same shape as `CreateContainer`, unverified whether all fields are required for update-only calls)

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

Full lifecycle live-captured 2026-08-07 via the same iframe-XHR-patch technique
used for `CreateContainer` (see above) — created, listed, inspected, stopped,
and removed a real two-service project (`nginx`+`redis`) on picard.

### GetUserId
- **Method**: GET
- **Response**: `{data: <userId>}`
- Called by the "New Project" dialog before it lets you type a name — not
  observed to gate anything else; likely just used to scope the default
  storage path.

### CheckProjectName
- **Method**: GET
- **Params**: `name=<name>`
- **Response**: `{result: true}` (true = name is available)

### GetDockerSharedFolder
- **Method**: GET
- **Response**: `{data: "/volume1/docker"}` (or similar) — the shared-folder
  root the UI concatenates the project name onto to build `projectPath`
  (`Freigegebener Ordner/docker/<name>` in the UI ↔
  `/volume1/docker/<name>` on disk).

### HasYAMLFile
- **Method**: GET
- **Params**: `path=<projectPath>`
- **Response**: `{result: false}` — checks whether a `docker-compose.yml`
  already exists at the target path (e.g. reusing a folder from an earlier
  project) before letting `CreateProject` overwrite it.

### CreateProject
- **Method**: POST
- **Body**:
  ```json
  {
    "projectName": "re-test",
    "projectPath": "/volume1/docker/re-test",
    "projectContent": "services: {web: {image: \"nginx:latest\", ports: [\"8099:80\"]}, cache: {image: \"redis:alpine\"}}",
    "runProject": true,
    "cols": 60,
    "latestImages": false
  }
  ```
- **Response**: `{result: true}`
- Notable quirks:
  - `projectContent` is the raw `docker-compose.yml` text, sent verbatim as a
    string (not parsed/re-serialized client-side) — any valid Compose YAML
    works, block-style or flow-style.
  - `cols` (terminal width, `60` observed) is passed through to whatever PTY
    streams the `docker compose up` log — cosmetic, safe to hardcode.
  - `latestImages: false` means "use the tags as written in the compose
    file"; `true` presumably forces a pull/re-tag to `:latest` — not tested.
  - `runProject: false` creates the project without starting it (untested via
    live capture, but consistent with the UI's "sofort ausführen" toggle).

### GetProjectListV3
- **Method**: POST
- **Body**:
  ```json
  {
    "projectName": "",
    "projectFilter": {
      "projectStatusFilter": [],
      "projectTypeFilter": [],
      "projectHasUpFilter": []
    },
    "projectSort": { "projectSortEnum": 1, "projectSortOrder": 0 }
  }
  ```
- **Response**: `{data: {list: [Project], originalTotal: N}}`
  ```json
  {
    "name": "re-test",
    "path": "/volume1/docker/re-test",
    "status": 1,
    "containerSum": 2,
    "runContainerSum": 2,
    "configFileMissing": false,
    "createTime": "2026-08-07 ...",
    "containerList": [
      {
        "containerName": "re-test-web-1",
        "containerId": "...",
        "imageName": "nginx",
        "version": "latest",
        "restartPolicy": "no"
      },
      {
        "containerName": "re-test-cache-1",
        "containerId": "...",
        "imageName": "redis",
        "version": "alpine",
        "restartPolicy": "no"
      }
    ],
    "quickAccess": null,
    "application": "",
    "containerNum": 2,
    "progress": 100,
    "imgHasUpdate": false
  }
  ```
  `projectFilter`/`projectSort` fields are all empty/default in the captured
  call (no filtering/sorting was exercised in the UI) — shapes are confirmed,
  actual filter *values* are not.

### GetProjectInfoV2
- **Method**: GET
- **Params**: `projectName=<name>`
- Returns the same per-project shape as one `GetProjectListV3` list entry,
  used by the project detail view (Container/Ressourcenüberwachung/Protokoll/
  Compose-Konfiguration tabs).

### StopProject
- **Method**: GET
- **Params**: `projectName=<name>`
- **Response**: `{data: {result: "successful"}}`
- Equivalent to `docker compose stop` — containers are stopped, not removed;
  `DownProject` is still needed to actually delete them.

### DownProject
- **Method**: POST
- **Body**: `{"projectName": "re-test", "delImages": false}`
- **Response**: `{data: {result: true}}`
- Equivalent to `docker compose down`; `delImages: true` presumably also
  removes the images pulled for the project (untested — only `false` was
  captured).

### Start/Restart (not captured)
The project action menu also has "Starten" and "Neu starten" entries; these
were not exercised during this RE session. By analogy with `StopProject` and
the container-level `StartContainer`/`RestartContainer` pattern, they are
almost certainly:
- `GET docker/compose/StartProject?projectName=<name>`
- `GET docker/compose/RestartProject?projectName=<name>`
but this is an **unverified guess**, not a live capture — confirm before
relying on it.

### ContainerListV2 / ShowContainerDetailListV2 project filter
Both container-listing endpoints accept a `projectName` field in their POST
body to scope results to one compose project (used by the project detail
view's "Container" tab) — same request shape as the standalone calls
documented above, just with `projectName` set instead of `""`.

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
