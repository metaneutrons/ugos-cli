# Download Center API (`com.ugreen.downloadmgr`)

Base path: `/ugreen/v1/downloadCenter/`

Unlike the file manager, this app answers **plaintext** requests — no
`encrypt_query` needed. 30 endpoints exist; the read side is verified against
a live NAS (2026-08-18), the write side is not yet usable.

## Verified

### download/getPath
- **Method**: GET
- **Response**: `{path, path_display, path_is_validity, available_size}`

```json
{"path": "/volume1/download", "path_display": "Shared Folder/download",
 "path_is_validity": true, "available_size": 15396562034688}
```

### download/globalSpeed
- **Method**: GET
- **Response**: `{download_speed, upload_speed, downloading_num, completed_num}`

### download/getListV3 and complete/getListV2
- **Method**: GET
- **Params**: `page`, `limit`
- **Response**: `{result: [...] | null, total: N}`

`result` is `null` rather than `[]` when empty. The entry shape could not be
captured because the NAS had no tasks — see below.

### setup/getGeneralSettings, setup/isInit
- **Method**: GET
- Settings include `default_save_path`, `auto_listen_path`,
  `max_concurrent_task`.

## Blocked: adding a task

`download/addV2` (POST, JSON) accepts `download_url`, `save_dir` and
`task_name` — with those names it answers `1302, Path does not exist`,
whereas any other field naming answers `9999`, so the names are right. But
`1302` persists for every path tried, including `/volume1/download` (which
`getPath` reports as valid) and `/volume1`, and with no path at all. The app
reports `setup/isInit: true`, so it is configured.

Some other required field is missing, and its absence is reported as a path
error. `download/add` (the older sibling) takes **FormData**, not JSON, and
sets `is_batch`. `download/deleteTask` uses HTTP **DELETE** with query
parameters, a method this client does not implement yet.

Resolving this needs a capture of the web UI adding a download.

## Task fields, from the UI's response mapper

Not verified against live data, listed for whoever picks this up:
`id`, `task_name`, `save_dir`, `download_url`, `download_start_time`,
`download_completed_time`, `uploaded_size`, `upload_speed`, `share_ratio`,
`uname`, `path_is_exist`. A second mapper exposes them as `fileName`,
`status`, `url`, `savePath`, `fileSize`, `downloadSize`, `speed`, `overTime`.
