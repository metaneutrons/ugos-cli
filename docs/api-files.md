# File manager API (`com.ugreen.filemgr`)

Split across two API versions, which matters for the transport:

- **v1** (`filemgr/…`) answers plain requests: `getVolumes`, and the upload
  pair `fileUpload` / `fileUploadV2`.
- **v2** (`v2/filemgr/…`) answers **only encrypted** requests. Listing,
  creating, deleting and renaming all live here. See `api-encryption.md`.

Verified against a live NAS on 2026-08-18.

## Listing

### v2/filemgr/getDirFileListV2
- **Method**: POST, encrypted
- **Body**: `{path, page, limit, is_shield_recycle}`
- **Response**: `{left_tree: {...}, right_files: {...}, status}`

The two panes mirror the web UI: `left_tree` feeds the navigation sidebar,
`right_files` holds the directory itself. In both, `files` is **`null`**
rather than an empty list when there is nothing to show.

Entries carry 40-odd fields; the useful ones are `name`, `path`, `size`,
`file_type` (0 file, 1 directory), `ext`, `mtime`, `ctime`, `owner` and
`permission_mask`.

A path the user cannot reach answers `1301, Access not allowed` — `/` does,
`/home/<user>` and `/volume1/download` do not.

## Writing

### v2/filemgr/createFolder
- **Method**: POST, encrypted
- **Body**: `{path}` — the **full target path**

Passing a parent plus a name (`{path, name}`, `{path, folder_name}`) fails
with `1365, The file or directory does not exist`; `{dir, name}` fails with
`9999`.

### v2/filemgr/delPaths
- **Method**: POST, encrypted
- **Body**: `{paths: [...], forever: bool}`

Without `forever` the entries go to the recycle bin. Deleting a path that
does not exist still reports success.

### v2/filemgr/rename
- **Method**: POST, encrypted
- **Body**: `{path, new_name}` — `new_name` is a base name, not a path

## Volumes

### filemgr/getVolumes
- **Method**: GET, plain
- **Response**: `{result: [{name, path, fs_type, all, used, free}]}`

## Transfer

### filemgr/downloadFile
- **Method**: GET, plain, v1
- **Params**: `paths=<absolute path>`
- **Response**: the file itself

There is also a v2 route via `getDownloadToken` plus `v2/filemgr/downloadFile`,
which the web UI uses; the v1 one needs no token dance and is what this client
calls.

### Upload: two steps

**1. `filemgr/fileUpload`** — multipart, announces the transfer:
`uuid`, `dir`, `action_type` (0), `size`, `begin_size` (0), `current_size`
(0), `change_time` (mtime), `filename`, `resume` ("true"), `first_request`
("true").

**2. `filemgr/fileUploadV2`** — the raw bytes as the body, with:

- `Content-Disposition: attachment; filename="..."`
- `Content-Type: application/octet-stream`
- `X-Ugreen-Token` (RSA-sealed) and `X-Ugreen-Security-Key` (MD5), as for any
  encrypted endpoint — the body itself stays unencrypted
- **`ug-param`**, a JSON header repeating the metadata:
  `{uuid, file_name, action_type, size, current_size, resume, dir,
  change_time, is_live_photo, first_request, begin_size}`

The `uuid` must match step one. Inside `ug-param`, **`dir` is URL-encoded**,
unlike in step one. Without this header the call fails with a bare
`parameter error`.

The response names the path the file landed at. `getUpdateTmpInfo` sits
between the two steps in the web UI and appears to serve resumption; it is
not needed for a fresh upload.

Verified with an 8 MiB round trip: uploaded, downloaded again, byte-identical.
