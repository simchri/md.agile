//! Library crate root: re-exports all public modules.
//!
//! Both binaries (`agile` and `agilels`) and the integration tests in
//! `tests/` consume the project through this crate.

pub mod checker;
pub mod cli;
pub mod formatter;
pub mod lsp;
pub mod parser;
pub mod rules;
