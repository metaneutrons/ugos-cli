# Backup

UGOS has **no data backup app** in the control panel bundles: probing `backup`,
`backupmgr` and `backupcenter` under `http://<nas>:9999/` returns 404, and the
neighbouring `snapshot` app (`com.ugreen.snapshot`) manages filesystem
snapshots, not backups.

An earlier version of this note said the three `backup_restore/*` paths had no
caller in the web UI and that nothing could be said about them. That was wrong.
They are called from the control panel page **Update & Restore**, and they back
up and restore the **configuration of the NAS**, not its data. They are
described in [api-update.md](api-update.md) and wired as `ugos-cli backup`.

What `ugos-cli` still does not cover is a backup of files. For that the
nearest tools are snapshots (`snapshot`) and `rsync` against the NAS (`rsync`
service, see [api-fileservices.md](api-fileservices.md)).
