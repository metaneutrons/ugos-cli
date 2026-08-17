//! Construction of a [`VmDetail`] request body from `vm create` and
//! `vm update` flags.
//!
//! The flags are deliberately thin wrappers around the fields the UGOS
//! `CreateVirtualMachine` and `UpdateVirtualMachine` endpoints accept. Every
//! device is repeatable and parsed from either a short form (`--disk 40g`,
//! `--iso /path.iso`, `--nic vnet-bridge0`) or a `key=value,...` list.
//!
//! Both entry points work the same way: take a base spec, apply the flags on
//! top, then fill in what the flags left open. For [`build`] the base is empty
//! (or a `--spec-file`) and missing fields get the defaults the web UI uses;
//! for [`update`] the base is the VM's current configuration and nothing is
//! defaulted, so untouched fields survive verbatim. A repeatable flag that is
//! used replaces the corresponding list wholesale; [`update`] additionally
//! offers `--add-*`, `--set-*` and `--rm-*` for incremental edits.
//!
//! Device names (`vda`, `hda`, …) and boot order are assigned automatically
//! from the bus type and the order the flags appear in, unless `dev=` or
//! `order=` says otherwise.

use std::collections::BTreeSet;
use std::io::Read;

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Map, Value};
use ugos_client::types::kvm::{
    VmDetail, VmDevice, VmDisk, VmImage, VmNetwork, VmOtherConfig, VmResource,
};

use crate::cli::{VmCreateArgs, VmEditFlags, VmSpecFlags, VmUpdateArgs};

// ── Defaults ────────────────────────────────────────────────────────

const DEFAULT_OS: &str = "linux";
const DEFAULT_NETWORK: &str = "vnet-bridge0";
const DEFAULT_NIC_TYPE: &str = "virtio";
const DEFAULT_DISK_BUS: &str = "virtio";
const DEFAULT_BOOT_TYPE: &str = "uefi";
const DEFAULT_STORAGE: &str = "volume1";
const DEFAULT_GRAPHICS: &str = "virtio";
const DEFAULT_KEYBOARD: &str = "en-us";
const DEFAULT_USB_CONTROLLER: i64 = 2;
const DEFAULT_ISO_PREFIX: &str = "hd";

// ── Entry point ─────────────────────────────────────────────────────

/// Build a VM spec from the `vm create` flags.
///
/// # Errors
///
/// Returns an error when a device spec cannot be parsed, when required values
/// are missing, or when the resulting configuration is inconsistent (duplicate
/// device names, unusable bus type, …).
pub fn build(args: &VmCreateArgs) -> Result<VmDetail> {
    let has_base = args.spec_file.is_some();
    let mut spec = match &args.spec_file {
        Some(path) => load_base(path)?,
        None => empty_spec(),
    };

    // A create always gets a fresh UUID; the client fills it in.
    spec.virtual_machine_name = String::new();
    args.name.clone_into(&mut spec.virtual_machine_display_name);

    apply_flags(&mut spec, &args.spec, !has_base)?;

    assign_devs(&mut spec, true)?;
    assign_order(&mut spec);
    validate_create(&spec, args, has_base)?;

    Ok(spec)
}

/// Build an updated VM spec from a VM's current configuration and the
/// `vm update` flags.
///
/// The VM's UUID is always taken from `current`, never from a spec file, so an
/// update cannot be redirected at a different VM by accident.
///
/// # Errors
///
/// Returns an error when a device spec cannot be parsed, when a `--set-*` or
/// `--rm-*` selector matches nothing, or when the result is inconsistent.
pub fn update(current: &VmDetail, args: &VmUpdateArgs) -> Result<VmDetail> {
    let mut spec = match &args.spec_file {
        Some(path) => load_base(path)?,
        None => current.clone(),
    };

    current
        .virtual_machine_name
        .clone_into(&mut spec.virtual_machine_name);
    match &args.rename {
        Some(new) => new.clone_into(&mut spec.virtual_machine_display_name),
        None => current
            .virtual_machine_display_name
            .clone_into(&mut spec.virtual_machine_display_name),
    }

    apply_flags(&mut spec, &args.spec, false)?;
    apply_edits(&mut spec, &args.edits)?;

    assign_devs(&mut spec, false)?;
    assign_order(&mut spec);
    validate_common(&spec)?;

    Ok(spec)
}

// ── Flag application ────────────────────────────────────────────────

/// Apply the flags shared by create and update.
///
/// With `defaults`, empty fields are filled with the values the UGOS web UI
/// uses for a new VM; an update leaves them as the VM has them.
fn apply_flags(spec: &mut VmDetail, flags: &VmSpecFlags, defaults: bool) -> Result<()> {
    // ── Scalars ─────────────────────────────────────────────────────

    apply_str(
        &mut spec.system_type,
        flags.os.as_ref(),
        DEFAULT_OS,
        defaults,
    );
    if let Some(v) = &flags.os_version {
        v.clone_into(&mut spec.system_version);
    }
    apply_str(
        &mut spec.storage_name,
        flags.storage.as_ref(),
        DEFAULT_STORAGE,
        defaults,
    );
    apply_str(
        &mut spec.device.boot_type,
        flags.boot_type.as_ref(),
        DEFAULT_BOOT_TYPE,
        defaults,
    );
    apply_str(
        &mut spec.device.graphics_card,
        flags.graphics.as_ref(),
        DEFAULT_GRAPHICS,
        defaults,
    );
    apply_str(
        &mut spec.other_config.keyboard_language,
        flags.keyboard.as_ref(),
        DEFAULT_KEYBOARD,
        defaults,
    );

    if let Some(n) = flags.usb_controller {
        ensure!(n >= 0, "usb-controller must be >= 0, got {n}");
        spec.device.usb_controller = n;
    } else if defaults && spec.device.usb_controller == 0 {
        spec.device.usb_controller = DEFAULT_USB_CONTROLLER;
    }

    if let Some(a) = flags.autostart {
        spec.other_config.auto_matic_start_up = a;
    }

    if let Some(c) = flags.cores {
        spec.core.value = c;
    }
    if let Some(m) = &flags.memory {
        spec.memory.value = parse_size_kib(m).context("invalid --memory")?;
    }

    // ── Devices ─────────────────────────────────────────────────────

    if !flags.disk.is_empty() {
        spec.dists = flags
            .disk
            .iter()
            .map(|s| parse_disk(s))
            .collect::<Result<Vec<_>>>()?;
    }
    if !flags.iso.is_empty() {
        spec.images = flags
            .iso
            .iter()
            .map(|s| parse_iso(s))
            .collect::<Result<Vec<_>>>()?;
    }
    if !flags.nic.is_empty() {
        spec.networks = flags
            .nic
            .iter()
            .map(|s| parse_nic(s))
            .collect::<Result<Vec<_>>>()?;
    } else if let Some(name) = &flags.network {
        spec.networks = vec![nic(name)];
    } else if defaults && spec.networks.is_empty() {
        spec.networks = vec![nic(DEFAULT_NETWORK)];
    }
    if !flags.usb.is_empty() {
        spec.device.usb_devices = flags
            .usb
            .iter()
            .map(|s| parse_usb(s))
            .collect::<Result<Vec<_>>>()?;
    }
    if !flags.share.is_empty() {
        spec.other_config.share_directory = flags
            .share
            .iter()
            .map(|s| parse_share(s))
            .collect::<Result<Vec<_>>>()?;
    }

    Ok(())
}

/// Apply the incremental `--rm-*`, `--set-*` and `--add-*` edits, in that
/// order, so that an edit never touches an entry added in the same command.
fn apply_edits(spec: &mut VmDetail, edits: &VmEditFlags) -> Result<()> {
    for sel in &edits.rm_disk {
        remove_where(&mut spec.dists, |d| d.dev == *sel, "disk", sel)?;
    }
    for sel in &edits.rm_iso {
        remove_where(
            &mut spec.images,
            |i| i.dev == *sel || i.path == *sel,
            "iso",
            sel,
        )?;
    }
    for sel in &edits.rm_nic {
        remove_where(
            &mut spec.networks,
            |n| n.name == *sel || n.mac_address == *sel,
            "nic",
            sel,
        )?;
    }

    for input in &edits.set_disk {
        set_disk(&mut spec.dists, input)?;
    }
    for input in &edits.set_iso {
        set_iso(&mut spec.images, input)?;
    }
    for input in &edits.set_nic {
        set_nic(&mut spec.networks, input)?;
    }

    for input in &edits.add_disk {
        spec.dists.push(parse_disk(input)?);
    }
    for input in &edits.add_iso {
        spec.images.push(parse_iso(input)?);
    }
    for input in &edits.add_nic {
        spec.networks.push(parse_nic(input)?);
    }

    Ok(())
}

fn remove_where<T>(
    items: &mut Vec<T>,
    matches: impl Fn(&T) -> bool,
    what: &str,
    selector: &str,
) -> Result<()> {
    let before = items.len();
    items.retain(|item| !matches(item));
    ensure!(
        items.len() < before,
        "no {what} matching '{selector}' on this VM"
    );
    Ok(())
}

/// Pull the `match=` selector out of a `--set-*` spec.
fn selector<'a>(pairs: &[(&'a str, &'a str)], flag: &str) -> Result<&'a str> {
    let sel = pairs
        .iter()
        .find(|(k, _)| *k == "match")
        .map(|(_, v)| *v)
        .with_context(|| format!("{flag} needs a match= selector, e.g. {flag} match=vda,..."))?;
    ensure!(!sel.is_empty(), "{flag} has an empty match= selector");
    Ok(sel)
}

fn set_disk(disks: &mut [VmDisk], input: &str) -> Result<()> {
    let pairs = parse_kv(input, &["match", "size", "bus", "dev", "path", "order"])?;
    let sel = selector(&pairs, "--set-disk")?;
    let disk = disks
        .iter_mut()
        .find(|d| d.dev == sel)
        .with_context(|| format!("no disk with dev '{sel}' on this VM"))?;
    for (k, v) in pairs {
        match k {
            "match" => {}
            "size" => disk.size = parse_size_kib(v).context("invalid disk size")?,
            "bus" => v.clone_into(&mut disk.bus),
            "dev" => v.clone_into(&mut disk.dev),
            "path" => v.clone_into(&mut disk.path),
            "order" => disk.order = parse_order(v)?,
            other => bail!("unhandled disk key '{other}'"),
        }
    }
    ensure!(disk.size > 0, "disk size must be > 0, got {}", disk.size);
    Ok(())
}

fn set_iso(images: &mut [VmImage], input: &str) -> Result<()> {
    let pairs = parse_kv(input, &["match", "path", "dev", "order"])?;
    let sel = selector(&pairs, "--set-iso")?;
    let image = images
        .iter_mut()
        .find(|i| i.dev == sel || i.path == sel)
        .with_context(|| format!("no iso matching '{sel}' on this VM"))?;
    for (k, v) in pairs {
        match k {
            "match" => {}
            "path" => v.clone_into(&mut image.path),
            "dev" => v.clone_into(&mut image.dev),
            "order" => image.order = parse_order(v)?,
            other => bail!("unhandled iso key '{other}'"),
        }
    }
    ensure!(!image.path.is_empty(), "iso path cannot be empty");
    Ok(())
}

fn set_nic(networks: &mut [VmNetwork], input: &str) -> Result<()> {
    let pairs = parse_kv(input, &["match", "name", "type", "mac"])?;
    let sel = selector(&pairs, "--set-nic")?;
    let network = networks
        .iter_mut()
        .find(|n| n.name == sel || n.mac_address == sel)
        .with_context(|| format!("no nic matching '{sel}' on this VM"))?;
    for (k, v) in pairs {
        match k {
            "match" => {}
            "name" => v.clone_into(&mut network.name),
            "type" => v.clone_into(&mut network.nic_type),
            "mac" => {
                validate_mac(v)?;
                v.clone_into(&mut network.mac_address);
            }
            other => bail!("unhandled nic key '{other}'"),
        }
    }
    ensure!(!network.name.is_empty(), "nic name cannot be empty");
    Ok(())
}

// ── Base spec ───────────────────────────────────────────────────────

fn load_base(path: &str) -> Result<VmDetail> {
    let raw = if path == "-" {
        let mut buf = String::new();
        let _ = std::io::stdin()
            .read_to_string(&mut buf)
            .context("reading spec from stdin")?;
        buf
    } else {
        std::fs::read_to_string(path).with_context(|| format!("reading spec file '{path}'"))?
    };
    serde_json::from_str(&raw).with_context(|| format!("parsing spec from '{path}' as a VM spec"))
}

const fn empty_spec() -> VmDetail {
    VmDetail {
        virtual_machine_name: String::new(),
        virtual_machine_display_name: String::new(),
        system_type: String::new(),
        system_version: String::new(),
        core: VmResource { value: 0 },
        memory: VmResource { value: 0 },
        images: vec![],
        dists: vec![],
        networks: vec![],
        device: VmDevice {
            usb_controller: 0,
            usb_devices: vec![],
            graphics_card: String::new(),
            boot_type: String::new(),
        },
        other_config: VmOtherConfig {
            auto_matic_start_up: false,
            keyboard_language: String::new(),
            share_directory: vec![],
        },
        storage_name: String::new(),
        // Filled in from storage_name by the client, which needs the NAS to
        // resolve it.
        storage_uuid: String::new(),
    }
}

/// Override `field` with `arg`; with `defaults`, an otherwise empty `field`
/// falls back to `default`.
fn apply_str(field: &mut String, arg: Option<&String>, default: &str, defaults: bool) {
    if let Some(v) = arg {
        v.clone_into(field);
    } else if defaults && field.is_empty() {
        default.clone_into(field);
    }
}

// ── Device parsers ──────────────────────────────────────────────────

fn parse_disk(input: &str) -> Result<VmDisk> {
    let mut disk = VmDisk {
        bus: DEFAULT_DISK_BUS.to_owned(),
        size: 0,
        dev: String::new(),
        path: String::new(),
        order: 0,
    };
    if input.contains('=') {
        for (k, v) in parse_kv(input, &["size", "bus", "dev", "path", "order"])? {
            match k {
                "size" => disk.size = parse_size_kib(v).context("invalid disk size")?,
                "bus" => v.clone_into(&mut disk.bus),
                "dev" => v.clone_into(&mut disk.dev),
                "path" => v.clone_into(&mut disk.path),
                "order" => disk.order = parse_order(v)?,
                other => bail!("unhandled disk key '{other}'"),
            }
        }
    } else {
        disk.size = parse_size_kib(input).context("invalid disk size")?;
    }
    ensure!(
        disk.size > 0,
        "disk '{input}' needs a size greater than 0 (e.g. --disk size=40g)"
    );
    ensure!(!disk.bus.is_empty(), "disk '{input}' has an empty bus");
    Ok(disk)
}

fn parse_iso(input: &str) -> Result<VmImage> {
    let mut image = VmImage {
        path: String::new(),
        dev: String::new(),
        order: 0,
    };
    if input.contains('=') {
        for (k, v) in parse_kv(input, &["path", "dev", "order"])? {
            match k {
                "path" => v.clone_into(&mut image.path),
                "dev" => v.clone_into(&mut image.dev),
                "order" => image.order = parse_order(v)?,
                other => bail!("unhandled iso key '{other}'"),
            }
        }
    } else {
        input.trim().clone_into(&mut image.path);
    }
    ensure!(
        !image.path.is_empty(),
        "iso '{input}' needs a path (e.g. --iso /volume1/iso/x.iso)"
    );
    Ok(image)
}

fn parse_nic(input: &str) -> Result<VmNetwork> {
    if !input.contains('=') {
        return Ok(nic(input.trim()));
    }
    let mut network = VmNetwork {
        name: String::new(),
        mac_address: String::new(),
        nic_type: DEFAULT_NIC_TYPE.to_owned(),
    };
    for (k, v) in parse_kv(input, &["name", "type", "mac"])? {
        match k {
            "name" => v.clone_into(&mut network.name),
            "type" => v.clone_into(&mut network.nic_type),
            "mac" => {
                validate_mac(v)?;
                v.clone_into(&mut network.mac_address);
            }
            other => bail!("unhandled nic key '{other}'"),
        }
    }
    ensure!(
        !network.name.is_empty(),
        "nic '{input}' needs a name (e.g. --nic name=vnet-bridge0)"
    );
    Ok(network)
}

fn nic(name: &str) -> VmNetwork {
    VmNetwork {
        name: name.to_owned(),
        mac_address: String::new(),
        nic_type: DEFAULT_NIC_TYPE.to_owned(),
    }
}

/// Parse a USB passthrough device.
///
/// The field names mirror the verified `USBList` response. Whether
/// `CreateVirtualMachine` expects the same shape is **not** verified against a
/// live NAS — pass a raw JSON object to control the body exactly.
fn parse_usb(input: &str) -> Result<Value> {
    if let Some(v) = parse_json_object(input, "usb")? {
        return Ok(v);
    }
    let mut map = Map::new();
    for (k, v) in parse_kv(
        input,
        &[
            "vendor-id",
            "product-id",
            "bus-id",
            "device-id",
            "vendor-name",
            "product-name",
        ],
    )? {
        match k {
            "vendor-id" => put(&mut map, "vendorID", v.into()),
            "product-id" => put(&mut map, "productID", v.into()),
            "vendor-name" => put(&mut map, "vendorName", v.into()),
            "product-name" => put(&mut map, "productName", v.into()),
            "bus-id" => put(&mut map, "busID", parse_i64(v, "bus-id")?.into()),
            "device-id" => put(&mut map, "deviceID", parse_i64(v, "device-id")?.into()),
            other => bail!("unhandled usb key '{other}'"),
        }
    }
    ensure!(!map.is_empty(), "usb '{input}' is empty");
    Ok(Value::Object(map))
}

/// Parse a shared directory entry.
///
/// The request schema for shared directories is unknown, so keys are passed
/// through verbatim and all values stay strings. Use a raw JSON object when the
/// endpoint needs other types.
fn parse_share(input: &str) -> Result<Value> {
    if let Some(v) = parse_json_object(input, "share")? {
        return Ok(v);
    }
    let mut map = Map::new();
    for part in input.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some((k, v)) = part.split_once('=') else {
            bail!("expected key=value or a JSON object in share '{input}'");
        };
        put(&mut map, k.trim(), v.trim().into());
    }
    ensure!(!map.is_empty(), "share '{input}' is empty");
    Ok(Value::Object(map))
}

/// Insert into a JSON object, discarding any replaced value.
fn put(map: &mut Map<String, Value>, key: &str, value: Value) {
    let _ = map.insert(key.to_owned(), value);
}

fn parse_json_object(input: &str, what: &str) -> Result<Option<Value>> {
    let trimmed = input.trim();
    if !trimmed.starts_with('{') {
        return Ok(None);
    }
    let value: Value =
        serde_json::from_str(trimmed).with_context(|| format!("parsing {what} JSON '{input}'"))?;
    ensure!(
        value.is_object(),
        "{what} JSON must be an object: '{input}'"
    );
    Ok(Some(value))
}

// ── Value parsers ───────────────────────────────────────────────────

/// Split a `key=value,...` list, rejecting keys outside `allowed`.
fn parse_kv<'a>(input: &'a str, allowed: &[&str]) -> Result<Vec<(&'a str, &'a str)>> {
    let mut out = Vec::new();
    for part in input.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some((k, v)) = part.split_once('=') else {
            bail!(
                "expected key=value in '{part}', allowed keys: {}",
                allowed.join(", ")
            );
        };
        let k = k.trim();
        ensure!(
            allowed.contains(&k),
            "unknown key '{k}', allowed keys: {}",
            allowed.join(", ")
        );
        out.push((k, v.trim()));
    }
    ensure!(!out.is_empty(), "no key=value pairs in '{input}'");
    Ok(out)
}

/// Parse a size into bytes. A bare number is MiB; `k`, `m`, `g` and `t`
/// suffixes (also `kb`/`kib` spellings) select binary multiples.
fn parse_size_bytes(input: &str) -> Result<i64> {
    let s = input.trim().to_ascii_lowercase();
    ensure!(!s.is_empty(), "empty size value");
    let split = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let (digits, unit) = s.split_at(split);
    ensure!(
        !digits.is_empty(),
        "invalid size '{input}': expected a number, optionally followed by k, m, g or t"
    );
    let value: i64 = digits
        .parse()
        .with_context(|| format!("invalid size '{input}'"))?;
    let mult: i64 = match unit {
        "" | "m" | "mb" | "mib" => 1024 * 1024,
        "k" | "kb" | "kib" => 1024,
        "g" | "gb" | "gib" => 1024 * 1024 * 1024,
        "t" | "tb" | "tib" => 1024 * 1024 * 1024 * 1024,
        other => bail!("invalid size unit '{other}' in '{input}', expected: k, m, g, t"),
    };
    value
        .checked_mul(mult)
        .with_context(|| format!("size '{input}' is too large"))
}

/// Parse a size into KiB, the unit UGOS uses for both memory and disk sizes.
fn parse_size_kib(input: &str) -> Result<i64> {
    Ok(parse_size_bytes(input)? / 1024)
}

fn parse_order(input: &str) -> Result<i64> {
    let order = parse_i64(input, "order")?;
    ensure!(order > 0, "boot order must be > 0, got {order}");
    Ok(order)
}

fn parse_i64(input: &str, what: &str) -> Result<i64> {
    input
        .trim()
        .parse()
        .with_context(|| format!("invalid {what} '{input}': expected a number"))
}

fn validate_mac(mac: &str) -> Result<()> {
    let groups: Vec<&str> = mac.split([':', '-']).collect();
    ensure!(
        groups.len() == 6
            && groups
                .iter()
                .all(|g| g.len() == 2 && g.chars().all(|c| c.is_ascii_hexdigit())),
        "invalid MAC address '{mac}', expected six hex pairs (e.g. 52:54:00:12:34:56)"
    );
    Ok(())
}

// ── Device name and boot order assignment ───────────────────────────

/// Map a disk bus to the device name prefix the guest will see.
fn bus_prefix(bus: &str) -> Result<&'static str> {
    match bus.to_ascii_lowercase().as_str() {
        "virtio" => Ok("vd"),
        "sata" | "scsi" | "usb" => Ok("sd"),
        "ide" => Ok("hd"),
        other => bail!("cannot derive a device name for bus '{other}', set dev= explicitly"),
    }
}

/// Fill in empty `dev` fields, skipping names that are already taken.
fn assign_devs(spec: &mut VmDetail, name_disks: bool) -> Result<()> {
    let mut taken = BTreeSet::new();
    for dev in spec
        .dists
        .iter()
        .map(|d| &d.dev)
        .chain(spec.images.iter().map(|i| &i.dev))
        .filter(|d| !d.is_empty())
    {
        ensure!(
            taken.insert(dev.clone()),
            "duplicate device name '{dev}' — device names must be unique per VM"
        );
    }

    // On an update UGOS rejects a new disk that already carries a device name
    // and picks one itself, so only a create names its disks.
    if name_disks {
        for disk in &mut spec.dists {
            if disk.dev.is_empty() {
                disk.dev = next_dev(bus_prefix(&disk.bus)?, &mut taken)?;
            }
        }
    }
    for image in &mut spec.images {
        if image.dev.is_empty() {
            image.dev = next_dev(DEFAULT_ISO_PREFIX, &mut taken)?;
        }
    }
    Ok(())
}

fn next_dev(prefix: &str, taken: &mut BTreeSet<String>) -> Result<String> {
    for suffix in 'a'..='z' {
        let name = format!("{prefix}{suffix}");
        if taken.insert(name.clone()) {
            return Ok(name);
        }
    }
    bail!("no free device name left for prefix '{prefix}'")
}

/// Fill in unset boot orders: disks first, then ISOs, skipping taken numbers.
fn assign_order(spec: &mut VmDetail) {
    let taken: BTreeSet<i64> = spec
        .dists
        .iter()
        .map(|d| d.order)
        .chain(spec.images.iter().map(|i| i.order))
        .filter(|o| *o > 0)
        .collect();
    let mut next = 1;
    let mut take_next = |taken: &BTreeSet<i64>| {
        while taken.contains(&next) {
            next += 1;
        }
        let order = next;
        next += 1;
        order
    };
    for disk in &mut spec.dists {
        if disk.order == 0 {
            disk.order = take_next(&taken);
        }
    }
    for image in &mut spec.images {
        if image.order == 0 {
            image.order = take_next(&taken);
        }
    }
}

// ── Validation ──────────────────────────────────────────────────────

/// Checks that apply to any spec, whether created or updated.
fn validate_common(spec: &VmDetail) -> Result<()> {
    ensure!(
        !spec.virtual_machine_display_name.is_empty(),
        "VM name cannot be empty"
    );
    ensure!(
        matches!(spec.system_type.as_str(), "linux" | "windows" | "other"),
        "invalid OS type '{}', expected: linux, windows, other",
        spec.system_type
    );
    ensure!(
        matches!(spec.device.boot_type.as_str(), "uefi" | "bios"),
        "invalid boot type '{}', expected: uefi, bios",
        spec.device.boot_type
    );
    ensure!(
        spec.core.value > 0,
        "cores must be > 0, got {}",
        spec.core.value
    );
    ensure!(
        spec.memory.value > 0,
        "memory must be > 0, got {} KiB",
        spec.memory.value
    );
    ensure!(
        !spec.dists.is_empty() || !spec.images.is_empty(),
        "a VM needs at least one disk or ISO image"
    );
    Ok(())
}

/// Additional checks for `vm create`, where nothing pre-exists to fall back on.
fn validate_create(spec: &VmDetail, args: &VmCreateArgs, has_base: bool) -> Result<()> {
    if !has_base {
        if args.spec.cores.is_none() {
            bail!("missing --cores");
        }
        if args.spec.memory.is_none() {
            bail!("missing --memory");
        }
        if args.spec.disk.is_empty() && args.spec.iso.is_empty() {
            bail!("missing --disk (or --iso for a diskless VM)");
        }
    }
    validate_common(spec)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// A minimal valid `vm create` invocation.
    fn args(name: &str) -> VmCreateArgs {
        VmCreateArgs {
            name: name.to_owned(),
            spec: VmSpecFlags {
                cores: Some(2),
                memory: Some("4096".to_owned()),
                disk: vec!["20480".to_owned()],
                ..VmSpecFlags::default()
            },
            ..VmCreateArgs::default()
        }
    }

    /// A VM as it would come back from `vm show`: one disk, one NIC, a UUID and
    /// a disk path the NAS has filled in.
    fn existing() -> VmDetail {
        let mut vm = build(&args("CachyOS")).unwrap();
        vm.virtual_machine_name = "797fb54f-0000-0000-0000-000000000001".to_owned();
        vm.dists[0].path = "/volume1/kvm/cachyos/disk0.qcow2".to_owned();
        vm
    }

    fn update_args(name: &str) -> VmUpdateArgs {
        VmUpdateArgs {
            name: name.to_owned(),
            ..VmUpdateArgs::default()
        }
    }

    // ── create: defaults match the pre-flexibility behaviour ────────

    #[test]
    fn defaults_unchanged() {
        let spec = build(&args("TestVM")).unwrap();
        assert_eq!(spec.virtual_machine_display_name, "TestVM");
        assert_eq!(spec.system_type, "linux");
        assert_eq!(spec.core.value, 2);
        assert_eq!(spec.memory.value, 4096 * 1024);
        assert_eq!(spec.dists.len(), 1);
        assert_eq!(spec.dists[0].size, 20480 * 1024);
        assert_eq!(spec.dists[0].bus, "virtio");
        assert_eq!(spec.dists[0].dev, "vda");
        assert_eq!(spec.dists[0].order, 1);
        assert_eq!(spec.networks.len(), 1);
        assert_eq!(spec.networks[0].name, "vnet-bridge0");
        assert_eq!(spec.networks[0].nic_type, "virtio");
        assert_eq!(spec.device.boot_type, "uefi");
        assert_eq!(spec.device.graphics_card, "virtio");
        assert_eq!(spec.device.usb_controller, 2);
        assert_eq!(spec.other_config.keyboard_language, "en-us");
        assert_eq!(spec.storage_name, "volume1");
        assert!(!spec.other_config.auto_matic_start_up);
        assert!(spec.virtual_machine_name.is_empty());
    }

    #[test]
    fn single_iso_keeps_hda_and_order_two() {
        let mut a = args("TestVM");
        a.spec.iso = vec!["/volume1/iso/ubuntu.iso".to_owned()];
        let spec = build(&a).unwrap();
        assert_eq!(spec.images.len(), 1);
        assert_eq!(spec.images[0].path, "/volume1/iso/ubuntu.iso");
        assert_eq!(spec.images[0].dev, "hda");
        assert_eq!(spec.images[0].order, 2);
    }

    // ── create: multiple devices ────────────────────────────────────

    #[test]
    fn multiple_disks_get_sequential_devs() {
        let mut a = args("TestVM");
        a.spec.disk = vec!["20g".to_owned(), "size=100g".to_owned(), "1t".to_owned()];
        let spec = build(&a).unwrap();
        let devs: Vec<&str> = spec.dists.iter().map(|d| d.dev.as_str()).collect();
        assert_eq!(devs, ["vda", "vdb", "vdc"]);
        assert_eq!(spec.dists[1].size, 100 * 1024 * 1024);
        assert_eq!(spec.dists[2].size, 1024 * 1024 * 1024);
        let orders: Vec<i64> = spec.dists.iter().map(|d| d.order).collect();
        assert_eq!(orders, [1, 2, 3]);
    }

    #[test]
    fn mixed_buses_use_separate_prefixes() {
        let mut a = args("TestVM");
        a.spec.disk = vec![
            "size=20g,bus=sata".to_owned(),
            "size=20g".to_owned(),
            "size=20g,bus=sata".to_owned(),
        ];
        let spec = build(&a).unwrap();
        let devs: Vec<&str> = spec.dists.iter().map(|d| d.dev.as_str()).collect();
        assert_eq!(devs, ["sda", "vda", "sdb"]);
    }

    #[test]
    fn explicit_dev_is_reserved_before_auto_assignment() {
        let mut a = args("TestVM");
        a.spec.disk = vec!["size=20g".to_owned(), "size=20g,dev=vda".to_owned()];
        let spec = build(&a).unwrap();
        // vda is claimed explicitly, so the first disk moves to vdb.
        assert_eq!(spec.dists[0].dev, "vdb");
        assert_eq!(spec.dists[1].dev, "vda");
    }

    #[test]
    fn ide_disk_and_iso_do_not_collide() {
        let mut a = args("TestVM");
        a.spec.disk = vec!["size=20g,bus=ide".to_owned()];
        a.spec.iso = vec!["/x.iso".to_owned()];
        let spec = build(&a).unwrap();
        assert_eq!(spec.dists[0].dev, "hda");
        assert_eq!(spec.images[0].dev, "hdb");
    }

    #[test]
    fn explicit_order_is_kept_and_others_skip_it() {
        let mut a = args("TestVM");
        a.spec.disk = vec!["size=20g".to_owned(), "size=20g".to_owned()];
        a.spec.iso = vec!["path=/x.iso,order=1".to_owned()];
        let spec = build(&a).unwrap();
        assert_eq!(spec.images[0].order, 1);
        assert_eq!(spec.dists[0].order, 2);
        assert_eq!(spec.dists[1].order, 3);
    }

    #[test]
    fn multiple_nics() {
        let mut a = args("TestVM");
        a.spec.nic = vec![
            "vnet-bridge0".to_owned(),
            "name=vnet-nat,type=e1000,mac=52:54:00:12:34:56".to_owned(),
        ];
        let spec = build(&a).unwrap();
        assert_eq!(spec.networks.len(), 2);
        assert_eq!(spec.networks[0].name, "vnet-bridge0");
        assert_eq!(spec.networks[1].nic_type, "e1000");
        assert_eq!(spec.networks[1].mac_address, "52:54:00:12:34:56");
    }

    #[test]
    fn network_flag_still_works() {
        let mut a = args("TestVM");
        a.spec.network = Some("vnet-bridge1".to_owned());
        let spec = build(&a).unwrap();
        assert_eq!(spec.networks.len(), 1);
        assert_eq!(spec.networks[0].name, "vnet-bridge1");
    }

    #[test]
    fn nic_flag_wins_over_network_flag() {
        let mut a = args("TestVM");
        a.spec.network = Some("vnet-bridge1".to_owned());
        a.spec.nic = vec!["vnet-bridge9".to_owned()];
        let spec = build(&a).unwrap();
        assert_eq!(spec.networks.len(), 1);
        assert_eq!(spec.networks[0].name, "vnet-bridge9");
    }

    #[test]
    fn usb_key_value_and_json() {
        let mut a = args("TestVM");
        a.spec.usb = vec![
            "vendor-id=0x8087,product-id=0x0033,bus-id=1,device-id=4".to_owned(),
            r#"{"vendorID":"0x1d6b","busID":2}"#.to_owned(),
        ];
        let spec = build(&a).unwrap();
        assert_eq!(spec.device.usb_devices.len(), 2);
        assert_eq!(spec.device.usb_devices[0]["vendorID"], "0x8087");
        assert_eq!(spec.device.usb_devices[0]["busID"], 1);
        assert_eq!(spec.device.usb_devices[1]["busID"], 2);
    }

    #[test]
    fn share_passes_keys_through() {
        let mut a = args("TestVM");
        a.spec.share = vec!["name=docs,path=/volume1/docs".to_owned()];
        let spec = build(&a).unwrap();
        assert_eq!(
            spec.other_config.share_directory[0]["path"],
            "/volume1/docs"
        );
    }

    #[test]
    fn device_knobs_are_settable() {
        let mut a = args("TestVM");
        a.spec.os = Some("windows".to_owned());
        a.spec.os_version = Some("win11".to_owned());
        a.spec.graphics = Some("qxl".to_owned());
        a.spec.keyboard = Some("de".to_owned());
        a.spec.usb_controller = Some(0);
        a.spec.boot_type = Some("bios".to_owned());
        a.spec.storage = Some("volume2".to_owned());
        a.spec.autostart = Some(true);
        let spec = build(&a).unwrap();
        assert_eq!(spec.system_type, "windows");
        assert_eq!(spec.system_version, "win11");
        assert_eq!(spec.device.graphics_card, "qxl");
        assert_eq!(spec.other_config.keyboard_language, "de");
        assert_eq!(spec.device.usb_controller, 0);
        assert_eq!(spec.device.boot_type, "bios");
        assert_eq!(spec.storage_name, "volume2");
        assert!(spec.other_config.auto_matic_start_up);
    }

    // ── Sizes ───────────────────────────────────────────────────────

    #[test]
    fn size_units() {
        assert_eq!(parse_size_bytes("100").unwrap(), 100 * 1024 * 1024);
        assert_eq!(parse_size_bytes("100m").unwrap(), 100 * 1024 * 1024);
        assert_eq!(parse_size_bytes("100MiB").unwrap(), 100 * 1024 * 1024);
        assert_eq!(parse_size_bytes("2g").unwrap(), 2 * 1024 * 1024 * 1024);
        assert_eq!(parse_size_bytes("512k").unwrap(), 512 * 1024);
    }

    #[test]
    fn memory_accepts_suffix() {
        let mut a = args("TestVM");
        a.spec.memory = Some("8g".to_owned());
        let spec = build(&a).unwrap();
        assert_eq!(spec.memory.value, 8 * 1024 * 1024);
    }

    #[test]
    fn size_rejects_unknown_unit() {
        let err = parse_size_bytes("10x").unwrap_err().to_string();
        assert!(err.contains("invalid size unit"), "{err}");
    }

    // ── create: validation ──────────────────────────────────────────

    #[test]
    fn rejects_bad_os() {
        let mut a = args("Test");
        a.spec.os = Some("freebsd".to_owned());
        let err = build(&a).unwrap_err().to_string();
        assert!(err.contains("invalid OS type"), "{err}");
    }

    #[test]
    fn rejects_bad_boot_type() {
        let mut a = args("Test");
        a.spec.boot_type = Some("grub".to_owned());
        let err = build(&a).unwrap_err().to_string();
        assert!(err.contains("invalid boot type"), "{err}");
    }

    #[test]
    fn rejects_zero_cores() {
        let mut a = args("Test");
        a.spec.cores = Some(0);
        let err = build(&a).unwrap_err().to_string();
        assert!(err.contains("cores must be > 0"), "{err}");
    }

    #[test]
    fn rejects_zero_memory() {
        let mut a = args("Test");
        a.spec.memory = Some("0".to_owned());
        let err = build(&a).unwrap_err().to_string();
        assert!(err.contains("memory must be > 0"), "{err}");
    }

    #[test]
    fn rejects_empty_name() {
        let err = build(&args("")).unwrap_err().to_string();
        assert!(err.contains("VM name cannot be empty"), "{err}");
    }

    #[test]
    fn rejects_missing_cores() {
        let mut a = args("Test");
        a.spec.cores = None;
        let err = build(&a).unwrap_err().to_string();
        assert!(err.contains("missing --cores"), "{err}");
    }

    #[test]
    fn rejects_missing_disk_and_iso() {
        let mut a = args("Test");
        a.spec.disk = vec![];
        let err = build(&a).unwrap_err().to_string();
        assert!(err.contains("missing --disk"), "{err}");
    }

    #[test]
    fn rejects_duplicate_dev() {
        let mut a = args("Test");
        a.spec.disk = vec!["size=20g,dev=vda".to_owned(), "size=20g,dev=vda".to_owned()];
        let err = build(&a).unwrap_err().to_string();
        assert!(err.contains("duplicate device name"), "{err}");
    }

    #[test]
    fn rejects_unknown_bus_without_dev() {
        let mut a = args("Test");
        a.spec.disk = vec!["size=20g,bus=nvme".to_owned()];
        let err = build(&a).unwrap_err().to_string();
        assert!(err.contains("cannot derive a device name"), "{err}");
    }

    #[test]
    fn unknown_bus_is_fine_with_explicit_dev() {
        let mut a = args("Test");
        a.spec.disk = vec!["size=20g,bus=nvme,dev=nvme0n1".to_owned()];
        let spec = build(&a).unwrap();
        assert_eq!(spec.dists[0].dev, "nvme0n1");
    }

    #[test]
    fn rejects_unknown_key() {
        let mut a = args("Test");
        a.spec.disk = vec!["size=20g,cache=writeback".to_owned()];
        let err = build(&a).unwrap_err().to_string();
        assert!(err.contains("unknown key 'cache'"), "{err}");
    }

    #[test]
    fn rejects_bad_mac() {
        let mut a = args("Test");
        a.spec.nic = vec!["name=vnet-bridge0,mac=52:54:00".to_owned()];
        let err = build(&a).unwrap_err().to_string();
        assert!(err.contains("invalid MAC address"), "{err}");
    }

    // ── create: base spec merging ───────────────────────────────────

    #[test]
    fn base_spec_fills_gaps_and_flags_override() {
        let base = serde_json::to_string(&build(&args("Original")).unwrap()).unwrap();
        let file = std::env::temp_dir().join("ugos-vmspec-base-test.json");
        std::fs::write(&file, base).unwrap();

        let spec = build(&VmCreateArgs {
            name: "Clone".to_owned(),
            spec_file: Some(file.to_string_lossy().into_owned()),
            spec: VmSpecFlags {
                memory: Some("16g".to_owned()),
                ..VmSpecFlags::default()
            },
            ..VmCreateArgs::default()
        })
        .unwrap();

        // Taken from the base.
        assert_eq!(spec.core.value, 2);
        assert_eq!(spec.dists.len(), 1);
        assert_eq!(spec.dists[0].size, 20480 * 1024);
        assert_eq!(spec.networks[0].name, "vnet-bridge0");
        // Overridden by flags.
        assert_eq!(spec.virtual_machine_display_name, "Clone");
        assert_eq!(spec.memory.value, 16 * 1024 * 1024);
        // Never inherited.
        assert!(spec.virtual_machine_name.is_empty());

        std::fs::remove_file(&file).unwrap();
    }

    #[test]
    fn repeatable_flag_replaces_base_list() {
        let mut original = args("Original");
        original.spec.disk = vec!["20g".to_owned(), "30g".to_owned()];
        let base = serde_json::to_string(&build(&original).unwrap()).unwrap();
        let file = std::env::temp_dir().join("ugos-vmspec-replace-test.json");
        std::fs::write(&file, base).unwrap();

        let spec = build(&VmCreateArgs {
            name: "Clone".to_owned(),
            spec_file: Some(file.to_string_lossy().into_owned()),
            spec: VmSpecFlags {
                disk: vec!["size=40g,dev=vdz".to_owned()],
                ..VmSpecFlags::default()
            },
            ..VmCreateArgs::default()
        })
        .unwrap();

        assert_eq!(spec.dists.len(), 1);
        assert_eq!(spec.dists[0].dev, "vdz");

        std::fs::remove_file(&file).unwrap();
    }

    // ── update: scalars ─────────────────────────────────────────────

    #[test]
    fn update_changes_only_named_fields() {
        let current = existing();
        let mut a = update_args("CachyOS");
        a.spec.cores = Some(8);
        let spec = update(&current, &a).unwrap();

        assert_eq!(spec.core.value, 8);
        // Everything else stays as it was.
        assert_eq!(spec.memory.value, current.memory.value);
        assert_eq!(spec.system_type, "linux");
        assert_eq!(spec.dists[0].path, "/volume1/kvm/cachyos/disk0.qcow2");
        assert_eq!(spec.networks[0].name, "vnet-bridge0");
        assert_eq!(spec.virtual_machine_display_name, "CachyOS");
    }

    #[test]
    fn update_keeps_uuid() {
        let current = existing();
        let spec = update(&current, &update_args("CachyOS")).unwrap();
        assert_eq!(spec.virtual_machine_name, current.virtual_machine_name);
    }

    #[test]
    fn update_memory_accepts_suffix() {
        let mut a = update_args("CachyOS");
        a.spec.memory = Some("32g".to_owned());
        let spec = update(&existing(), &a).unwrap();
        assert_eq!(spec.memory.value, 32 * 1024 * 1024);
    }

    #[test]
    fn update_renames() {
        let mut a = update_args("CachyOS");
        a.rename = Some("CachyOS-2".to_owned());
        let spec = update(&existing(), &a).unwrap();
        assert_eq!(spec.virtual_machine_display_name, "CachyOS-2");
        assert_eq!(
            spec.virtual_machine_name,
            existing().virtual_machine_name,
            "rename must not touch the UUID"
        );
    }

    // ── update: incremental device edits ────────────────────────────

    #[test]
    fn update_adds_disk_next_to_existing_one() {
        let mut a = update_args("CachyOS");
        a.edits.add_disk = vec!["500g".to_owned()];
        let spec = update(&existing(), &a).unwrap();

        assert_eq!(spec.dists.len(), 2);
        assert_eq!(spec.dists[0].dev, "vda");
        assert_eq!(spec.dists[0].path, "/volume1/kvm/cachyos/disk0.qcow2");
        // UGOS rejects an update that names a new disk itself, so dev stays
        // empty and is left out of the request body.
        assert!(spec.dists[1].dev.is_empty());
        assert_eq!(spec.dists[1].order, 2);
        assert!(
            !serde_json::to_string(&spec.dists[1])
                .unwrap()
                .contains("dev")
        );
    }

    #[test]
    fn update_grows_a_disk_without_losing_its_path() {
        let mut a = update_args("CachyOS");
        a.edits.set_disk = vec!["match=vda,size=80g".to_owned()];
        let spec = update(&existing(), &a).unwrap();

        assert_eq!(spec.dists.len(), 1);
        assert_eq!(spec.dists[0].size, 80 * 1024 * 1024);
        assert_eq!(spec.dists[0].path, "/volume1/kvm/cachyos/disk0.qcow2");
        assert_eq!(spec.dists[0].dev, "vda");
    }

    #[test]
    fn update_sets_nic_type() {
        let mut a = update_args("CachyOS");
        a.edits.set_nic = vec!["match=vnet-bridge0,type=e1000,mac=52:54:00:12:34:56".to_owned()];
        let spec = update(&existing(), &a).unwrap();
        assert_eq!(spec.networks[0].nic_type, "e1000");
        assert_eq!(spec.networks[0].mac_address, "52:54:00:12:34:56");
    }

    #[test]
    fn update_removes_a_nic() {
        let mut current = existing();
        current.networks.push(nic("vnet-nat"));
        let mut a = update_args("CachyOS");
        a.edits.rm_nic = vec!["vnet-nat".to_owned()];
        let spec = update(&current, &a).unwrap();
        assert_eq!(spec.networks.len(), 1);
        assert_eq!(spec.networks[0].name, "vnet-bridge0");
    }

    #[test]
    fn update_removes_iso_by_path() {
        let mut current = existing();
        current.images.push(VmImage {
            path: "/volume1/iso/x.iso".to_owned(),
            dev: "hda".to_owned(),
            order: 2,
        });
        let mut a = update_args("CachyOS");
        a.edits.rm_iso = vec!["/volume1/iso/x.iso".to_owned()];
        let spec = update(&current, &a).unwrap();
        assert!(spec.images.is_empty());
    }

    #[test]
    fn update_removes_before_adding() {
        let mut a = update_args("CachyOS");
        a.edits.rm_disk = vec!["vda".to_owned()];
        a.edits.add_disk = vec!["size=100g,dev=vda".to_owned()];
        let spec = update(&existing(), &a).unwrap();

        assert_eq!(spec.dists.len(), 1);
        assert_eq!(spec.dists[0].dev, "vda");
        assert_eq!(spec.dists[0].size, 100 * 1024 * 1024);
        assert!(spec.dists[0].path.is_empty());
    }

    #[test]
    fn update_replaces_whole_list() {
        let mut current = existing();
        current.dists.push(VmDisk {
            bus: "virtio".to_owned(),
            size: 1024,
            dev: "vdb".to_owned(),
            path: String::new(),
            order: 2,
        });
        let mut a = update_args("CachyOS");
        a.spec.disk = vec!["40g".to_owned()];
        let spec = update(&current, &a).unwrap();
        assert_eq!(spec.dists.len(), 1);
        assert_eq!(spec.dists[0].size, 40 * 1024 * 1024);
    }

    #[test]
    fn update_rejects_unknown_selector() {
        let mut a = update_args("CachyOS");
        a.edits.set_disk = vec!["match=vdz,size=80g".to_owned()];
        let err = update(&existing(), &a).unwrap_err().to_string();
        assert!(err.contains("no disk with dev 'vdz'"), "{err}");
    }

    #[test]
    fn update_rejects_removing_something_absent() {
        let mut a = update_args("CachyOS");
        a.edits.rm_nic = vec!["vnet-nope".to_owned()];
        let err = update(&existing(), &a).unwrap_err().to_string();
        assert!(err.contains("no nic matching 'vnet-nope'"), "{err}");
    }

    #[test]
    fn update_requires_a_selector_on_set() {
        let mut a = update_args("CachyOS");
        a.edits.set_disk = vec!["size=80g".to_owned()];
        let err = update(&existing(), &a).unwrap_err().to_string();
        assert!(err.contains("needs a match= selector"), "{err}");
    }

    #[test]
    fn update_rejects_removing_the_last_disk() {
        let mut a = update_args("CachyOS");
        a.edits.rm_disk = vec!["vda".to_owned()];
        let err = update(&existing(), &a).unwrap_err().to_string();
        assert!(err.contains("at least one disk or ISO"), "{err}");
    }

    #[test]
    fn update_from_spec_file_keeps_the_target_uuid() {
        let mut other = build(&args("Other")).unwrap();
        other.virtual_machine_name = "ffffffff-0000-0000-0000-000000000009".to_owned();
        let file = std::env::temp_dir().join("ugos-vmspec-update-test.json");
        std::fs::write(&file, serde_json::to_string(&other).unwrap()).unwrap();

        let current = existing();
        let mut a = update_args("CachyOS");
        a.spec_file = Some(file.to_string_lossy().into_owned());
        let spec = update(&current, &a).unwrap();

        assert_eq!(spec.virtual_machine_name, current.virtual_machine_name);
        assert_eq!(spec.virtual_machine_display_name, "CachyOS");

        std::fs::remove_file(&file).unwrap();
    }

    #[test]
    fn update_does_not_inject_create_defaults() {
        let mut current = existing();
        current.device.graphics_card = "cirrus".to_owned();
        current.other_config.keyboard_language = "fr".to_owned();
        current.device.usb_controller = 0;
        let spec = update(&current, &update_args("CachyOS")).unwrap();
        assert_eq!(spec.device.graphics_card, "cirrus");
        assert_eq!(spec.other_config.keyboard_language, "fr");
        assert_eq!(spec.device.usb_controller, 0);
    }
}
