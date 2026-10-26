#![forbid(unsafe_code)]
pub use shot_core::platform::*;

#[cfg(target_os = "macos")]
pub mod macos;
