//! Unified error type for the platform.

use std::fmt;

/// Convenient result alias.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Platform-wide error classification.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Configuration is missing or invalid.
    #[error("configuration error: {0}")]
    Config(String),
    /// Requested entity was not found.
    #[error("not found: {0}")]
    NotFound(String),
    /// Caller supplied invalid input.
    #[error("validation error: {0}")]
    Validation(String),
    /// Operation is not supported in the current context.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// Plugin-related failure (missing, load, invoke).
    #[error("plugin error: {0}")]
    Plugin(String),
    /// Connector / data-source failure.
    #[error("connector error: {0}")]
    Connector(String),
    /// Persistence failure.
    #[error("storage error: {0}")]
    Storage(String),
    /// Scheduler / job failure.
    #[error("scheduler error: {0}")]
    Scheduler(String),
    /// Notification delivery failure.
    #[error("notification error: {0}")]
    Notification(String),
    /// Downstream dependency timed out.
    #[error("timeout: {0}")]
    Timeout(String),
    /// Conflict with existing state.
    #[error("conflict: {0}")]
    Conflict(String),
    /// Internal invariant violated.
    #[error("internal error: {0}")]
    Internal(String),
    /// Wrap opaque third-party errors.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Error {
    /// Create a configuration error.
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }
    /// Create a not-found error.
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }
    /// Create a validation error.
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }
    /// Create a plugin error.
    pub fn plugin(msg: impl Into<String>) -> Self {
        Self::Plugin(msg.into())
    }
    /// Create a storage error.
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::Storage(msg.into())
    }
    /// Create a scheduler error.
    pub fn scheduler(msg: impl Into<String>) -> Self {
        Self::Scheduler(msg.into())
    }
    /// Create a connector error.
    pub fn connector(msg: impl Into<String>) -> Self {
        Self::Connector(msg.into())
    }
    /// Create a notification error.
    pub fn notification(msg: impl Into<String>) -> Self {
        Self::Notification(msg.into())
    }
    /// Create an internal error.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Stable machine-readable error code for APIs.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Config(_) => "config_error",
            Self::NotFound(_) => "not_found",
            Self::Validation(_) => "validation_error",
            Self::Unsupported(_) => "unsupported",
            Self::Plugin(_) => "plugin_error",
            Self::Connector(_) => "connector_error",
            Self::Storage(_) => "storage_error",
            Self::Scheduler(_) => "scheduler_error",
            Self::Notification(_) => "notification_error",
            Self::Timeout(_) => "timeout",
            Self::Conflict(_) => "conflict",
            Self::Internal(_) | Self::Other(_) => "internal_error",
        }
    }

    /// HTTP-ish status class for API mapping.
    pub fn status_hint(&self) -> u16 {
        match self {
            Self::NotFound(_) => 404,
            Self::Validation(_) | Self::Config(_) => 400,
            Self::Unsupported(_) => 501,
            Self::Conflict(_) => 409,
            Self::Timeout(_) => 504,
            _ => 500,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

/// Extension trait for attaching context to results.
pub trait ResultExt<T> {
    /// Map any error into [`Error::Internal`] with a message prefix.
    fn internal_ctx(self, ctx: impl fmt::Display) -> Result<T>;
}

impl<T, E: fmt::Display> ResultExt<T> for std::result::Result<T, E> {
    fn internal_ctx(self, ctx: impl fmt::Display) -> Result<T> {
        self.map_err(|e| Error::internal(format!("{ctx}: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(Error::not_found("x").code(), "not_found");
        assert_eq!(Error::validation("x").status_hint(), 400);
    }
}
