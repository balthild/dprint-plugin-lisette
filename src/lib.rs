pub mod config;
mod format_text;
mod generation;

#[cfg(feature = "plugin")]
mod plugin;

pub use config::{Configuration, resolve_config};
pub use format_text::format_text;
