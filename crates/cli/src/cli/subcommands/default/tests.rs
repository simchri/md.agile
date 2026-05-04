use super::*;
use std::ffi::OsString;
use std::path::Path;

fn s(v: &str) -> OsString {
    OsString::from(v)
}

#[test]
fn vim_uses_plus_line() {
    assert_eq!(
        editor_open_args("vim", Path::new("f.agile.md"), 5),
        vec![s("+5"), s("f.agile.md")]
    );
}

#[test]
fn vi_uses_plus_line() {
    assert_eq!(
        editor_open_args("vi", Path::new("f.agile.md"), 1),
        vec![s("+1"), s("f.agile.md")]
    );
}

#[test]
fn nvim_uses_plus_line() {
    assert_eq!(
        editor_open_args("nvim", Path::new("f.agile.md"), 3),
        vec![s("+3"), s("f.agile.md")]
    );
}

#[test]
fn nano_uses_plus_line() {
    assert_eq!(
        editor_open_args("nano", Path::new("f.agile.md"), 7),
        vec![s("+7"), s("f.agile.md")]
    );
}

#[test]
fn emacs_uses_plus_line() {
    assert_eq!(
        editor_open_args("emacs", Path::new("f.agile.md"), 2),
        vec![s("+2"), s("f.agile.md")]
    );
}

#[test]
fn code_uses_goto_flag() {
    assert_eq!(
        editor_open_args("code", Path::new("f.agile.md"), 4),
        vec![s("--goto"), s("f.agile.md:4")]
    );
}

#[test]
fn full_path_uses_basename_for_matching() {
    assert_eq!(
        editor_open_args("/usr/bin/nvim", Path::new("f.agile.md"), 9),
        vec![s("+9"), s("f.agile.md")]
    );
}

#[test]
fn unknown_editor_omits_line_number() {
    assert_eq!(
        editor_open_args("gedit", Path::new("f.agile.md"), 6),
        vec![s("f.agile.md")]
    );
}
