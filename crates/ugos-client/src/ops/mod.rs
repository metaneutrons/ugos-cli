//! Operations that check their input and read the result back.
//!
//! The endpoint wrappers in [`crate::api`] send what they are given. UGOS
//! often answers success and changes nothing, or deletes more than the caller
//! thought, so `ugos-cli` and `ugos-mcp` both go through these instead: the
//! same checks, the same read-back, the same refusals.

/// Return a refusal with a formatted message.
macro_rules! refuse {
    ($($arg:tt)*) => {
        return Err($crate::error::UgosError::Refused(format!($($arg)*)))
    };
}

pub mod fileserver;
pub mod hw;
pub mod net;
pub mod shares;
pub mod users;
