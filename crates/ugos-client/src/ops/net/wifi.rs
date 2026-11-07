//! Wi-Fi: connecting, switching, editing and forgetting networks.

use std::time::Duration;

use serde_json::{Value, json};

use crate::api::net::{NetWifiApi, WifiAction};
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::net::{WifiNetwork, WifiResult};

/// Authentication strings of the web UI's form for a network that is not in
/// the scan, with the names the CLI uses for them.
pub const AUTHENTICATIONS: [(&str, &str); 6] = [
    ("wpa", "[WPA-PSK-TKIP]"),
    ("wpa2", "[WPA2-PSK+PSK-SHA256-TKIP+CCMP]"),
    ("wpa-wpa2", "[WPA-PSK-TKIP][WPA2-PSK+PSK-SHA256-TKIP+CCMP]"),
    ("wpa3", "[WPA2-SAE-CCMP]"),
    (
        "wpa2-wpa3",
        "[WPA2-PSK+PSK-SHA256-TKIP+CCMP][WPA2-SAE-CCMP]",
    ),
    ("none", "NONE"),
];

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// A saved network by name.
async fn saved(client: &UgosClient, iface: &str, ssid: &str) -> Result<WifiNetwork> {
    let list = client.net_wifi_list(iface, false).await?;
    list.saved_list
        .into_iter()
        .find(|n| n.ssid == ssid)
        .ok_or_else(|| refused(format!("no saved network '{ssid}'")))
}

/// The body that connects to a network the scan found.
fn scanned_body(iface: &str, found: &WifiNetwork, password: Option<&str>) -> Result<Value> {
    if found.encryption_key == "off" {
        let mut body = serde_json::to_value(found)?;
        if let Some(map) = body.as_object_mut() {
            let _ = map.insert("interface".into(), json!(iface));
            let _ = map.insert("auto_add".into(), json!(1));
            let _ = map.insert("save_net".into(), json!(1));
        }
        return Ok(body);
    }
    let password = password.ok_or_else(|| refused("this network needs a password"))?;
    if password.chars().count() < 8 {
        return Err(refused("a Wi-Fi password is at least 8 characters"));
    }
    Ok(json!({
        "interface": iface, "ssid": found.ssid, "bssid": found.bssid,
        "channel": found.channel, "signal": found.signal,
        "authentication": found.authentication, "password": password,
        "auto_add": 1, "save_net": 1, "status": 0,
    }))
}

/// The body that adds a network by hand, as for a hidden one.
fn manual_body(iface: &str, ssid: &str, password: Option<&str>, auth: &str) -> Result<Value> {
    if ssid.is_empty() || ssid.chars().count() > 64 {
        return Err(refused("a network name is 1 to 64 characters"));
    }
    let authentication = AUTHENTICATIONS
        .iter()
        .find(|(name, _)| *name == auth)
        .map(|(_, value)| *value)
        .ok_or_else(|| refused(format!("the authentication '{auth}' is unknown")))?;
    let open = authentication == "NONE";
    if !open && password.is_none_or(|p| p.chars().count() < 8) {
        return Err(refused("a Wi-Fi password is at least 8 characters"));
    }
    Ok(json!({
        "interface": iface, "ssid": ssid, "authentication": authentication,
        "encryption_key": if open { "off" } else { "on" },
        "password": password.unwrap_or_default(),
        "auto_add": 1, "save_net": 1, "status": 0, "bssid": "", "channel": 0, "signal": 0,
    }))
}

/// Connect to a network. One that is saved is switched to; one the scan finds
/// is connected with its scan data; any other is added by hand with `auth`.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for input the web UI would refuse, and with
/// UGOS's reason when the connection fails.
pub async fn connect(
    client: &UgosClient,
    iface: &str,
    ssid: &str,
    password: Option<&str>,
    auth: Option<&str>,
) -> Result<WifiResult> {
    let list = client.net_wifi_list(iface, true).await?;
    if let Some(known) = list.saved_list.iter().find(|n| n.ssid == ssid) {
        let body = json!({ "interface": iface, "ssid": ssid, "bssid": known.bssid });
        client.net_wifi_post(WifiAction::Switch, &body).await?;
        return wait(client, iface, ssid).await;
    }
    let body = match list.ssid_list.iter().find(|n| n.ssid == ssid) {
        Some(found) => scanned_body(iface, found, password)?,
        None => manual_body(iface, ssid, password, auth.unwrap_or("wpa2-wpa3"))?,
    };
    let first = client.net_wifi_connect(&body).await?;
    if first.status == 0 {
        return Ok(first);
    }
    Err(refused(failure_text(first.status)))
}

/// Say what a connection status means.
#[must_use]
pub fn failure_text(status: i64) -> String {
    match status {
        0 => "connected".into(),
        1 => "connected, but no address was obtained".into(),
        2 => "still connecting".into(),
        3 => "the network was not found".into(),
        5 => "check the router settings".into(),
        7 => "the password is wrong".into(),
        other => format!("the connection failed (status {other})"),
    }
}

async fn wait(client: &UgosClient, iface: &str, ssid: &str) -> Result<WifiResult> {
    for _ in 0..13 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let status = client.net_wifi_status(iface).await?.map(|now| now.status);
        match status {
            Some(0) => {
                return Ok(WifiResult {
                    status: 0,
                    ssid: ssid.into(),
                });
            }
            Some(2) | None => {}
            Some(other) => return Err(refused(failure_text(other))),
        }
    }
    Err(refused("the connection did not come up within 25 seconds"))
}

/// Forget a saved network.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] when the network is not saved.
pub async fn forget(client: &UgosClient, iface: &str, ssid: &str) -> Result<()> {
    let known = saved(client, iface, ssid).await?;
    let body = json!({ "interface": iface, "ssid": ssid, "bssid": known.bssid });
    client.net_wifi_post(WifiAction::Forget, &body).await
}

/// Change whether a saved network joins by itself and whether it stays saved.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] when the network is not saved.
pub async fn edit(
    client: &UgosClient,
    iface: &str,
    ssid: &str,
    auto_join: Option<bool>,
    keep: Option<bool>,
) -> Result<()> {
    let known = saved(client, iface, ssid).await?;
    let body = json!({
        "interface": iface, "ssid": ssid, "bssid": known.bssid,
        "authentication": known.authentication,
        "save_net": keep.map_or(known.save_net, i64::from),
        "auto_add": auto_join.map_or(known.auto_add, i64::from),
    });
    client.net_wifi_post(WifiAction::Edit, &body).await
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn found(open: bool) -> WifiNetwork {
        WifiNetwork {
            ssid: "home".into(),
            bssid: "00:00:5e:00:53:10".into(),
            encryption_key: if open { "off" } else { "on" }.into(),
            authentication: "[WPA2-SAE-CCMP]".into(),
            channel: 6,
            signal: -50,
            ..WifiNetwork::default()
        }
    }

    #[test]
    fn a_scanned_secured_network_needs_a_password_of_eight() {
        assert!(scanned_body("wlan0", &found(false), None).is_err());
        assert!(scanned_body("wlan0", &found(false), Some("short")).is_err());
        let body = scanned_body("wlan0", &found(false), Some("longenough")).unwrap();
        assert_eq!(body["authentication"], "[WPA2-SAE-CCMP]");
        assert_eq!(body["auto_add"], 1);
        assert_eq!(body["status"], 0);
        assert_eq!(body["channel"], 6);
    }

    #[test]
    fn an_open_network_sends_the_scan_item_with_the_flags() {
        let body = scanned_body("wlan0", &found(true), None).unwrap();
        assert_eq!(body["interface"], "wlan0");
        assert_eq!(body["encryption_key"], "off");
        assert_eq!(body["save_net"], 1);
        assert!(body.get("password").is_none());
    }

    #[test]
    fn a_hand_added_network_names_its_authentication() {
        let body = manual_body("wlan0", "hidden", Some("longenough"), "wpa3").unwrap();
        assert_eq!(body["authentication"], "[WPA2-SAE-CCMP]");
        assert_eq!(body["encryption_key"], "on");
        assert_eq!(body["bssid"], "");
        let open = manual_body("wlan0", "cafe", None, "none").unwrap();
        assert_eq!(open["encryption_key"], "off");
        assert!(manual_body("wlan0", "x", Some("longenough"), "wep").is_err());
        assert!(manual_body("wlan0", "", Some("longenough"), "wpa2").is_err());
        assert!(manual_body("wlan0", "x", Some("short"), "wpa2").is_err());
    }

    #[test]
    fn statuses_have_words() {
        assert_eq!(failure_text(7), "the password is wrong");
        assert!(failure_text(99).contains("99"));
    }
}
