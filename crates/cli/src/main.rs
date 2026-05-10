//! `agile` binary entry point.
//!
//! All logic lives in [`mdagile::cli`]; this file is just the dispatcher.

fn main() {
    mdagile::cli::logger::init();
    mdagile::cli::run();
}
