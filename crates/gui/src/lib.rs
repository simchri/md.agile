pub mod card_positioning;
// See the matching declaration (and its doc comment) in `main.rs` for why
// this is excluded from wasm32 builds.
#[cfg(not(target_arch = "wasm32"))]
pub mod lock;
pub mod physics;
pub mod server;
#[cfg(not(target_arch = "wasm32"))]
pub mod settings;
pub mod slots;
