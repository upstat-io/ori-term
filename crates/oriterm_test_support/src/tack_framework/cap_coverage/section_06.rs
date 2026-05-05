//! Section 06's cap-coverage contribution.
//!
//! Section 06 (tools menu + direct-VTE cap xcheck) covers 27 caps
//! across two tracks:
//!   - Track A — 4 tack-reachable caps (`u6`/`u7` via 06.1
//!     `status_reports` walker; `u8`/`u9` via 06.4 ENQ/ACK
//!     scenario).
//!   - Track B — 23 direct-VTE caps via 06.5 `tack_cap_xcheck`
//!     (Smulx, Setulc, Sync, BD, BE, PS, PE, Se, Ss, XF, kxIN,
//!     kxOUT, Tc, RGB, Cr, Cs, Ms, hs, dsl, fsl, tsl, AX, XT).
//!
//! All 27 caps live in `covered`. The `exempt` slice is empty;
//! Section 06 leaves no terminfo cap unowned. The
//! stale-exemption regression guard in
//! `oriterm_core/tests/tack/test_menu/cap_coverage_matrix.rs`
//! verifies no cap appears in both lists.

use super::CapCoverageContribution;

pub const CONTRIBUTION: CapCoverageContribution = CapCoverageContribution {
    section: "06",
    covered: &[
        // ----- Track A: tack-reachable caps -----
        // Status-report CPR cap pair — covered by Section 06.1
        // status_reports walker. The `(DSR) Cursor position
        // (CSI 6 n)` sub-test exercises `u7`; the round-trip
        // response exercises `u6`. Pin enforced by
        // `is_dsr_cursor_position_response` in
        // `scenarios::status_reports::mod.rs` and the walker
        // test in `oriterm_core/tests/tack/tools_menu/status_reports.rs`.
        "u6", "u7",
        // ENQ/ACK handshake cap pair — covered by Section 06.4
        // enq_ack scenario. tack's `u) test ENQ/ACK (DA1)
        // handshake` tool sends `u9` (DA1 query `\E[c`) to
        // ori_term, which responds via
        // `oriterm_core/src/term/handler/status.rs:121-148` with
        // `\x1b[?64;6;4c`. tack matches the response against the
        // `u8` regex `\E[?%[;0123456789]c` and reports the
        // success-path `ACK terminating character: c` line.
        "u8", "u9",
        // ----- Track B: direct-VTE cap xcheck (Section 06.5) -----
        // SGR extensions: kitty colon underline + truecolor
        // underline color. Tested in
        // `oriterm_core/src/term/handler/tack_cap_xcheck/sgr_extensions.rs`
        // via direct-VTE feeds of `CSI 4:N m` and `CSI 58:2;r;g;b m`.
        "Smulx", "Setulc",
        // Synchronized output mode 2026. Tested in
        // `tack_cap_xcheck/sync.rs` via DECSET/DECRST 2026
        // toggling `TermMode::SYNC_UPDATE`.
        "Sync",
        // Bracketed paste: mode toggle (`BD`/`BE` via DECSET/DECRST
        // 2004) and outbound markers (`PS`/`PE` via the
        // `oriterm_core::paste::prepare_paste` pure function).
        // Tested in `tack_cap_xcheck/bracketed_paste.rs`.
        "BD", "BE", "PS", "PE",
        // DECSCUSR cursor style (`Ss` set / `Se` reset). Tested
        // in `tack_cap_xcheck/cursor_style.rs` via direct CSI N SP q
        // feeds and `term.cursor_shape()` / `TermMode::CURSOR_BLINKING`
        // assertions.
        "Se", "Ss",
        // Focus event support: `XF` bool advertisement +
        // outbound `kxIN`/`kxOUT` byte sequences. `XF` tested in
        // `tack_cap_xcheck/focus_events.rs` via terminfo
        // declaration check; `kxIN`/`kxOUT` tested CROSS-CRATE
        // in `oriterm/src/app/event_loop_helpers/tests.rs`
        // because the byte emission requires a winit-driven
        // focus path that lives in the `oriterm` crate.
        "XF", "kxIN", "kxOUT",
        // Truecolor advertisement (`Tc` bool + `RGB` direct-color
        // marker). Tested in `tack_cap_xcheck/truecolor.rs` via
        // CSI 38;2;r;g;b m round-trip plus declaration checks.
        "Tc", "RGB",
        // OSC cursor color set/reset (`Cs` via OSC 12 set, `Cr`
        // via OSC 112 reset) + OSC 52 clipboard (`Ms`). Tested
        // in `tack_cap_xcheck/osc_color.rs` and
        // `tack_cap_xcheck/osc_clipboard.rs` via OSC round-trips
        // through the palette and the `Effect::Host(ClipboardStore)`
        // / `Effect::HostRequest` paths.
        "Cr", "Cs", "Ms",
        // Status line via OSC 0/2 title-backed (`hs` bool + `tsl`
        // open + `fsl` terminate + `dsl` clear). Tested in
        // `tack_cap_xcheck/status_line.rs` via OSC 2 round-trip
        // through `osc_set_title` (`oriterm_core/src/term/handler/osc.rs:22`).
        "hs", "dsl", "fsl", "tsl",
        // xterm extension markers (`AX`/`XT` bools). Tested in
        // `tack_cap_xcheck/xterm_markers.rs` via terminfo
        // declaration checks.
        "AX", "XT",
    ],
    exempt: &[],
};
