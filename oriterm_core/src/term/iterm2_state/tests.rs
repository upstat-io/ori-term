use super::{Iterm2State, USER_VARS_MAX_ENTRIES};

#[test]
fn new_defaults_are_empty_with_default_cap() {
    let state = Iterm2State::new();
    assert!(state.remote_host.is_none());
    assert!(state.shell_integration_version.is_none());
    assert!(state.user_vars.is_empty());
    assert_eq!(state.user_vars_max, USER_VARS_MAX_ENTRIES);
}

#[test]
fn record_user_var_replaces_in_place_without_refreshing_position() {
    let mut state = Iterm2State::new();
    state.user_vars_max = 3;
    state.record_user_var("A".into(), "1".into());
    state.record_user_var("B".into(), "2".into());
    state.record_user_var("C".into(), "3".into());
    // A is re-inserted; IndexMap::insert replaces in place WITHOUT
    // updating position, so A stays at index 0 (front of the FIFO).
    state.record_user_var("A".into(), "updated".into());
    // Inserting D at the cap evicts index 0, which is still A.
    state.record_user_var("D".into(), "4".into());
    assert_eq!(state.user_vars.len(), 3);
    assert_eq!(state.user_vars.get("A"), None, "A must be evicted");
    assert_eq!(state.user_vars.get("B").map(String::as_str), Some("2"));
    assert_eq!(state.user_vars.get("C").map(String::as_str), Some("3"));
    assert_eq!(state.user_vars.get("D").map(String::as_str), Some("4"));
}

#[test]
fn record_user_var_evicts_oldest_on_overflow() {
    let mut state = Iterm2State::new();
    state.user_vars_max = 2;
    state.record_user_var("first".into(), "1".into());
    state.record_user_var("second".into(), "2".into());
    state.record_user_var("third".into(), "3".into());
    assert_eq!(state.user_vars.len(), 2);
    assert_eq!(state.user_vars.get("first"), None);
    assert_eq!(state.user_vars.get("second").map(String::as_str), Some("2"));
    assert_eq!(state.user_vars.get("third").map(String::as_str), Some("3"));
}

#[test]
fn record_user_var_with_zero_cap_rejects_all_inserts() {
    let mut state = Iterm2State::new();
    state.user_vars_max = 0;
    state.record_user_var("A".into(), "1".into());
    state.record_user_var("B".into(), "2".into());
    assert_eq!(state.user_vars.len(), 0);
    assert!(state.user_vars.is_empty());
}
