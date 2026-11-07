//! Guarded operations on the network settings of the NAS.
//!
//! The pieces: `validate` has the input rules of the web UI's forms, and the
//! rest build the requests the UI builds, ask before a change that could cut
//! this session off, and read the result back.

pub mod flow;
pub mod general;
pub mod iface;
pub mod links;
pub mod validate;
pub mod wifi;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod validate_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod iface_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod flow_tests;
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod general_tests;
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod links_tests;
