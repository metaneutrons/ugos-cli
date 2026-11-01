# ugos-mcp tools

Generated from the build by a test in ugos-mcp. Started with `ugos-mcp`.

## ugos_docker_batch

Batch operate on Docker containers (start/stop/restart/remove)

- `action` (required): Operation: start, stop, restart, remove.
- `ids` (required): Container IDs.
- `target`: Target NAS.

## ugos_docker_clone

Clone a Docker container

- `id` (required): Source container ID.
- `name` (required): New container name.
- `target`: Target NAS.

## ugos_docker_compose

List containers in a Docker Compose project

- `project` (required): Compose project name.
- `target`: Target NAS.

## ugos_docker_create

Create a Docker container from a spec (JSON object matching ContainerDetail schema). Use ugos_docker_show to get an example spec.

- `spec` (required): Full VM specification as a JSON object.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_docker_image_export

Export a Docker image to a NAS path

- `id` (required): Image ID.
- `path` (required): Destination path on the NAS.
- `target`: Target NAS.

## ugos_docker_image_load_path

Load a Docker image from a NAS path

- `source` (required): URL or NAS path to load image from.
- `target`: Target NAS.

## ugos_docker_image_load_url

Load a Docker image from a URL

- `source` (required): URL or NAS path to load image from.
- `target`: Target NAS.

## ugos_docker_images

List local Docker images

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_docker_logs

Get Docker container logs

- `id` (required): Container ID.
- `lines`: Number of log lines (default: 100).
- `target`: Target NAS.

## ugos_docker_mirror_add

Add a Docker registry mirror source

- `address` (required): Mirror URL.
- `alias` (required): Display name.
- `target`: Target NAS.

## ugos_docker_mirror_delete

Delete a Docker registry mirror source

- `id` (required): Mirror ID.
- `target`: Target NAS.

## ugos_docker_mirror_switch

Switch active Docker registry mirror source

- `id` (required): Mirror ID.
- `target`: Target NAS.

## ugos_docker_mirrors

List Docker registry mirror sources

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_docker_overview

Get Docker engine overview (container/image counts, CPU/memory usage)

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_docker_proxy_get

Get Docker HTTP proxy configuration

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_docker_proxy_set

Set Docker HTTP proxy configuration

- `proxy` (required): Proxy configuration JSON object.
- `target`: Target NAS.

## ugos_docker_ps

List Docker containers

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_docker_pull

Pull a Docker image from a registry

- `image` (required): Image name (e.g. "nginx").
- `tag`: Image tag (default: "latest").
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_docker_restart

Restart a Docker container

- `id` (required): Container ID.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_docker_rm

Remove a Docker container

- `id` (required): Container ID.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_docker_search

Search Docker Hub for images

- `name` (required): Image name to search for.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_docker_show

Show detailed Docker container configuration

- `id` (required): Container ID.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_docker_start

Start a Docker container

- `id` (required): Container ID.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_docker_status

Get Docker engine status (online/offline)

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_docker_stop

Stop a Docker container

- `id` (required): Container ID.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_host_info

Show NAS host hardware info (CPU cores, memory)

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_image_delete

Delete an ISO/disk image

- `file_name` (required): Image file name (e.g. "CachyOS.iso").
- `image_name` (required): Image display name (e.g. `CachyOS`).
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_image_list

List ISO/disk images available for KVM

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_image_usage

Check which VMs use an image

- `name` (required): Image name.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_log_operators

List all operator usernames from KVM logs

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_log_search

Search KVM audit logs

- `page`: Page number (default: 1).
- `page_size`: Page size (default: 20).
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_network_create

Create a KVM network

- `interface` (required): Mapping network interface.
- `name` (required): Network name.
- `net_type`: Network type: bridge, nat, none.
- `target`: Target NAS.

## ugos_network_delete

Delete a KVM network

- `name` (required): Network name.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_network_list

List KVM networks

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_network_show

Show KVM network details

- `name` (required): Network name.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_network_update

Update a KVM network

- `spec` (required): Full VM specification as a JSON object.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_ova_export

Export a VM as an OVA file to a path on the NAS

- `ova_path` (required): Output OVA file path on the NAS.
- `storage_name` (required): Storage volume name.
- `storage_uuid` (required): Storage volume UUID.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `vm` (required): VM display name or UUID.

## ugos_ova_parse

Parse an OVA file on the NAS and return the VM configuration it contains

- `ova_path` (required): OVA file path on the NAS.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_project_create

Create a Docker Compose project from raw docker-compose.yml content. Use ugos_project_list first to check the shared-folder root for the storage path.

- `content` (required): Raw `docker-compose.yml` content.
- `name` (required): Compose project name.
- `path` (required): NAS storage path for the project (e.g. "/volume1/docker/<name>").
- `run`: Start the project immediately after creation.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_project_list

List Docker Compose projects

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_project_remove

Remove a Docker Compose project (docker compose down)

- `del_images`: Also remove images pulled for the project.
- `name` (required): Compose project name.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_project_restart

Restart a Docker Compose project

- `name` (required): Compose project name.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_project_show

Show details of a Docker Compose project

- `name` (required): Compose project name.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_project_start

Start a Docker Compose project

- `name` (required): Compose project name.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_project_stop

Stop a Docker Compose project (containers remain, not removed)

- `name` (required): Compose project name.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_snapshot_create

Create a snapshot of a virtual machine

- `name` (required): Snapshot name.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `vm` (required): VM display name or UUID.

## ugos_snapshot_delete

Delete a snapshot

- `name` (required): Snapshot name.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `vm` (required): VM display name or UUID.

## ugos_snapshot_describe

Set a snapshot's description

- `description` (required): Description text.
- `name` (required): Snapshot name.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `vm` (required): VM display name or UUID.

## ugos_snapshot_list

List snapshots for a virtual machine

- `target`: Target NAS name or host. Required when multiple targets are configured.
- `vm` (required): VM display name or UUID.

## ugos_snapshot_revert

Revert a virtual machine to a snapshot

- `name` (required): Snapshot name.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `vm` (required): VM display name or UUID.

## ugos_storage_add

Add a storage volume to KVM

- `name` (required): Storage volume name.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `uuid` (required): Storage volume UUID.

## ugos_storage_delete

Remove a storage volume from KVM

- `name` (required): Storage volume name.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `uuid` (required): Storage volume UUID.

## ugos_storage_list

List KVM storage volumes

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_storage_usage

Check which VMs use a storage volume

- `name` (required): Storage volume name.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `uuid` (required): Storage volume UUID.

## ugos_target_list

List available UGOS NAS targets. Call this first to discover target names for use in other tools.


## ugos_usb_list

List USB devices available to a VM

- `target`: Target NAS name or host. Required when multiple targets are configured.
- `vm` (required): VM display name or UUID.

## ugos_vm_create

Create a new VM from a full VM spec (JSON object matching VmDetail schema). Use ugos_vm_show to see an example spec.

- `spec` (required): Full VM specification as a JSON object.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_vm_delete

Delete a virtual machine

- `name` (required): VM display name or UUID.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_vm_list

List all virtual machines on a UGOS NAS

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_vm_reboot

Reboot a virtual machine (graceful or forced)

- `force`: Force reboot (default: false).
- `name` (required): VM display name or UUID.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_vm_show

Show detailed VM configuration

- `name` (required): VM display name or UUID.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_vm_start

Power on a virtual machine

- `name` (required): VM display name or UUID.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_vm_stop

Shut down a virtual machine (graceful or forced)

- `force`: Force shutdown (default: false).
- `name` (required): VM display name or UUID.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_vm_update

Update an existing VM (must be shut off). Takes a full VM spec (JSON object matching VmDetail schema).

- `spec` (required): Full VM specification as a JSON object.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_vnc_generate

Generate a noVNC link to access a VM console in the browser

- `source_url`: Base URL for the noVNC link.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `vm` (required): VM display name or UUID.

## ugos_vnc_list

List VNC links for a VM

- `target`: Target NAS name or host. Required when multiple targets are configured.
- `vm` (required): VM display name or UUID.

