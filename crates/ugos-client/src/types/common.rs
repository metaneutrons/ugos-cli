//! Common types shared across all UGOS API modules.

use serde::Deserialize;

use crate::error::{Result, UgosError};

/// Outer envelope for every UGOS API response.
///
/// ```json
/// {"code": 200, "msg": "success", "data": <T>, "time": 0.006}
/// ```
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    /// UGOS status code (200 = success).
    pub code: i32,
    /// Human-readable status message.
    pub msg: String,
    /// The payload — shape varies per endpoint.
    pub data: T,
}

impl<T> ApiResponse<T> {
    /// Check the response code and convert API errors into [`UgosError`].
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] variant when `code` is not 200.
    pub fn into_result(self) -> Result<T> {
        match self.code {
            200 => Ok(self.data),
            1003 => Err(UgosError::AuthFailed),
            1005 => Err(UgosError::ParameterError(self.msg)),
            1024 => Err(UgosError::LoginExpired),
            3004 => Err(UgosError::OperationFailed(self.msg)),
            9404 => Err(UgosError::AppNotFound(self.msg)),
            9405 => Err(UgosError::AppServiceError(self.msg)),
            code => Err(UgosError::Api {
                code,
                msg: self.msg,
            }),
        }
    }
}

/// Wrapper for endpoints returning `{result: T}` inside `data`.
#[derive(Debug, Deserialize)]
pub struct ResultWrapper<T> {
    /// The inner result value.
    pub result: T,
}
