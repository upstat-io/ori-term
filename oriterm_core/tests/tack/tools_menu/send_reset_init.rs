//! Excluded: tack's `i) send reset and init` overlaps with the
//! identically-named begin-testing `i) send reset and init`
//! exclusion stub from Section 05.0's `BEGIN_TESTING_INVENTORY`. The
//! canonical exclusion lives in
//! `oriterm_core/tests/tack/test_menu/send_reset_init.rs`. This
//! file exists only to satisfy the drift gate in
//! `TOOLS_MENU_INVENTORY` — any test of the reset/init sequences
//! belongs in Section 05's location.
