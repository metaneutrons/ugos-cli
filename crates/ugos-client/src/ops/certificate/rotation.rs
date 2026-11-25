//! Switch the `WebServer` assignment through an explicitly selected standby.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use serde::Deserialize;

use super::{
    Bundle, CertificateInfo, archive, custom_subject, find, install, mutation_lock, pem, read_pem,
    user_slot,
};
use crate::error::{Result, UgosError};
use crate::tls::{known_hosts, probe_fingerprint};
use crate::{CertFingerprint, TlsPolicy, UgosClient};

#[derive(Deserialize)]
#[serde(untagged)]
enum ServiceValue {
    One(String),
    Array(Vec<String>),
}

async fn settings(client: &UgosClient) -> Result<BTreeMap<String, String>> {
    let values: BTreeMap<String, ServiceValue> = client.get("security/ssl/getsettings").await?;
    let mut fields = BTreeMap::new();
    for (name, value) in values {
        if name == "ug.link" {
            continue;
        } // Omitted by the Control Panel on save.
        let single = match value {
            ServiceValue::One(s) => s,
            ServiceValue::Array(v) if v.len() <= 1 => v.into_iter().next().unwrap_or_default(),
            ServiceValue::Array(_) => refuse!("ambiguous certificate assignment for {name}"),
        };
        let _ = fields.insert(name, single);
    }
    if !["webserver", "ftps", "webdav"]
        .iter()
        .all(|&k| fields.contains_key(k))
    {
        refuse!("incomplete UGOS service certificate settings");
    }
    Ok(fields)
}

struct Endpoint<'a> {
    host: &'a str,
    port: u16,
}

impl Endpoint<'_> {
    async fn switch(
        &self,
        client: &UgosClient,
        from: &str,
        to: &str,
        fingerprint: CertFingerprint,
    ) -> Result<UgosClient> {
        let mut selection = settings(client).await?;
        if selection["webserver"] != from {
            refuse!("WebServer assignment changed before switching to {to}");
        }
        let _ = selection.insert("webserver".into(), to.into());
        // A reload can close the connection before UGOS returns its acknowledgement.
        let result: Result<serde_json::Value> =
            client.post("security/ssl/settings", &selection).await;
        let confirm = async {
            loop {
                if probe_fingerprint(self.host, self.port)
                    .await
                    .is_ok_and(|seen| seen == fingerprint)
                {
                    let next = client.repin(fingerprint)?;
                    if settings(&next)
                        .await
                        .is_ok_and(|fields| fields["webserver"] == to)
                    {
                        known_hosts::put(self.host, self.port, &fingerprint)?;
                        return Ok(next);
                    }
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        };
        if let Ok(verified) = tokio::time::timeout(Duration::from_secs(30), confirm).await {
            return verified;
        }
        let _ = result?;
        refuse!(
            "UGOS did not confirm the expected HTTPS certificate for {to} within 30 seconds; check the NAS and rerun"
        );
    }
}

/// Renew the UGOS HTTPS certificate through an explicitly selected standby.
///
/// Stores each new pin after the endpoint proves possession of the expected key.
///
/// # Errors
/// Refuses unknown peers, built-in replacement targets, shared target
/// certificates, and unexpected assignment changes. A failed renewal can
/// leave the verified standby serving until the command is rerun.
pub async fn rotate(
    client: &UgosClient,
    cert: &Path,
    key: &Path,
    fallback_cert: &Path,
) -> Result<CertificateInfo> {
    let _lock = mutation_lock(client)?;
    let new = Bundle::load(cert, key)?;
    let target = &new.chain.subject;
    custom_subject(target)?;
    let standby_pem = read_pem(fallback_cert)?;
    pem::require_current(&new.chain.leaf_pem)?;
    pem::require_current(&standby_pem)?;
    let (standby, standby_pin) = pem::identity(&standby_pem)?;
    if target == &standby {
        refuse!("standby and replacement must have different subjects");
    }
    let (host, port, _) = client.https_target();
    let endpoint = Endpoint { host, port };
    let (web, served) = verify_assignments(client, target, &standby, standby_pin).await?;
    let web = web.as_str();
    let mut connection = client.clone();
    if let Some(existing) = find(client, target).await? {
        if matches_existing(client, &new, &existing, web, served).await? {
            if web != target {
                connection = endpoint
                    .switch(client, web, target, new.chain.fingerprint)
                    .await?;
            }
            return confirmed_web(&connection, &new).await;
        }
        if web == target {
            connection = endpoint
                .switch(client, target, &standby, standby_pin)
                .await?;
        }
    } else if web == target {
        refuse!("WebServer points to a missing certificate");
    }
    let _ = install(&connection, &new, true).await?;
    let installed = Bundle::exported(&connection, target).await?;
    if !new.same(&installed) {
        refuse!(
            "uploaded certificate chain differs from the local files; standby remains assigned"
        );
    }
    connection = endpoint
        .switch(&connection, &standby, target, new.chain.fingerprint)
        .await?;
    confirmed_web(&connection, &new).await
}

async fn matches_existing(
    client: &UgosClient,
    new: &Bundle,
    existing: &CertificateInfo,
    web: &str,
    served: CertFingerprint,
) -> Result<bool> {
    user_slot(existing)?;
    let target = &existing.subject;
    if existing.be_used.iter().any(|s| s != "webserver") {
        refuse!("other services use {target}; this rotation manages WebServer only");
    }
    let old = Bundle::exported(client, target).await?;
    if web == target && old.chain.fingerprint != served {
        refuse!("the live HTTPS certificate differs from the UGOS web assignment");
    }
    Ok(new.same(&old))
}

async fn verify_assignments(
    client: &UgosClient,
    target: &str,
    standby: &str,
    standby_pin: CertFingerprint,
) -> Result<(String, CertFingerprint)> {
    let (host, port, policy) = client.https_target();
    let served = probe_fingerprint(host, port).await?;
    // Every request uses the authenticated session's original policy until a
    // switch has been verified. Insecure connections cannot authorize a rotation.
    if !matches!(policy, TlsPolicy::Pinned(pin) if *pin == served) {
        refuse!("rotation requires a direct, pinned connection to the UGOS web service");
    }
    let current = settings(client).await?;
    let web = current["webserver"].as_str();
    if web != target && web != standby {
        refuse!(
            "WebServer is assigned to {web}; expected {target} or the selected standby {standby}"
        );
    }
    if web == standby && served != standby_pin {
        refuse!("the supplied standby certificate differs from the one served by the NAS");
    }
    if find(client, standby).await?.is_none() {
        refuse!("standby certificate {standby} is missing from the NAS");
    }
    if web != standby && pem::identity(&archive::leaf(client, standby).await?)?.1 != standby_pin {
        refuse!("the installed standby differs from the supplied public certificate");
    }

    if known_hosts::get(host, port)? != Some(served) {
        known_hosts::put(host, port, &served)?;
    }
    Ok((web.to_owned(), served))
}

async fn confirmed_web(client: &UgosClient, new: &Bundle) -> Result<CertificateInfo> {
    find(client, &new.chain.subject)
        .await?
        .filter(|c| c.expired == new.chain.expires && c.be_used.iter().any(|s| s == "webserver"))
        .ok_or_else(|| {
            UgosError::Refused("the NAS did not confirm the renewed WebServer certificate".into())
        })
}
