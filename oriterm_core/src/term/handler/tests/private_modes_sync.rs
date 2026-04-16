use crate::term::Term;
use crate::theme::Theme;

use super::super::test_helpers::feed;

/// Create a Term with VoidEffectSink (when effects don't matter).
fn term() -> Term<crate::effect::VoidEffectSink> {
    Term::new(24, 80, 0, Theme::default(), crate::effect::VoidEffectSink)
}

// --- BSU/ESU (Synchronized Update, mode 2026) ---

#[test]
fn bsu_esu_sync_update_via_vte() {
    use crate::term::TermMode;

    let mut t = term();

    // Mode 2026 should start off.
    assert!(
        !t.mode().contains(TermMode::SYNC_UPDATE),
        "SYNC_UPDATE should be off by default"
    );

    // BSU: Begin Synchronized Update (DECSET ?2026).
    feed(&mut t, b"\x1b[?2026h");
    assert!(
        t.mode().contains(TermMode::SYNC_UPDATE),
        "SYNC_UPDATE should be on after \\x1b[?2026h"
    );

    // ESU: End Synchronized Update (DECRST ?2026).
    feed(&mut t, b"\x1b[?2026l");
    assert!(
        !t.mode().contains(TermMode::SYNC_UPDATE),
        "SYNC_UPDATE should be off after \\x1b[?2026l"
    );
}
