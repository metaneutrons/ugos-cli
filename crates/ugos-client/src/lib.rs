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
//! let client = UgosClient::connect("192.168.2.5", 9443, creds).await?;
//! let vms = client.vm_list().await?;
//! # Ok(())
//! # }
//! ```

pub mod api;
pub mod auth;
pub mod client;
pub mod error;
pub mod types;

// Re-export the most commonly used items at the crate root.
pub use auth::{Credentials, Session};
pub use client::UgosClient;
