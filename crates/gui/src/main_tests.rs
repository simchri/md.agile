use super::*;

#[test]
fn resolve_kiosk_flag_hides_write_actions_while_loading() {
    // Resource not yet resolved — fail safe, hide write actions.
    assert!(resolve_kiosk_flag(None));
}

#[test]
fn resolve_kiosk_flag_hides_write_actions_on_error() {
    // Resource resolved but failed — fail safe, hide write actions.
    let result: Result<bool, ServerFnError> = Err(ServerFnError::new("boom"));
    assert!(resolve_kiosk_flag(Some(&result)));
}

#[test]
fn resolve_kiosk_flag_shows_write_actions_when_kiosk_mode_is_off() {
    let result: Result<bool, ServerFnError> = Ok(false);
    assert!(!resolve_kiosk_flag(Some(&result)));
}

#[test]
fn resolve_kiosk_flag_hides_write_actions_when_kiosk_mode_is_on() {
    let result: Result<bool, ServerFnError> = Ok(true);
    assert!(resolve_kiosk_flag(Some(&result)));
}

#[test]
fn format_server_error_strips_the_generic_server_function_wrapper() {
    // `ServerFnError::new(...)` (used by every server fn error in this
    // crate) produces `ServerFnError::ServerError`, whose `Display` impl
    // wraps the message as `"error running server function: {message}
    // (details: {details:#?})"`. Only the plain message is worth showing.
    let error = ServerFnError::new("\"foo\" is already done");
    assert_eq!(format_server_error(&error), "\"foo\" is already done");
}

#[test]
fn format_server_error_falls_back_to_display_for_non_server_error_variants() {
    // Variants other than `ServerError` (e.g. a network failure) don't
    // carry a separate plain message, so the full `Display` output is kept.
    let error = ServerFnError::Registration("poisoned lock".to_string());
    assert_eq!(format_server_error(&error), error.to_string());
}
