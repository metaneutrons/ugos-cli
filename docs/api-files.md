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

## Upload, not yet wrapped

Two steps, both v1 and plain:

1. `filemgr/fileUpload`, multipart, announces the transfer: `uuid`, `dir`,
   `action_type`, `size`, `begin_size`, `current_size`, `change_time`,
   `filename`, `resume`, `first_request`.
2. `filemgr/fileUploadV2`, the raw bytes as the body, with
   `Content-Disposition: attachment; filename="..."` and
   `Content-Type: application/octet-stream`. The response names the path the
   file landed at.

`getUpdateTmpInfo` sits between them and appears to serve resumption.
