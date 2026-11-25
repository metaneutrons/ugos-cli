//! Bounded, in-memory handling of UGOS certificate export archives.

use std::io::{Cursor, Read, Seek};

use zip::ZipArchive;

use crate::UgosClient;
use crate::error::{Result, UgosError};

const MAX_ARCHIVE_BYTES: usize = 16 * 1024 * 1024;
const MAX_MEMBER_BYTES: usize = 8 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 128;

/// Export a certificate and its key for a safe renewal or rollback.
pub(super) async fn read(client: &UgosClient, subject: &str) -> Result<(Vec<u8>, Vec<u8>)> {
    let bytes = download(client, subject).await?;
    backup(&bytes)
}

/// Export only the selected leaf certificate, without decompressing its key.
pub(super) async fn leaf(client: &UgosClient, subject: &str) -> Result<Vec<u8>> {
    let bytes = download(client, subject).await?;
    let mut archive = open(&bytes)?;
    let certificate = read_member(&mut archive, "cert.pem")?;
    if certificate.is_empty() {
        return Err(invalid_export());
    }
    Ok(certificate)
}

async fn download(client: &UgosClient, subject: &str) -> Result<Vec<u8>> {
    let mut response = client
        .get_bytes("security/ssl/export", &[("cert", subject)])
        .await
        .map_err(|_| invalid_export())?;
    if response
        .content_length()
        .is_some_and(|n| n > MAX_ARCHIVE_BYTES as u64)
    {
        return Err(invalid_export());
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| invalid_export())? {
        if chunk.len() > MAX_ARCHIVE_BYTES - bytes.len() {
            return Err(invalid_export());
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        return Err(invalid_export());
    }
    Ok(bytes)
}

fn backup(bytes: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut archive = open(bytes)?;
    if archive
        .file_names_raw()
        .any(|name| !matches!(name, b"cert.pem" | b"privkey.pem" | b"chain.pem"))
    {
        return Err(invalid_export());
    }
    let has_chain = archive.index_for_name("chain.pem").is_some();
    let mut certificate = read_member(&mut archive, "cert.pem")?;
    if certificate.is_empty() {
        return Err(invalid_export());
    }
    if has_chain {
        let chain = read_member(&mut archive, "chain.pem")?;
        if !chain.is_empty() {
            if !certificate.ends_with(b"\n") {
                certificate.push(b'\n');
            }
            certificate.extend_from_slice(&chain);
        }
    }
    let key = read_member(&mut archive, "privkey.pem")?;
    if key.is_empty() {
        return Err(invalid_export());
    }
    Ok((certificate, key))
}

fn open(bytes: &[u8]) -> Result<ZipArchive<Cursor<&[u8]>>> {
    let archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| invalid_export())?;
    if archive.is_empty() || archive.len() > MAX_ARCHIVE_ENTRIES || archive.has_duplicate_names() {
        return Err(invalid_export());
    }
    Ok(archive)
}

fn read_member<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Result<Vec<u8>> {
    let file = archive.by_name(name).map_err(|_| invalid_export())?;
    let expected = file.size();
    if expected > MAX_MEMBER_BYTES as u64 {
        return Err(invalid_export());
    }
    let mut contents = Vec::with_capacity(usize::try_from(expected).map_err(|_| invalid_export())?);
    let _ = file
        .take(MAX_MEMBER_BYTES as u64 + 1)
        .read_to_end(&mut contents)
        .map_err(|_| invalid_export())?;
    if contents.len() as u64 != expected {
        return Err(invalid_export());
    }
    Ok(contents)
}

fn invalid_export() -> UgosError {
    UgosError::Refused("UGOS returned an invalid or oversized certificate export".into())
}

#[cfg(test)]
mod tests {
    use super::open;

    #[test]
    fn rejects_malformed_zip() {
        assert!(open(b"not a zip archive").is_err());
    }
}
