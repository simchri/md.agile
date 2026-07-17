//! Library crate root: re-exports all public modules.
//!
//! Both binaries (`agile` and `agilels`) and the integration tests in
//! `tests/` consume the project through this crate.

pub mod checker;
pub mod cli;
pub mod config;
pub mod eta;
pub mod formatter;
pub mod git;
pub mod history_cache;
pub mod lsp;
pub mod parser;
pub mod rules;
