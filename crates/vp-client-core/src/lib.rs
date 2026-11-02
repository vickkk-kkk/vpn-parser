//! Shared client business logic without platform lifecycle or implicit I/O.
//!
//! This initial extraction contains configuration models and pure rule helpers.
//! It does not implement a VPN backend, fetch engines, or alter host networking.

pub mod config;

pub mod models;
pub mod parser;

pub mod geo;
pub mod rule_io;
