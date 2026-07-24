//! AI / LLM provider plugins.
//!
//! Built-in providers are offline-safe (mock / echo). Remote providers (OpenAI,
//! SpaceXAI, etc.) should be separate crates implementing
//! [`drp_core::AiProviderPlugin`].

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod echo;
mod registry;
mod service;

pub use echo::{simple_user_request, EchoAiProvider};
pub use registry::register_builtin_ai_providers;
pub use service::AiService;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
