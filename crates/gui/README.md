# mdagile GUI

Dioxus web application for the mdagile task manager.

## Development

### Running with hot-reload

Start the development server with enhanced hot-reload features:

```bash
cd crates/gui
dx serve --platform web --hot-patch --interactive
```

This enables:
- **Hot patching** (`--hot-patch`): Rust code changes are patched without full rebuild (experimental)
- **Interactive mode** (`--interactive`): CLI commands available during development
- **File watching**: Automatic rebuild on file changes (enabled by default)
- **Full hot-reload**: UI updates reflect immediately in the browser

The server runs on `http://localhost:8080/`

### Without hot-patch (faster, more stable)

If you prefer stable hot-reload without hot-patching:

```bash
dx serve --platform web --interactive
```

## Project Structure

- `src/main.rs` — Dioxus component and layout
- `public/style.css` — External stylesheet (CSS-in-file, not in component)
- `index.html` — HTML entry point
- `Dioxus.toml` — Dioxus configuration
- `Cargo.toml` — Project dependencies
