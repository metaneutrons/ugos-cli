//! Certificate import and public inventory.

use std::io::Write;

use anyhow::Result;
use ugos_client::UgosClient;
use ugos_client::ops::certificate::{self, CertificateInfo};

use crate::cli::{CertificateAction, OutputFormat};
use crate::output::{self, CertificateRow};

pub(super) async fn run(
    client: &UgosClient,
    action: &CertificateAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        CertificateAction::List => {
            let certs = certificate::list(client).await?;
            match fmt {
                OutputFormat::Json => output::print_json(w, &certs)?,
                OutputFormat::Table => output::print_list(
                    w,
                    &certs.iter().map(CertificateRow::from).collect::<Vec<_>>(),
                    fmt,
                )?,
            }
        }
        CertificateAction::Import {
            cert,
            key,
            replace_unused,
        } => {
            let added = certificate::import(client, cert, key, *replace_unused).await?;
            show(w, fmt, &added)?;
        }
        CertificateAction::Rotate {
            cert,
            key,
            fallback_cert,
        } => {
            let active = certificate::rotate(client, cert, key, fallback_cert).await?;
            show(w, fmt, &active)?;
        }
    }
    Ok(())
}

fn show(w: &mut impl Write, fmt: OutputFormat, cert: &CertificateInfo) -> Result<()> {
    match fmt {
        OutputFormat::Json => output::print_json(w, cert)?,
        OutputFormat::Table => output::print_list(w, &[CertificateRow::from(cert)], fmt)?,
    }
    Ok(())
}
