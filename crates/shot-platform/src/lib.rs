#![forbid(unsafe_code)]
// Native platform adapters remain gated on a real macOS 14+ environment.
pub use shot_core::platform::*;
