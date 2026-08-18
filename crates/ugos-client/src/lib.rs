//! UGOS NAS API client library.
//!
//! Provides typed access to the UGREEN NAS (UGOS) REST API,
//! including authentication (RSA + session tokens) and all
//! service modules (KVM, storage, network, etc.).
//!
//! # Quick start
//!
//! ```rust,no_run
//! use ugos_client::{UgosClient, Credentials};
//! use ugos_client::api::kvm::KvmApi;
//!
//! # async fn example() -> ugos_client::error::Result<()> {
//! let creds = Credentials {
//!     username: "admin".into(),
//!     password: "secret".into(),
//! };
//! let tls = ugos_client::TlsPolicy::Pinned(
//!     ugos_client::tls::probe_fingerprint("192.168.1.10", 9443).await?,
//! );
//! let client = UgosClient::connect("192.168.1.10", 9443, creds, &tls).await?;
//! let vms = client.vm_list().await?;
//! # Ok(())
//! # }
//! ```

pub mod api;
pub mod auth;
pub mod client;
pub mod crypto;
pub mod error;
pub mod tls;
pub mod types;

// Re-export the most commonly used items at the crate root.
pub use auth::{Credentials, Session};
pub use client::UgosClient;
pub use tls::{CertFingerprint, TlsPolicy};

#[cfg(test)]
mod tests;
