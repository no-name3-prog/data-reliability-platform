//! AI / LLM provider plugins and validation rule suggestions.
//!
//! # Pluggable providers
//!
//! Implementations of [`drp_core::AiProviderPlugin`]:
//! - **`heuristic`** — offline rule suggestions from schema + profile + samples
//! - **`echo`** — stub completions for tests
//! - **`openai_compatible`** — SpaceXAI/xAI, Ollama, or any OpenAI-compatible API
//!
//! Register providers via [`register_ai_providers_with_config`]. Third-party
//! crates can implement the trait and register at the composition root.
//!
//! # Rule suggestions
//!
//! [`RuleSuggestionService`] proposes validation rules that stay **pending**
//! until a human approves (activates a check) or rejects them.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

mod echo;
mod heuristic;
mod openai_compatible;
mod registry;
mod service;
mod suggest;

pub use echo::{simple_user_request, EchoAiProvider};
pub use heuristic::HeuristicAiProvider;
pub use openai_compatible::OpenAiCompatibleProvider;
pub use registry::{register_ai_providers_with_config, register_builtin_ai_providers};
pub use service::AiService;
pub use suggest::{ApproveResult, RuleSuggestionService};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
