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
- **Body**: `{pageNum, pageSize}` (same page as `ContainerListV2`)
- **Response**: `{result: [...]}`, `null` when empty. Per container: `containerId`,
  `cpuUsage` (percent), `memUsage` and `totalMemory` (bytes), `downstreamTraffic` and
  `upstreamTraffic` (bytes), `status`, `restarting`, `imgHasUpdate`, `imageNotExist`,
  `nasIPs` (`null` or list), `portMapping` (`null` or `[{containerPort, nasPort, portType}]`),
  `volumes`, `progress`.
- `ContainerListV2` carries no CPU and memory: `docker ps` joins the two by `containerId`.
  A stopped container has `cpuUsage` 0, which is not a measurement, so the CLI shows `-`.

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
- **Body**: `{containerId}`. A `tail` field is ignored: asking for 1, 50 or 500 lines
  gave the same 153 lines (verified live). `ugos-cli` cuts the last lines itself.
- **Response**: `{result: {message, stream, time, upperLimit}}`. `message` is the whole log
  as one string, `upperLimit` is true when UGOS stopped at its limit. `stream` and `time`
  came back empty.
- The web UI does not use this for its log view. It opens a WebSocket instead, see
  `ShowContainerLogsFollow` below.

## WebSockets (logs and terminal)

The web UI reads container and project logs and runs the terminal over WebSockets. They
sit beside the HTTP API: `wss://<nas>:<port>/ugreen/v1/docker/...`, with the session
token in the query string as `ugk=<token>` (a browser cannot set headers on a WebSocket).
Verified live: **the ordinary session token works as `ugk`**, and the connection goes
through the same pinned certificate check as every other request. The address is a
credential, so errors never print it.

Frames are JSON text: `{"type": ..., "data": {"code", "msg", "text", "columns", "rows"},
"role": {...}}`. `data.code` `200` carries `text`, `2001` is a ping from the NAS that must
be answered, `2002` is ignored.

### ShowContainerLogsFollow
- **URL**: `docker/container/ShowContainerLogsFollow?containerId=<full id>&rawData=true`
- The NAS sends the backlog at once (205 frames in 0.24 s in the capture), then every line
  the container writes. `type` is `docker-log`. There is no parameter for a number of
  lines. Frames can end in the middle of a line.
- The ping is answered with `{"type": "docker-log", "data": {"code": 2001, "msg": "",
  "text": ""}}`.

### GetProjectLogs
- **URL**: `docker/compose/GetProjectLogs?projectName=<name>`
- The NAS sends nothing until the client has sent
  `{"type": "docker-project-log", "data": {"code": 200, "msg": "", "text": "", "columns":
  80, "rows": 24}}`. Then `type` is `compose-log`, one frame per burst of lines from any
  service, each line prefixed `\u001b[36mservice-1  | \u001b[0m` (a colour per service).
  The ping is answered with `{"type": "compose-log", "data": {"code": 2001, "msg": "pong",
  "text": ""}}`.
- `ugos-cli` takes the backlog as everything up to the first pause of 0.7 s, cuts the last
  lines itself, and takes out the colour codes and `\r`.

### Terminal (`AddTerminal`, `Ws`, `DelTerminal`)
Read from the app's own JavaScript (`/docker/assets/index-*.js` and the container chunk it
loads, served by the NAS), and verified live on a CI runner container:
- `POST docker/container/AddTerminal` with `{id: <full container id>, exec: "/bin/bash",
  pageID: "<number as text>"}` answers `{result: "<uuid>"}`. The body length matches the
  encrypted request in the capture (113 bytes). Plain requests are accepted. Code `2100`
  means the container has no such command (tried: `/bin/nope`). The web UI allows 10
  terminals per container.
- `GET docker/container/GetTerminalList?containerId=<full id>&processCheck=true` answers
  `{result: [{uuid, cmd, state, uid, pageID}]}`, `state` 1 while someone is connected.
  A terminal whose process ended is dropped from the list.
- `Ws` (WebSocket) `docker/container/Ws?containerId=&uuid=&pageID=`: `type:
  docker-terminal`. The client sends `{code: 200, text: "", columns, rows}` to resize and
  `{code: 200, text: "<keystrokes>"}` for input, a heartbeat `{code: 2223}` every 60 s,
  and answers the ping `{code: 2001}` with `{code: 2001, msg: "pong"}`. The NAS sends the
  output as `{code: 200, text}` (a `pty`: `\r\n`, colour and title codes, bracketed paste
  `\u001b[?2004h`). Codes `2047` and `2048` arrive on the app's own notification channel
  when a terminal is removed or created.
- **The connection stays open when the shell exits.** The web UI polls
  `GetTerminalList` every 2 s and drops terminals that are gone; `docker shell` does the
  same.
- `POST docker/container/DelTerminal` with `{containerId, uuid}` answers `{}`.
- `POST docker/container/ChangeTerminalClientID` with `{uuid, containerId, pageID}` takes
  over a terminal another page is connected to. Not used.
- `GET docker/container/RecommendedTerminalCommands` answers `{result: ["/bin/bash"]}`.

`docker exec` runs a command through this: it opens a terminal, turns the echo off with
`stty -echo`, types `<command>; echo "<marker>$?"`, reads until the marker (which carries
the exit status), and removes the terminal. Output and error output arrive merged, as a
pty gives them.

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
- **Response**: `{result: <n>}`, containers with a newer image. `GetUpdateProjectCount` in
  `docker/compose/` is the same for compose projects. Both gave 0 on picard when last read
  (5 and 3 an hour earlier, before projects were redeployed), so the positive case is only
  as verified as the earlier read.

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
- **Body**: `{alias, address}`, plus `userName` and `password` for a registry
  that needs a login. The web UI's "add image source" form posts exactly these
  four fields (and `id` when it edits a source). `ugos-cli docker mirror-add`
  sends the login fields only when `--registry-user` is given.
- Whether UGOS applies the stored login when `DownloadImage` pulls from that
  registry is not verified on hardware yet. `ShowMirrorList` returns the
  `password` field, so a token stored here is readable by any NAS admin session.

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
and removed a real two-service project (`nginx`+`redis`) on nas1.

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

### StartProject / RestartProject
- `GET docker/compose/StartProject?projectName=<name>`
- `GET docker/compose/RestartProject?projectName=<name>`

Both calls appear in the web UI bundle of the Docker app with a `projectName`
query parameter. A restart does not pick up a changed compose file or image;
use `UpdateProject` for that.

### UpdateProject
- **Method**: POST
- **Body**: `{"projectName": "<name>", "projectContent": "<compose yaml>",
  "runProject": true, "cols": 60, "latestImages": false}`
- This is the "recreate" action of the project detail view in the web UI
  (read from its bundle). `latestImages: true` is the "pull the images again"
  checkbox of its confirmation dialog.

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
- **Method**: GET (`docker/user/`)
- **Response**: `{isPlan, planSettings: {date: "06:00", day: [1], planType: 2}, updateReminder}`.
  `isPlan` is the scheduled check, `date` its time of day. What `day` and `planType` mean is
  not known beyond the values read, so `ugos-cli` passes them through untouched.

### SetContainerImageUpdateConfig
- **Method**: POST (`docker/user/`)
- **Body**: the same object. Verified live: the whole object has to be sent (read, change,
  send), the answer is `{}` and the change shows on the next read. The reminder and the
  time were changed and restored.

## Networks (`docker/network/`)

### ShowLocalList
- **Method**: GET
- **Response**: `{result: [{networkId, networkName, driver, ipv4Subnet, ipv4Gateway,
  ipv6Subnet, ipv6Gateway, ipv6Flag, ipv4Flag, networkCard, networkCardInvalid,
  disableIPMasquerade, create, containerNames}]}`. `--` stands for no address. `driver` is
  `bridge`, `host`, `null` or `ipvlan`. `containerNames` lists the attached containers.

### ShowLocalContainerAndNetwork
- **Method**: GET
- **Params**: `networkId=<full id>` (without it: error 9999)
- **Response**: `{result: [{containerId, containerName, flag, running}]}`. It lists every
  container on the NAS (97 on picard). `flag` is true for the ones attached to this
  network: for `host`, `traefik`, `ipvlan-lan` and `vellum_internal` the flagged names are
  exactly `containerNames` of `ShowLocalList`. `running` is `Running` or `exited`.
- The web UI uses this for its dialog to attach containers. Nothing here changes a network:
  no endpoint for creating, deleting or attaching was in the capture.

## Logs (`docker/log/`)

This is the log of the Docker app itself (who deployed, started or removed what), not
the output of a container.

### PageSearchLogs
- **Method**: POST
- **Body**: `{pageNum, pageSize, operator, level, content, createTimeSort}`. All filters
  verified live: `operator` is an exact user (`system` is the NAS), `level` is `0` all,
  `1` info, `2` failure (`3` matches nothing), `content` is a substring match,
  `createTimeSort` is `asc` or `desc`.
- A `startTime`/`endTime` in `YYYY-MM-DD HH:MM:SS` is accepted and ignored. Other formats
  error. The export has `fromTime`/`toTime` fields (printed as `all`), their request
  names are not known.
- **Response**: `{curPage, logs: [{content, createTime, level, operator}], total}`.
  `logs` is `null`, not `[]`, when nothing matches.

### GetAllOperator
- **Method**: GET
- **Response**: `{result: ["system", "fabian"]}`

### ExportLogs
- **Method**: GET (a file download, `ugk=<token>` in the browser)
- **Params**: `pageNum=1&pageSize=20&level=0&exportType=txt`. The page size does not
  limit the export: it holds the whole log.
- **Response**: `text/plain`. A header (`content: all`, `fromTime`, `toTime`, `level`,
  `operator`), a blank line, then `create_time<TAB>level<TAB>operator<TAB>content` with
  level as text (`info`).

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
