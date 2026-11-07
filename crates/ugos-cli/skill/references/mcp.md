# ugos-mcp tools

Generated from the build by a test in ugos-mcp. Started with `ugos-mcp`.

## ugos_backup_info

Show the latest cloud backup and whether it is scheduled

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_backup_progress

Progress of a backup or a restore (restore: true)

- `restore`: A restore, not a backup.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_backup_reset_factory

Reset the NAS to factory state: data and configuration are deleted for good. Refused unless force is given: ask the user first, and again

- `force`: Go on although this restarts the NAS. Ask the user first.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_backup_reset_network

Reset the network settings. The NAS may get another address and this session ends. Refused unless force is given: ask the user first

- `force`: Go on although this restarts the NAS. Ask the user first.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_backup_restore

Restore services from a backup file. Overwrites their configuration and restarts them. Refused unless force is given: ask the user first

- `file` (required): Absolute path of `backup_restore.ugb`.
- `force`: Go on although the restore overwrites the configuration and restarts
those services. Ask the user first.
- `services` (required): Services as `category/item` or the item, or `all`.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_backup_save

Save the NAS configuration to a .ugb file on this machine, named like backup_restore.ugb

- `file` (required): Absolute path, named like `backup_restore.ugb`.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_backup_services

List the services a restore can bring back

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_disk_history

Show the history of a disk's SMART tests

- `name` (required): What to look at: a name, label or id, as the matching list tool shows it.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_disk_list

List the disks with model, size, temperature, hours on and the pool they serve

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_disk_plans

List the scheduled disk test plans

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_disk_show

Show one disk with serial, firmware and interface. Name it by short name, device path, slot or label

- `name` (required): What to look at: a name, label or id, as the matching list tool shows it.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_disk_smart

Show the SMART report of a disk: wear, spare, errors, unsafe shutdowns and the test state

- `name` (required): What to look at: a name, label or id, as the matching list tool shows it.
- `target`: Target NAS name or host. Required when multiple targets are configured.

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

## ugos_fileservice_set

Change the settings of a file service that runs. Refused if it is off: start it with the settings instead

- `force`: Go on although the web UI asks a confirmation: plain FTP, SMB1,
`NTLMv1`, NFS, discovery services while SMB is off. Ask the user first.
- `password`: The rsync password. Only for a new rsync account.
- `service` (required): `smb`, `ftp`, `nfs`, `rsync` or `webdav`.
- `settings`: Settings by alias, like `{"port": 2121, "log": "on"}`. The settings
tool lists the aliases and what they take.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_fileservice_settings

List the settings of a file service by alias, what they take and mean

- `service` (required): `smb`, `ftp`, `nfs`, `rsync` or `webdav`.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_fileservice_start

Switch a file service on, with settings. Plain FTP, SMB1, NTLMv1 and NFS are refused unless force is given: ask the user first. rsync needs an account, and a password for a new one

- `force`: Go on although the web UI asks a confirmation: plain FTP, SMB1,
`NTLMv1`, NFS, discovery services while SMB is off. Ask the user first.
- `password`: The rsync password. Only for a new rsync account.
- `service` (required): `smb`, `ftp`, `nfs`, `rsync` or `webdav`.
- `settings`: Settings by alias, like `{"port": 2121, "log": "on"}`. The settings
tool lists the aliases and what they take.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_fileservice_status

Show a file service (smb, ftp, nfs, rsync or webdav): whether it runs and its settings. For smb, also the discovery services

- `service` (required): `smb`, `ftp`, `nfs`, `rsync` or `webdav`.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_fileservice_stop

Switch a file service off. SMB is refused while Bonjour or WS-Discovery announce it unless force is given: ask the user first

- `force`: Go on although discovery services announce the SMB shares. Ask the
user first.
- `service` (required): `smb`, `ftp`, `nfs`, `rsync` or `webdav`.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_group_list

List the groups an administrator made, with their members. The built-in groups are not listed

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_host_info

Show NAS host hardware info (CPU cores, memory)

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_buzzer

Show the buzzer settings: which events sound it

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_buzzer_alarms

List the events the buzzer can sound for

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_buzzer_set

Change which events sound the buzzer

- `cache_exception`: A cache disk fails.
- `fan_fault`: A fan fails.
- `shutdown`: The NAS shuts down.
- `startup`: The NAS starts (not on every model).
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `too_hot`: A disk is too hot.

## ugos_hw_cpu

Show the CPU performance mode

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_cpu_set

Set the CPU performance mode: high, balanced or save

- `mode` (required): The mode: for the CPU `high`, `balanced` or `save`; for the fan
`quiet`, `auto` or `full`.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_fan

Show the fan mode and the fan curve

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_fan_curve

Set a custom fan curve of 17 duties per fan (20 to 100 percent at 0 to 80 degrees). Only on models that support it

- `cpu` (required): 17 duties for the CPU fan. From 55 °C the duty is 100 whatever is given.
- `sys` (required): 17 duties (20 to 100 %) for the device fan at 0, 5, ... 80 °C.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_fan_set

Set the fan mode: quiet, auto or full

- `mode` (required): The mode: for the CPU `high`, `balanced` or `save`; for the fan
`quiet`, `auto` or `full`.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_gpu

List the GPUs of the NAS and their mode

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_gpu_mode

Set a GPU to NAS use or VFIO passthrough. The NAS restarts, so this is refused unless force is given: ask the user first

- `accept_disclaimer`: Accept the passthrough disclaimer, as the web UI asks the user to.
- `force`: Go on although the NAS restarts. Ask the user first.
- `gpu` (required): The GPU id or name.
- `mode` (required): `nas` or `vfio` (passthrough).
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_led

Show the LED settings and brightness schedule

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_led_brightness

Set the LED brightness on weekdays

- `days` (required): Weekdays, `1` Monday to `7` Sunday.
- `percent` (required): Brightness in percent.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_led_off

Switch the LEDs off

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_led_on

Switch the LEDs on

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_power

Show the power settings: restart after power loss, wake on LAN, disk sleep and the schedule

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_power_set

Change the power settings

- `auto_on`: Power on again after a power loss.
- `schedule`: The power schedule on or off.
- `sleep`: When disks sleep: `never`, `10m`, `20m`, `30m`, `1h`, `2h` or `3h`.
- `sleep_log`: Log the sleeping.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `usb_sleep`: When USB disks sleep, like `sleep`.
- `wake`: What wakes a sleeping disk: `login` or `rw`.
- `wol`: Wake on LAN.

## ugos_hw_schedule_add

Add an entry to the power schedule (power on or off at a time on weekdays)

- `action` (required): `on` powers the NAS on, `off` powers it off.
- `days` (required): Weekdays, `1` Monday to `7` Sunday, comma separated, or `daily`,
`weekdays`, `weekend`.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `time` (required): Time as `HH:MM`.

## ugos_hw_schedule_remove

Remove the power schedule entry at a position

- `position` (required): Position, counting from 1.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_screen

Show the settings of the small display

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_screen_set

Change brightness, timeout, wallpaper or language of the small display

- `brightness`: Brightness, 25 to 100.
- `language`: A language key from the language list.
- `off_after`: Seconds until it goes dark: 15, 30, 60, 120, 300, or 0 for never.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `wallpaper`: A wallpaper path from the wallpaper list.

## ugos_hw_screen_wallpapers

List the wallpapers of the small display

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_ups

Show the UPS settings and state

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_ups_check

Check that a NUT server answers at an address

- `name` (required): What to look at: a name, label or id, as the matching list tool shows it.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_ups_disconnect

Disconnect the UPS

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_ups_firmware

Show the firmware of the USB UPS

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_ups_mibs

List the SNMP MIBs for a UPS

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_ups_set

Connect a UPS (USB, SNMP or network) and set what to do on battery. Stops and restarts the UPS service

- `action`: What to do on battery: `standby` or `shutdown` (default).
- `community`: SNMP community.
- `mib`: SNMP MIB (default `auto`).
- `network`: A UPS that another NAS serves, at this address.
- `nut_server`: Share the USB UPS with other NAS (NUT server).
- `snmp`: A UPS read by SNMP at this address.
- `snmp_version`: SNMP version, 1 or 2 (default 2).
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `usb`: A UPS on a USB port.
- `wait`: When: `battery-low` (default), `now`, or a time like `5m` or `2h`.

## ugos_hw_zram

Show whether memory compression (zram) is on

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_zram_off

Switch memory compression off. The NAS restarts, so this is refused unless force is given: ask the user first

- `force`: Go on although this restarts the NAS. Ask the user first.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_hw_zram_on

Switch memory compression on

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

## ugos_index_pause

Pause the thumbnail service for 1, 2 or 5 hours

- `hours` (required): 1, 2 or 5 hours.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_index_ranges

List the folder ranges thumbnails are made for

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_index_regenerate

Make thumbnails again, for all ranges or the given range ids

- `all`: All ranges.
- `missing_only`: Only the thumbnails that are missing.
- `ranges`: Ids of ranges, from the ranges tool.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_index_resume

Resume the thumbnail service after a pause

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_index_status

Show the state of the thumbnail service

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_lang_list

List the languages

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_lang_set

Set the language (name is the language key, or auto)

- `name` (required): What to look at: a name, label or id, as the matching list tool shows it.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_lang_show

Show the language of the NAS

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_log_operators

List all operator usernames from KVM logs

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_log_search

Search KVM audit logs

- `page`: Page number (default: 1).
- `page_size`: Page size (default: 20).
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_bond_candidates

List the ports that could join a bond

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_bond_create

Create a bond (link aggregation) from two or more ports. Refused when a port carries this session or the ports are in different networks unless force is given

- `default_route`: Make the bond the default route.
- `dns`: DNS servers, up to two.
- `force`: Go on although a port carries this session's address or the ports are
in different networks.
- `gateway`: Gateway of the static address.
- `ip`: Static IPv4 address; DHCP when left out.
- `mode`: Mode: `balance-alb` (default), `active-backup`, `balance-rr`,
`balance-xor` or `802.3ad`.
- `netmask`: Netmask of the static address.
- `ports` (required): Two or more ports, e.g. `["eth0", "eth1"]`.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_bond_delete

Delete a bond. Refused when firewall or traffic rules name it or it carries this session unless force is given

- `force`: Go on although rules name it or it carries this session.
- `name` (required): Interface name or label, e.g. `bond0`.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_bridge_candidates

List the ports that could join a bridge

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_bridge_create

Create a bridge from two or more ports. Refused when a port carries this session unless force is given

- `force`: Go on although a port carries this session's address.
- `ports` (required): The ports. Wi-Fi cannot be bridged.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_bridge_create_virtual

Create a virtual bridge, which VMs attach to, from one or more ports

- `force`: Go on although a port carries this session's address.
- `ports` (required): The ports. Wi-Fi cannot be bridged.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_bridge_delete

Delete a bridge. Refused when rules name it or it carries this session unless force is given

- `force`: Go on although rules name it or it carries this session.
- `name` (required): Interface name or label, e.g. `bond0`.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_dns_set

Set the DNS servers (automatic, or one or two) and the general switches: several gateways, prefer IPv4, address clash check

- `auto`: Take the DNS servers from DHCP.
- `clash_check`: Check for an address clash when an address changes.
- `multiple_gateways`: Allow several default gateways.
- `prefer_ipv4`: Prefer IPv4 over IPv6.
- `servers`: One or two DNS servers to use instead.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_flow_add

Add a traffic control rule at the end of the list: all ports, custom ports, or built-in services, with a rate limit in KB/s

- `all_ports`: A rule for every port.
- `interface` (required): Interface name, or `all`.
- `max_rate` (required): Limit in KB/s.
- `min_rate`: Guaranteed rate in KB/s (default 0).
- `ports`: Ports: a comma list of up to 15, or a range `a-b`.
- `protocol`: `tcp`, `udp` or `all` (default `all`).
- `services`: Built-in services by section id or name, see the services tool.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `target_port`: `src` or `dest` port of a custom rule (default `dest`).

## ugos_net_flow_enable

Switch the traffic control rule at a position on or off (on: true or false)

- `on`: Switch the rule on (`true`) or off (`false`), for the enable tool.
- `position` (required): Position in the list, counting from 1.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_flow_list

List the traffic control rules in order of priority

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_flow_remove

Remove the traffic control rule at a position (counting from 1)

- `on`: Switch the rule on (`true`) or off (`false`), for the enable tool.
- `position` (required): Position in the list, counting from 1.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_flow_services

List the built-in services a traffic control rule can name

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_gateway_order

Put the default gateways in an order of preference, by interface

- `ifaces` (required): Interface names or labels, most preferred first.
- `ipv6`: Order the IPv6 gateways instead of the IPv4 ones.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_general

Show DNS, default gateways, the IPv6 interface and the proxy of the NAS

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_list

List the network interfaces of the NAS with link, address and speed

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_proxy_set

Set or switch off the HTTP proxy of the NAS. A password passes through the model; prefer ugos-cli for that

- `all`: One proxy for every protocol, `IP:PORT`. Leave out with `off`.
- `http`: HTTP proxy, `IP:PORT`.
- `https`: HTTPS proxy, `IP:PORT`.
- `local`: Use the proxy for local addresses too.
- `off`: Switch the proxy off.
- `password`: Password of the proxy. It passes through the model; prefer `ugos-cli`.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `user`: User name of the proxy.

## ugos_net_set

Change the setup of one network interface (IPv4, IPv6, MTU, bond mode). Refused for the interface this session talks through unless force is given: a changed address cuts it off

- `bond_mode`: Bond mode, for a bond.
- `default_route`: Make the interface the default route.
- `dns`: DNS servers of the interface; an empty list clears them.
- `force`: Go on although this is the interface this session talks through; a
changed address cuts it off.
- `gateway`: Gateway of the static address.
- `ip`: Static IPv4 address.
- `ipv4_protocol`: IPv4 protocol: `dhcp`, `static` or `ipv4ll` (Thunderbolt only).
- `ipv6`: IPv6: `auto`, `off` or `static`.
- `ipv6_address`: Static IPv6 address, without the prefix.
- `ipv6_dns`: IPv6 DNS servers.
- `ipv6_gateway`: IPv6 gateway.
- `ipv6_prefix`: Static IPv6 prefix length.
- `mtu`: MTU in bytes, or `auto`.
- `name` (required): Interface name or label, e.g. `eth0`, `bridge0`, `LAN1`.
- `netmask`: Netmask of the static address, e.g. `255.255.255.0`.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_show

Show one network interface: addresses, DNS, gateway, MTU, ports of a bridge or bond

- `name` (required): What to look at: a name, label or id, as the matching list tool shows it.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_wifi_connect

Connect to a Wi-Fi network (ssid). A saved one is switched to. A password passes through the model; prefer ugos-cli for that

- `auth`: Authentication of a hidden network: `wpa`, `wpa2`, `wpa-wpa2`, `wpa3`,
`wpa2-wpa3` or `none`.
- `auto_join`: Join the network by itself, for the edit tool.
- `iface` (required): The Wi-Fi interface, e.g. `wlan0`.
- `on`: Switch the radio on (`true`) or off (`false`), for the radio tool.
- `password`: Password of the network. It passes through the model; prefer `ugos-cli`.
- `rescan`: Scan again instead of using the last scan, for the list tool.
- `ssid`: The network name, where the tool needs one.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_wifi_edit

Change whether a saved Wi-Fi network joins by itself (ssid, auto_join)

- `auth`: Authentication of a hidden network: `wpa`, `wpa2`, `wpa-wpa2`, `wpa3`,
`wpa2-wpa3` or `none`.
- `auto_join`: Join the network by itself, for the edit tool.
- `iface` (required): The Wi-Fi interface, e.g. `wlan0`.
- `on`: Switch the radio on (`true`) or off (`false`), for the radio tool.
- `password`: Password of the network. It passes through the model; prefer `ugos-cli`.
- `rescan`: Scan again instead of using the last scan, for the list tool.
- `ssid`: The network name, where the tool needs one.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_wifi_forget

Forget a saved Wi-Fi network (ssid)

- `auth`: Authentication of a hidden network: `wpa`, `wpa2`, `wpa-wpa2`, `wpa3`,
`wpa2-wpa3` or `none`.
- `auto_join`: Join the network by itself, for the edit tool.
- `iface` (required): The Wi-Fi interface, e.g. `wlan0`.
- `on`: Switch the radio on (`true`) or off (`false`), for the radio tool.
- `password`: Password of the network. It passes through the model; prefer `ugos-cli`.
- `rescan`: Scan again instead of using the last scan, for the list tool.
- `ssid`: The network name, where the tool needs one.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_wifi_list

List the Wi-Fi networks in range and the saved ones

- `auth`: Authentication of a hidden network: `wpa`, `wpa2`, `wpa-wpa2`, `wpa3`,
`wpa2-wpa3` or `none`.
- `auto_join`: Join the network by itself, for the edit tool.
- `iface` (required): The Wi-Fi interface, e.g. `wlan0`.
- `on`: Switch the radio on (`true`) or off (`false`), for the radio tool.
- `password`: Password of the network. It passes through the model; prefer `ugos-cli`.
- `rescan`: Scan again instead of using the last scan, for the list tool.
- `ssid`: The network name, where the tool needs one.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_wifi_radio

Switch the Wi-Fi radio on or off (on: true or false)

- `auth`: Authentication of a hidden network: `wpa`, `wpa2`, `wpa-wpa2`, `wpa3`,
`wpa2-wpa3` or `none`.
- `auto_join`: Join the network by itself, for the edit tool.
- `iface` (required): The Wi-Fi interface, e.g. `wlan0`.
- `on`: Switch the radio on (`true`) or off (`false`), for the radio tool.
- `password`: Password of the network. It passes through the model; prefer `ugos-cli`.
- `rescan`: Scan again instead of using the last scan, for the list tool.
- `ssid`: The network name, where the tool needs one.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_net_wifi_status

Show whether the Wi-Fi radio is on and which network it is connected to

- `auth`: Authentication of a hidden network: `wpa`, `wpa2`, `wpa-wpa2`, `wpa3`,
`wpa2-wpa3` or `none`.
- `auto_join`: Join the network by itself, for the edit tool.
- `iface` (required): The Wi-Fi interface, e.g. `wlan0`.
- `on`: Switch the radio on (`true`) or off (`false`), for the radio tool.
- `password`: Password of the network. It passes through the model; prefer `ugos-cli`.
- `rescan`: Scan again instead of using the last scan, for the list tool.
- `ssid`: The network name, where the tool needs one.
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

## ugos_pool_list

List the storage pools (RAID sets) with level, capacity and disks

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_pool_show

Show one storage pool with its disks, scrub and trim

- `name` (required): What to look at: a name, label or id, as the matching list tool shows it.
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

## ugos_share_acl_set

Set access rules on a shared folder. The administrators group always keeps read and write; a rule asking for less is reported, not applied

- `rules` (required): Rules like `user:anna=rw` or `group:family=read`. Access is `none`,
`read`, `rw` or `inherit`. Names must be in the share's access list.
- `share` (required): Share name or numeric id.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_share_acl_show

Show who may access a shared folder: groups, users and their access

- `share` (required): Share name or numeric id.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_share_create

Create a shared folder. Only administrators can open it at first; set access with ugos_share_acl_set

- `data_check`: Keep data checksums.
- `hide_in_network`: Hide the share from network browsing.
- `name` (required): Share name. Not `home`, `homes` or `personal_folder`, no slash, at most 80 characters.
- `no_recycle`: Turn the recycle bin off.
- `quota`: Quota such as `500m`, `100g` or `2t`. No quota when left out.
- `recycle_for_everyone`: Let everyone empty the recycle bin, not only administrators.
- `show_unreadable`: Show the share to users who cannot open it (default: hidden from them).
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `volume`: Volume name or path, e.g. `/volume1`. Optional when the NAS has one volume.

## ugos_share_delete

Delete a shared folder and everything in it. Refused while the folder holds files unless force is given; ask the user first

- `force`: Delete although the share holds files, which go with it.
- `share` (required): Share name or numeric id.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_share_list

List the shared folders (SMB shares) of a UGOS NAS

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_share_show

Show the settings of a shared folder: volume, quota, visibility, encryption

- `share` (required): Share name or numeric id.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_share_update

Change the quota or the visibility of a shared folder. The result is read back; an error says what UGOS did not apply

- `hide_in_network`: Hide the share from network browsing (`true`) or show it (`false`).
- `hide_unreadable`: Hide the share from users who cannot open it (`true`) or show it (`false`).
- `no_quota`: Lift the quota.
- `quota`: New quota such as `500m`, `100g` or `2t`.
- `share` (required): Share name or numeric id.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_smb_cache_clear

Clear the SMB cache. The network service restarts, so this is refused unless force is given: ask the user first

- `force`: Go on although the network service restarts. Ask the user first.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_smb_discovery

Show or change the SMB discovery services (WS-Discovery, Bonjour, UPnP) and the Time Machine targets. Without arguments it only reads

- `bonjour`: Bonjour, for Macs.
- `force`: Go on although UGOS advises against it. Ask the user first.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `time_machine`: Shares that are Time Machine targets; an empty list switches it off.
- `upnp`: `UPnP` announcements.
- `wsdd2`: WS-Discovery, for Windows.

## ugos_smb_status

Show the SMB service settings and the discovery services

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

## ugos_system_about

Show name, model, serial, version, run time, hardware and warranty of the NAS

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_system_finder

Show the finder switches: improvement plan and findable from the web

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_system_finder_set

Change the finder switches

- `improvement`: Join the improvement plan.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `web_find`: Let the NAS be found from the web.

## ugos_system_hostname_set

Set the host name of the NAS (name is the new host name)

- `name` (required): What to look at: a name, label or id, as the matching list tool shows it.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_system_ports

List the services and applications and the ports they listen on

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_system_poweroff

Power the NAS off; only the power button starts it again. Refused unless force is given: ask the user first

- `force`: Go on although this restarts the NAS. Ask the user first.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_system_reboot

Restart the NAS. Refused unless force is given: ask the user first

- `force`: Go on although this restarts the NAS. Ask the user first.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_target_list

List available UGOS NAS targets. Call this first to discover target names for use in other tools.


## ugos_terminal_set

Change SSH and Telnet. Switching a service on is refused unless force is given: it opens a way in, so ask the user first

- `force`: Go on although this switches a service on. Ask the user first.
- `lan_only`: Only from the local network, for both.
- `sftp`: SFTP over SSH.
- `ssh`: SSH on or off.
- `ssh_duration`: How long SSH stays on: `1` 10 min, `2` 30 min, `3` 1 h, `4` 2 h, `5` 3 h,
`6` 6 h, `7` until switched off.
- `ssh_encryption`: SSH encryption: `1` low or `3` high.
- `ssh_port`: SSH port.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `telnet`: Telnet on or off. Telnet sends passwords in the clear.
- `telnet_port`: Telnet port.

## ugos_terminal_show

Show SSH and Telnet settings

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_time_server_add

Add a time server (name is the host)

- `name` (required): What to look at: a name, label or id, as the matching list tool shows it.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_time_server_remove

Remove a time server (name is the host)

- `name` (required): What to look at: a name, label or id, as the matching list tool shows it.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_time_servers

List the time servers

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_time_set

Set the time zone, a time server or the clock by hand. A change that may invalidate the cloud certificate needs force

- `force`: Go on although the cloud certificate may not be valid then.
- `manual_utc`: Set the clock by hand, as `YYYY-MM-DD HH:MM:SS` in UTC.
- `server`: Keep the clock from this time server.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `timezone`: A time zone from the zone list, e.g. `Europe/Berlin`.

## ugos_time_show

Show time zone, time and how the clock is kept

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_time_zones

List the time zones

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_update_clean

Clear the stored failure of the last firmware install

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_update_config

Show or change how the NAS looks for updates (auto: notify, important, any; check interval). Auto-install is refused unless force is given: ask the user first

- `auto`: `notify`, `important` or `any`. The last two install by themselves and
restart the NAS, which needs force.
- `default_interval`: Let UGOS decide how often to check.
- `every`: Check every this many hours, days or months (1 to 72), with `unit`.
- `force`: Go on although the NAS then installs updates by itself. Ask the user
first.
- `target`: Target NAS name or host. Required when multiple targets are configured.
- `unit`: `hours`, `days` or `months`.

## ugos_update_install

Install the update that was found. The NAS restarts. Refused unless force is given: ask the user first

- `force`: Go on although this restarts the NAS. Ask the user first.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_update_key

Trust a firmware signing key from a .pub file. Refused unless force is given: ask the user first

- `file` (required): Absolute path of the file.
- `force`: Go on although this installs, trusts or overwrites. Ask the user
first.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_update_progress

Progress of a firmware install from a file

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_update_status

Show the installed firmware and the update that was found (version, size, changes, state)

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_update_upload

Install a firmware image (.img) from a file on this machine, in chunks. The NAS flashes and restarts. Refused unless force is given: ask the user first

- `file` (required): Absolute path of the file.
- `force`: Go on although this installs, trusts or overwrites. Ask the user
first.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_usb_list

List USB devices available to a VM

- `target`: Target NAS name or host. Required when multiple targets are configured.
- `vm` (required): VM display name or UUID.

## ugos_user_delete

Delete a user account. Its home folder and the files in it go with it, so an account with files or snapshots is refused unless force is given; ask the user first

- `account` (required): Login name or numeric uid.
- `force`: Delete although the account has snapshots or files in its home folder,
which go with it.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_user_list

List user accounts with role, groups and status

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_user_show

Show one user account

- `account` (required): Login name or numeric uid.
- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_user_update

Change a user account: email, description, groups, home folder, password rule, disable or expiry. Cannot set a password or a role; use ugos-cli for those. Refuses to lock out the account this server is logged in with

- `account` (required): Login name or numeric uid.
- `add_groups`: Groups to join.
- `deny_password_change`: Forbid (`true`) or allow (`false`) the account to change its own password.
- `description`: New description.
- `disable`: Disable the account.
- `email`: New email address. The stored one is kept when left out.
- `enable`: Enable a disabled account, or lift an expiry.
- `expires`: Let the account expire on a date, `YYYY-MM-DD`.
- `home_dir`: Give (`true`) or take away (`false`) the home folder.
- `remove_groups`: Groups to leave. The built-in groups cannot be left.
- `target`: Target NAS name or host. Required when multiple targets are configured.

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

## ugos_volume_list

List the NAS volumes (file systems on the pools) with usage

- `target`: Target NAS name or host. Required when multiple targets are configured.

## ugos_volume_show

Show one NAS volume, found by name, label or mount path

- `name` (required): What to look at: a name, label or id, as the matching list tool shows it.
- `target`: Target NAS name or host. Required when multiple targets are configured.

