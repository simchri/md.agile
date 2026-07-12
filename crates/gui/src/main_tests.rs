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
