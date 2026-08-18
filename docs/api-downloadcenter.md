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

## Adding a task

### download/add
- **Method**: POST, **multipart/form-data** — not JSON, which is what made
  earlier attempts fail with a misleading `1302, Path does not exist`
- **Fields**: `is_batch` ("false"), `save_dir`, `download_url`
- On the UI's no-encryption whitelist, so it takes plain requests

### download/checkLinks
- **Method**: POST, multipart
- **Field**: `download_url`
- **Response**: `{status: 0}` when the link is usable

### download/deleteTask
- **Method**: **DELETE**, encrypted
- **Params**: `ids`, `delete_file`, `is_download`

`ids` is the **numeric `id`** from the listing. Passing the `task_id` string
answers `9999`.

## Task fields

A running task and a finished one report different keys, so everything
defaults:

- Running: `downloaded_size`, `download_speed`, `task_status`, `plan`
  (percent), `remaining_time`, `error_code`, `created_at`, `ext`, `index`
- Finished: `download_completed_time`, `uploaded_size`, `seeding_status`,
  `is_zip_mode`
- Both: `id`, `task_id`, `download_file_name`, `download_url`, `save_dir`,
  `total_size`, `uname`, `link_type`

`error_code` is non-zero on failure — 9 for a URL the NAS cannot reach, in
which case `total_size` stays 0.

Verified end to end on 2026-08-18: queueing a URL, watching it finish,
listing running and finished tasks, and removing both.
