//! Serves a `dx bundle` web output that has been baked into the binary at
//! compile time, so the resulting server executable is fully self-contained
//! and does not depend on a sibling `public/` directory at runtime (contrast
//! with dioxus-server's default `serve_static_assets()`, which looks up
//! `current_exe().parent().join("public")`).
//!
//! The embedded directory (`.bundled-assets/public`) is produced ahead of
//! time by `scripts/install-gui.sh` running `dx bundle`; see `build.rs` for
//! the fail-fast check that this directory exists before compiling.

use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/.bundled-assets/public"]
struct BundledAssets;

/// Adds a route for every embedded asset file (everything `dx bundle`
/// produced under `public/`, e.g. `wasm/*`, `assets/*`, `style.css`) to
/// `router`, serving the embedded bytes with a guessed MIME type.
///
/// `index.html` is intentionally skipped: the root route (`/`) is served by
/// Dioxus's own server-side render fallback, not the static bundled file.
pub fn merge_embedded_assets(mut router: Router<()>) -> Router<()> {
    for path in BundledAssets::iter() {
        if path.as_ref() == "index.html" {
            continue;
        }

        let route_path = format!("/{path}");
        let owned_path = path.to_string();
        router = router.route(
            &route_path,
            get(move || serve_embedded_asset(owned_path.clone())),
        );
    }

    router
}

async fn serve_embedded_asset(path: String) -> Response {
    match BundledAssets::get(&path) {
        Some(file) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            (
                [(header::CONTENT_TYPE, mime.essence_str().to_string())],
                file.data.into_owned(),
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
