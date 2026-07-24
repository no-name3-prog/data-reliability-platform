//! Time helpers. All platform timestamps are UTC.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// UTC timestamp newtype for consistent serde and display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UtcTimestamp(DateTime<Utc>);

impl UtcTimestamp {
    /// Current time in UTC.
    pub fn now() -> Self {
        Self(Utc::now())
    }
    /// Wrap an existing `DateTime<Utc>`.
    pub fn from_datetime(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
    /// Access the inner chrono value.
    pub fn inner(self) -> DateTime<Utc> {
        self.0
    }
    /// RFC 3339 string.
    pub fn to_rfc3339(&self) -> String {
        self.0.to_rfc3339()
    }
}

impl Default for UtcTimestamp {
    fn default() -> Self {
        Self::now()
    }
}

impl From<DateTime<Utc>> for UtcTimestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
}

impl std::fmt::Display for UtcTimestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_rfc3339())
    }
}
