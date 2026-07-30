//! Build script for `mdagile-gui`.
//!
//! Deliberately does *not* shell out to `dx` or `cargo` itself — doing so
//! from within a build script risks deadlocking on cargo's own build lock
//! (a nested `cargo`/`dx` invocation contending for the same `target/`
//! directory) and would also fire on every ordinary `cargo build`/`cargo
//! test`/`dx serve` invocation, not just intentional installs.
//!
//! Instead, when the `embed-assets` feature is enabled, this script only
//! *checks* that a pre-bundled asset directory already exists (produced by
//! `scripts/install-gui.sh` running `dx bundle` as a separate, top-level
//! step) and fails fast with clear instructions if it doesn't. The actual
//! embedding happens at compile time via `rust_embed` in
//! `src/embedded_assets.rs`.

use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=.bundled-assets/public");

    if std::env::var_os("CARGO_FEATURE_EMBED_ASSETS").is_none() {
        return;
    }

    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is always set by cargo");
    let bundled_public = Path::new(&manifest_dir).join(".bundled-assets/public");

    if !bundled_public.join("index.html").is_file() {
        panic!(
            "\n\nmdagile-gui: the `embed-assets` feature requires a pre-bundled \
            `dx bundle` output at:\n  {}\n\n\
            This directory is not created by `cargo build`/`cargo install` \
            directly (to avoid nesting `dx`/`cargo` inside this build script). \
            Instead, run the top-level install script, which bundles the web \
            assets and then builds this crate with `embed-assets` enabled:\n\n  \
            ./scripts/install-gui.sh\n\n",
            bundled_public.display()
        );
    }
}
