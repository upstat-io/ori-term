//! Dispatch-arm tests for CSI sequences.
//!
//! Each new dispatch arm gets a parse test (raw bytes → handler method called
//! with expected params, observed via `RecordingHandler`) plus an unhandled
//! negative pin (same final byte with a different intermediate falls through
//! to `unhandled!()`).
//!
//! Parser-level tests for `ansi/mod.rs` live at `crates/vte/src/ansi/tests.rs`
//! per `.claude/rules/test-organization.md` §Sibling tests.rs Pattern.

use core::time::Duration;

use crate::ansi::handler::Handler;
use crate::ansi::{Processor, Timeout};

#[derive(Default)]
struct TestSync(usize);

impl Timeout for TestSync {
    fn set_timeout(&mut self, _: Duration) {
        self.0 += 1;
    }
    fn clear_timeout(&mut self) {
        self.0 = 0;
    }
    fn pending_timeout(&self) -> bool {
        self.0 != 0
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum Call {
    Decsace(u16),
    Deccara(u16, u16, u16, u16, Vec<u16>),
    Decrara(u16, u16, u16, u16, Vec<u16>),
    Deccra(u16, u16, u16, u16, u16, u16, u16, u16),
    Decfra(u16, u16, u16, u16, u16),
    Xtchecksum(u16),
    Decrqcra(u16, u16, u16, u16, u16, u16),
    Decera(u16, u16, u16, u16),
    Decsera(u16, u16, u16, u16),
    Xtreportsgr(u16, u16, u16, u16),
    Decrqpsr(u16),
    Decrqupss,
    Decrqde,
    Decscl(u16, u16),
    Decsca(u16),
    Decsasd(u16),
    Decssdt(u16),
    Decic(u16),
    Decdc(u16),
}

#[derive(Default)]
struct RecordingHandler {
    calls: Vec<Call>,
}

impl Handler for RecordingHandler {
    fn decsace(&mut self, mode: u16) {
        self.calls.push(Call::Decsace(mode));
    }
    fn deccara(&mut self, top: u16, left: u16, bot: u16, right: u16, attrs: &[u16]) {
        self.calls.push(Call::Deccara(top, left, bot, right, attrs.to_vec()));
    }
    fn decrara(&mut self, top: u16, left: u16, bot: u16, right: u16, attrs: &[u16]) {
        self.calls.push(Call::Decrara(top, left, bot, right, attrs.to_vec()));
    }
    fn deccra(
        &mut self,
        st: u16,
        sl: u16,
        sb: u16,
        sr: u16,
        sp: u16,
        dt: u16,
        dl: u16,
        dp: u16,
    ) {
        self.calls.push(Call::Deccra(st, sl, sb, sr, sp, dt, dl, dp));
    }
    fn decfra(&mut self, ch: u16, top: u16, left: u16, bot: u16, right: u16) {
        self.calls.push(Call::Decfra(ch, top, left, bot, right));
    }
    fn xtchecksum(&mut self, flags: u16) {
        self.calls.push(Call::Xtchecksum(flags));
    }
    fn decrqcra(&mut self, id: u16, page: u16, top: u16, left: u16, bot: u16, right: u16) {
        self.calls.push(Call::Decrqcra(id, page, top, left, bot, right));
    }
    fn decera(&mut self, top: u16, left: u16, bot: u16, right: u16) {
        self.calls.push(Call::Decera(top, left, bot, right));
    }
    fn decsera(&mut self, top: u16, left: u16, bot: u16, right: u16) {
        self.calls.push(Call::Decsera(top, left, bot, right));
    }
    fn xtreportsgr(&mut self, top: u16, left: u16, bot: u16, right: u16) {
        self.calls.push(Call::Xtreportsgr(top, left, bot, right));
    }
    fn decrqpsr(&mut self, mode: u16) {
        self.calls.push(Call::Decrqpsr(mode));
    }
    fn decrqupss(&mut self) {
        self.calls.push(Call::Decrqupss);
    }
    fn decrqde(&mut self) {
        self.calls.push(Call::Decrqde);
    }
    fn decscl(&mut self, level: u16, c1_mode: u16) {
        self.calls.push(Call::Decscl(level, c1_mode));
    }
    fn decsca(&mut self, protected: u16) {
        self.calls.push(Call::Decsca(protected));
    }
    fn decsasd(&mut self, target: u16) {
        self.calls.push(Call::Decsasd(target));
    }
    fn decssdt(&mut self, line_type: u16) {
        self.calls.push(Call::Decssdt(line_type));
    }
    fn decic(&mut self, count: u16) {
        self.calls.push(Call::Decic(count));
    }
    fn decdc(&mut self, count: u16) {
        self.calls.push(Call::Decdc(count));
    }
}

fn run(bytes: &[u8]) -> Vec<Call> {
    let mut p = Processor::<TestSync>::new();
    let mut h = RecordingHandler::default();
    p.advance(&mut h, bytes);
    h.calls
}

// ── DEC rectangular ops ─────────────────────────────────────────────

#[test]
fn deccara_dispatches_with_attrs() {
    let calls = run(b"\x1b[5;3;10;8;1;7$r");
    assert_eq!(calls, vec![Call::Deccara(5, 3, 10, 8, vec![1, 7])]);
}

#[test]
fn deccara_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[5;3;10;8;1;7%r");
    assert!(calls.is_empty());
}

#[test]
fn decrara_dispatches_with_attrs() {
    let calls = run(b"\x1b[1;1;5;5;7$t");
    assert_eq!(calls, vec![Call::Decrara(1, 1, 5, 5, vec![7])]);
}

#[test]
fn decrara_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[1;1;5;5;7%t");
    assert!(calls.is_empty());
}

#[test]
fn deccra_dispatches_eight_coordinates() {
    let calls = run(b"\x1b[1;1;5;5;1;6;6;1$v");
    assert_eq!(calls, vec![Call::Deccra(1, 1, 5, 5, 1, 6, 6, 1)]);
}

#[test]
fn deccra_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[1;1;5;5;1;6;6;1%v");
    assert!(calls.is_empty());
}

#[test]
fn decsace_dispatches_mode() {
    let calls = run(b"\x1b[2*x");
    assert_eq!(calls, vec![Call::Decsace(2)]);
}

#[test]
fn decsace_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[2%x");
    assert!(calls.is_empty());
}

#[test]
fn decfra_dispatches_with_char_and_rect() {
    let calls = run(b"\x1b[42;1;1;5;5$x");
    assert_eq!(calls, vec![Call::Decfra(42, 1, 1, 5, 5)]);
}

#[test]
fn decfra_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[42;1;1;5;5%x");
    assert!(calls.is_empty());
}

#[test]
fn xtchecksum_dispatches_flags() {
    let calls = run(b"\x1b[3#y");
    assert_eq!(calls, vec![Call::Xtchecksum(3)]);
}

#[test]
fn xtchecksum_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[3%y");
    assert!(calls.is_empty());
}

#[test]
fn decrqcra_dispatches_id_page_rect() {
    let calls = run(b"\x1b[7;1;1;1;5;5*y");
    assert_eq!(calls, vec![Call::Decrqcra(7, 1, 1, 1, 5, 5)]);
}

#[test]
fn decrqcra_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[7;1;1;1;5;5%y");
    assert!(calls.is_empty());
}

#[test]
fn decera_dispatches_rect() {
    let calls = run(b"\x1b[1;1;5;5$z");
    assert_eq!(calls, vec![Call::Decera(1, 1, 5, 5)]);
}

#[test]
fn decera_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[1;1;5;5%z");
    assert!(calls.is_empty());
}

#[test]
fn decsera_dispatches_rect() {
    let calls = run(b"\x1b[1;1;5;5${");
    assert_eq!(calls, vec![Call::Decsera(1, 1, 5, 5)]);
}

#[test]
fn decsera_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[1;1;5;5%{");
    assert!(calls.is_empty());
}

#[test]
fn xtreportsgr_dispatches_rect() {
    let calls = run(b"\x1b[1;1;5;5#|");
    assert_eq!(calls, vec![Call::Xtreportsgr(1, 1, 5, 5)]);
}

#[test]
fn xtreportsgr_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[1;1;5;5%|");
    assert!(calls.is_empty());
}

// ── DEC presentation ops ─────────────────────────────────────────────

#[test]
fn decrqpsr_dispatches_mode() {
    let calls = run(b"\x1b[1$w");
    assert_eq!(calls, vec![Call::Decrqpsr(1)]);
}

#[test]
fn decrqpsr_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[1%w");
    assert!(calls.is_empty());
}

#[test]
fn decrqupss_dispatches() {
    let calls = run(b"\x1b[&u");
    assert_eq!(calls, vec![Call::Decrqupss]);
}

#[test]
fn decrqupss_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[%u");
    assert!(calls.is_empty());
}

#[test]
fn decrqde_dispatches() {
    let calls = run(b"\x1b[\"v");
    assert_eq!(calls, vec![Call::Decrqde]);
}

#[test]
fn decrqde_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[%v");
    assert!(calls.is_empty());
}

#[test]
fn decscl_dispatches_level_and_c1_mode() {
    let calls = run(b"\x1b[65;1\"p");
    assert_eq!(calls, vec![Call::Decscl(65, 1)]);
}

#[test]
fn decscl_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[65;1%p");
    assert!(calls.is_empty());
}

#[test]
fn decsca_dispatches_protected() {
    let calls = run(b"\x1b[1\"q");
    assert_eq!(calls, vec![Call::Decsca(1)]);
}

#[test]
fn decsca_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[1%q");
    assert!(calls.is_empty());
}

#[test]
fn decsasd_dispatches_target() {
    let calls = run(b"\x1b[1$}");
    assert_eq!(calls, vec![Call::Decsasd(1)]);
}

#[test]
fn decsasd_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[1%}");
    assert!(calls.is_empty());
}

#[test]
fn decssdt_dispatches_line_type() {
    let calls = run(b"\x1b[2$~");
    assert_eq!(calls, vec![Call::Decssdt(2)]);
}

#[test]
fn decssdt_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[2%~");
    assert!(calls.is_empty());
}

#[test]
fn decic_dispatches_count() {
    let calls = run(b"\x1b[5'}");
    assert_eq!(calls, vec![Call::Decic(5)]);
}

#[test]
fn decic_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[5%}");
    assert!(calls.is_empty());
}

#[test]
fn decdc_dispatches_count() {
    let calls = run(b"\x1b[3'~");
    assert_eq!(calls, vec![Call::Decdc(3)]);
}

#[test]
fn decdc_unhandled_with_other_intermediate() {
    let calls = run(b"\x1b[3%~");
    assert!(calls.is_empty());
}
