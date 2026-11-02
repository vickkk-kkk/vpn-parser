//! Portable config surface for the fork, not the side-effectful desktop App.
//! Upstream theme tests refer to desktop AppSettings, so unit-test builds cover
//! the isolated editor model only. UI compile/render validation is separate.
#[cfg(all(feature = "rendering", not(test)))]
mod app;
mod editor;
#[cfg(all(feature = "rendering", not(test)))]
mod fonts;
#[cfg(all(feature = "rendering", not(test)))]
#[path = "../../../src/gui/theme.rs"]
#[allow(dead_code)]
mod theme;
#[cfg(all(feature = "rendering", not(test)))]
pub use app::MobileApp;

/// Pure paste/preview/confirm model; available without rendering dependencies.
pub mod import;

/// Pure checked configuration-preview state; never starts an engine.
pub mod preview;
