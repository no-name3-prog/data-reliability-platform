//! Storage layer. Built and tested only inside Docker.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod memory;
mod postgres;
mod traits;

pub use memory::MemoryStore;
pub use postgres::PostgresStore;
pub use traits::Store;

use std::sync::Arc;

use drp_common::{AppConfig, Error, Result};

/// Construct a store from application config (async for postgres).
pub async fn open_store(cfg: &AppConfig) -> Result<Arc<dyn Store>> {
    match cfg.storage.backend.as_str() {
        "memory" => Ok(Arc::new(MemoryStore::new())),
        "postgres" => {
            let store =
                PostgresStore::connect(&cfg.storage.database_url, cfg.storage.max_connections)
                    .await?;
            Ok(Arc::new(store))
        }
        other => Err(Error::config(format!(
            "unknown storage backend '{other}' (supported: memory, postgres)"
        ))),
    }
}

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
