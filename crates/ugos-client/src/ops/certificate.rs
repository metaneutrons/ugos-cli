//! Certificate inventory, import and renewal through the UGOS Control Panel.

mod archive;
mod pem;
mod rotation;

use std::path::Path;

use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};

use crate::error::{Result, UgosError};
use crate::types::common::{ResultWrapper, null_vec_as_empty};
use crate::{CertFingerprint, UgosClient};
pub use rotation::rotate;

/// Public metadata from the UGOS certificate inventory.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateInfo {
    /// Common name used by UGOS as the inventory key.
    pub subject: String,
    /// Issuer displayed in Control Panel.
    pub issuer: String,
    /// Expiry as Unix seconds.
    pub expired: u64,
    /// One for the system default.
    pub is_default: u8,
    /// Services using the certificate.
    #[serde(deserialize_with = "null_vec_as_empty")]
    pub be_used: Vec<String>,
}

/// List available certificates and their assignments.
///
/// # Errors
/// Returns an error if the NAS cannot provide the inventory.
pub async fn list(client: &UgosClient) -> Result<Vec<CertificateInfo>> {
    let response: ResultWrapper<Vec<CertificateInfo>> = client.get("security/ssl/list").await?;
    Ok(response.result)
}

fn read_pem(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|e| UgosError::io(format!("reading {}", path.display()), e))
}

/// Read the public subject and fingerprint of a local certificate.
///
/// # Errors
/// Returns an error for an unreadable or malformed PEM file.
pub fn identity(path: &Path) -> Result<(String, CertFingerprint)> {
    pem::identity(&read_pem(path)?)
}

struct Bundle {
    chain: pem::ValidatedChain,
    key: Vec<u8>,
}

impl Bundle {
    fn new(chain: &[u8], key: Vec<u8>) -> Result<Self> {
        Ok(Self {
            chain: pem::validate(chain, &key)?,
            key,
        })
    }

    fn load(cert: &Path, key: &Path) -> Result<Self> {
        Self::new(&read_pem(cert)?, read_pem(key)?)
    }

    async fn exported(client: &UgosClient, subject: &str) -> Result<Self> {
        let (chain, key) = archive::read(client, subject).await?;
        let bundle = Self::new(&chain, key)?;
        if bundle.chain.subject != subject {
            refuse!("exported certificate has an unexpected subject");
        }
        Ok(bundle)
    }

    fn same(&self, other: &Self) -> bool {
        self.chain.fingerprint == other.chain.fingerprint
            && self.chain.intermediates_pem == other.chain.intermediates_pem
    }

    async fn upload(&self, client: &UgosClient) -> Result<CertificateInfo> {
        let mut form = Form::new()
            .part(
                "file",
                Part::bytes(self.key.clone()).file_name("privkey.pem"),
            )
            .part(
                "file",
                Part::bytes(self.chain.leaf_pem.clone()).file_name("cert.pem"),
            );
        if !self.chain.intermediates_pem.is_empty() {
            form = form.part(
                "file",
                Part::bytes(self.chain.intermediates_pem.clone()).file_name("chain.pem"),
            );
        }
        let _: serde_json::Value = client
            .post_multipart("security/ssl/import?cover=0", form)
            .await
            .map_err(|error| match error {
                UgosError::Api { code, .. } => UgosError::Refused(format!(
                    "UGOS rejected the certificate import (code {code})"
                )),
                UgosError::LoginExpired => error,
                _ => {
                    UgosError::Refused("certificate upload failed; check the NAS inventory".into())
                }
            })?;
        find(client, &self.chain.subject)
            .await?
            .filter(|info| info.expired == self.chain.expires)
            .ok_or_else(|| {
                UgosError::Refused(
                    "import acknowledged but subject or expiry differs on read-back".into(),
                )
            })
    }
}

async fn find(client: &UgosClient, subject: &str) -> Result<Option<CertificateInfo>> {
    Ok(list(client)
        .await?
        .into_iter()
        .find(|c| c.subject == subject))
}

fn user_slot(info: &CertificateInfo) -> Result<()> {
    if info.is_default != 0 {
        refuse!(
            "the default certificate {} cannot be replaced",
            info.subject
        );
    }
    Ok(())
}

fn mutation_lock(client: &UgosClient) -> Result<std::fs::File> {
    let (host, port, _) = client.https_target();
    let id = crate::crypto::sha256_hex(&format!("{}:{port}", host.to_ascii_lowercase()));
    let dir = dirs::config_dir()
        .ok_or_else(|| UgosError::Refused("missing configuration directory".into()))?
        .join("ugos-cli");
    std::fs::create_dir_all(&dir)
        .map_err(|e| UgosError::io("creating certificate lock directory", e))?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join(format!("certificate-{id}.lock")))
        .map_err(|e| UgosError::io("opening certificate lock", e))?;
    file.try_lock().map_err(|e| match e {
        std::fs::TryLockError::WouldBlock => UgosError::Refused(
            "another local certificate change is in progress for this NAS".into(),
        ),
        std::fs::TryLockError::Error(io) => UgosError::io("locking certificate changes", io),
    })?;
    Ok(file)
}

fn custom_subject(subject: &str) -> Result<()> {
    if ["ugreen", "ugreenlink", "ug.link", "uglink"]
        .iter()
        .any(|name| subject.eq_ignore_ascii_case(name))
    {
        refuse!("the certificate name {subject} is reserved by UGOS");
    }
    Ok(())
}

async fn install(client: &UgosClient, new: &Bundle, replace: bool) -> Result<CertificateInfo> {
    let subject = &new.chain.subject;
    custom_subject(subject)?;
    let Some(info) = find(client, subject).await? else {
        return new.upload(client).await;
    };
    user_slot(&info)?;
    if !replace || !info.be_used.is_empty() {
        refuse!(
            "certificate {subject} exists; replacement requires an unassigned custom certificate and --replace-unused"
        );
    }
    let old = Bundle::exported(client, subject).await?;
    if new.same(&old) {
        return Ok(info);
    }
    let current = find(client, subject).await?.ok_or_else(|| {
        UgosError::Refused("the certificate disappeared before replacement".into())
    })?;
    user_slot(&current)?;
    if !current.be_used.is_empty() || current.expired != old.chain.expires {
        refuse!("certificate assignment or expiry changed before replacement");
    }
    let removed: Result<serde_json::Value> = client
        .delete_params("security/ssl/cert/delete", &[("cert", subject)])
        .await;
    if let Some(remaining) = find(client, subject).await? {
        let _ = removed?;
        refuse!("UGOS kept certificate {} after deletion", remaining.subject);
    }
    match new.upload(client).await {
        Ok(info) => Ok(info),
        Err(error) => {
            if let Some(info) = find(client, subject).await? {
                if new.same(&Bundle::exported(client, subject).await?) {
                    return Ok(info);
                }
                refuse!("another certificate occupies {subject} after the upload failure");
            }
            if old.upload(client).await.is_err() {
                refuse!(
                    "replacement and restoration both failed for {subject}; check the NAS inventory"
                );
            }
            refuse!(
                "replacement failed for {subject}; the previous certificate was restored: {error}"
            )
        }
    }
}

/// Import a PEM leaf or full chain and its private key. With `replace_unused`,
/// back up and replace an unassigned custom entry of the same name.
///
/// # Errors
/// Refuses invalid input, protected entries, and unconfirmed writes.
pub async fn import(
    client: &UgosClient,
    cert: &Path,
    key: &Path,
    replace_unused: bool,
) -> Result<CertificateInfo> {
    let _lock = mutation_lock(client)?;
    install(client, &Bundle::load(cert, key)?, replace_unused).await
}
