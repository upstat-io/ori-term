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
//! # Where the equivalent coverage WILL live (Section 08)
//!
//! **Important** Section 08 of the
//! tack-conformance plan is the planned home for function-key
//! cross-check coverage via `oriterm/src/key_encoding/terminfo_xcheck.rs`.
//! That in-crate sibling test is intended to:
//! - Load `extra/ori_term.info` via `terminfo`.
//! - Iterate every `kf*` cap declared in the terminfo entry.
//! - Map each cap to ori_term's internal key code via the
//! keymap dispatch.
//! - Assert the encoded byte sequence ori_term would write back
//! to the PTY matches the cap's terminfo string EXACTLY.
//!
//! **NOTE:** The file `oriterm/src/key_encoding/terminfo_xcheck.rs`
//! does NOT yet exist in the workspace. Until that lands,
//! function-key correctness for the `kf0..kf63` capability set
//! is NOT under automated test in this repository. This is a
//! known coverage gap that 05.4b inherits — it cannot be
//! resolved within Section 05 (the test must live in Section
//! 08's keyboard-encoding crate and use the keymap dispatch
//! that Section 08 will define).
//!
//! (Codex review-work iteration 2 of M2) flagged
//! the previous version of this stub for overstating coverage:
//! it claimed Section 08's cross-check "covers the same ground"
//! and "is faster, deterministic, and doesn't require human
//! interaction" without acknowledging that the cross-check is
//! planned future work, not landed code.
//!
//! # What 05.4b actually delivers for function keys
//!
//! Nothing executable — only this stub recording the gap. The
//! function-key entry is preserved in `BEGIN_TESTING_INVENTORY`
//! with `DelegatedToSection { section: "08" }` so the drift
//! gate still catches a future tack release that adds or
//! removes the `f)` menu entry, but Section 05 has no per-key
//! coverage of its own. The 05.5 cap-coverage matrix should
//! record the entire `kf0..kf63` set as a Section-08 deferral
//! exemption (NOT as covered) until the Section 08 cross-check
//! lands.
