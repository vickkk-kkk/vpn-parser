//! Portable config surface for the fork, not the side-effectful desktop App.
//! Upstream theme tests refer to desktop AppSettings, so unit-test builds cover
//! the isolated editor model only. UI compile/render validation is separate.
#[cfg(not(test))]
mod app;
mod editor;
#[cfg(not(test))]
#[path = "../../../src/gui/theme.rs"]
#[allow(dead_code)]
mod theme;
#[cfg(not(test))]
pub use app::MobileApp;
