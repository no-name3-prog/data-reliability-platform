//! Storage layer. Built and tested only inside Docker.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod memory;
mod traits;

pub use memory::MemoryStore;
pub use traits::Store;

use std::sync::Arc;

use drp_common::{AppConfig, Error, Result};

/// Construct a store from application config.
pub fn open_store(cfg: &AppConfig) -> Result<Arc<dyn Store>> {
    match cfg.storage.backend.as_str() {
        "memory" => Ok(Arc::new(MemoryStore::new())),
        other => Err(Error::config(format!(
            "unknown storage backend '{other}' (supported: memory; postgres reserved for future)"
        ))),
    }
}

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
