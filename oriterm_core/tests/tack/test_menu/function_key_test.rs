//! Doc-only stub for tack's `f) test function keys` begin-testing entry.
//!
//! Classification: `BeginTestingStatus::DelegatedToSection { section: "08" }`
//! per `crates/oriterm_test_support/src/tack_framework/scenarios/begin_testing_inventory/mod.rs`.
//!
//! # Why this is delegated, not excluded
//!
//! Pressing `f` from the begin-testing menu launches an
//! interactive function-key probe — tack prints `Press F1` and
//! waits for the user to physically press the F1 key, then
//! `Press F2`, etc. The screen is fundamentally interactive
//! (requires keystroke input from a human operator) so it cannot
//! be driven through `ScenarioRunner`. However, function-key
//! coverage IS critical for ori_term — wrong key encoding is one
//! of the most common terminal-emulator bugs.
//!
//! # Where the equivalent coverage lives
//!
//! Section 08 of the tack-conformance plan owns function-key
//! coverage via `oriterm/src/key_encoding/terminfo_xcheck.rs`.
//! That in-crate sibling test:
//! - Loads `extra/ori_term.info` via `terminfo`.
//! - Iterates every `kf*` cap declared in the terminfo entry.
//! - Maps each cap to ori_term's internal key code via the
//!   keymap dispatch.
//! - Asserts the encoded byte sequence ori_term would write back
//!   to the PTY matches the cap's terminfo string EXACTLY.
//!
//! That test is faster, deterministic, and doesn't require human
//! interaction — strictly stronger coverage than tack's `f) test
//! function keys` interactive probe. Both this stub and the
//! Section 08 cross-check live in the test tree so the connection
//! is visible to future readers.
