//! Excluded: tack's `e) echo tool` is an interactive keyboard-echo
//! probe — it reads from stdin and displays each keystroke as it's
//! received. It cannot be automated from the PTY test harness
//! because there is no "done" anchor and no deterministic exit
//! condition. The related keyboard echo correctness is validated
//! by Section 08's in-crate sibling tests at
//! `oriterm/src/key_encoding/terminfo_xcheck.rs`.
