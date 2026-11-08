pub mod config;
mod format_text;
mod generation;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod plugin;

pub use config::{Configuration, resolve_config};
pub use format_text::format_text;
