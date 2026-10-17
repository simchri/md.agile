//! `agile trust` — approves the form validation scripts declared in
//! `mdagile.toml` so they may be run (see [`crate::trust`]).

use crate::config::Config;
use crate::trust::{TrustStore, default_store_path, resolve_script, sha256_hex};
use std::collections::BTreeSet;
use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;

pub struct Options<'a> {
    pub yes: bool,
    pub interactive: bool,
    pub store_path: &'a Path,
}

/// `agile trust` entry point. Exits 1 on any error.
pub fn run(root: &Path, config: &Config, yes: bool) {
    let Some(store_path) = default_store_path(|name| std::env::var_os(name)) else {
        log::error!("cannot locate the trust store: neither XDG_CONFIG_HOME nor HOME is set");
        std::process::exit(1);
    };
    let options = Options {
        yes,
        interactive: std::io::stdin().is_terminal(),
        store_path: &store_path,
    };
    let result = run_with(
        root,
        config,
        &options,
        &mut std::io::stdin().lock(),
        &mut std::io::stdout(),
    );
    if let Err(message) = result {
        log::error!("{message}");
        std::process::exit(1);
    }
}

pub fn run_with(
    root: &Path,
    config: &Config,
    options: &Options,
    input: &mut impl BufRead,
    out: &mut impl Write,
) -> Result<(), String> {
    let scripts: BTreeSet<&str> = config
        .forms
        .values()
        .filter_map(|form| form.validation.as_deref())
        .collect();
    if scripts.is_empty() {
        writeln!(out, "No form validation scripts are declared.").map_err(|e| e.to_string())?;
        return Ok(());
    }

    let mut store = TrustStore::load(options.store_path).map_err(|e| e.to_string())?;
    let mut pending = Vec::new();
    for script in &scripts {
        let path = resolve_script(root, script)
            .map_err(|e| format!("invalid validation script '{script}': {e}"))?;
        let content =
            std::fs::read(&path).map_err(|e| format!("cannot read script '{script}': {e}"))?;
        let hash = sha256_hex(&content);
        if store.is_trusted(script, &hash) {
            writeln!(out, "already trusted: {script}").map_err(|e| e.to_string())?;
        } else {
            pending.push((*script, hash));
        }
    }

    if !pending.is_empty() && !options.yes && !options.interactive {
        let names: Vec<&str> = pending.iter().map(|(script, _)| *script).collect();
        return Err(format!(
            "validation script(s) not trusted: {}. Run `agile trust` in a terminal to review and approve them, or `agile trust --yes` to approve all of them non-interactively (e.g. on a CI runner).",
            names.join(", ")
        ));
    }

    let mut changed = false;
    for (script, hash) in pending {
        let approved = options.yes || {
            write!(out, "Trust {script} (sha256 {hash})? [y/N] ").map_err(|e| e.to_string())?;
            out.flush().map_err(|e| e.to_string())?;
            let mut answer = String::new();
            input.read_line(&mut answer).map_err(|e| e.to_string())?;
            answer.trim().eq_ignore_ascii_case("y")
        };
        if approved {
            store.trust(script, &hash);
            changed = true;
            writeln!(out, "trusted: {script}").map_err(|e| e.to_string())?;
        } else {
            writeln!(out, "skipped: {script}").map_err(|e| e.to_string())?;
        }
    }
    if changed {
        store.save(options.store_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "trust_tests.rs"]
mod tests;
