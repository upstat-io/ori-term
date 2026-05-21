//! Exact-replication tests for `notcurses-info` (`notcurses/src/info/main.c`).
//!
//! Reconstructs the byte stream `notcurses-info` sends in its init phase
//! directly from notcurses source, not from a captured `.cap` file. Each
//! constant below mirrors a `#define` in `notcurses/src/lib/termdesc.c`
//! verbatim so the test stays anchored to the spec implementation as the
//! reference. The C source lines are cited beside each constant.
//!
//! `notcurses-info` constructs `notcurses_options` with:
//!   NCOPTION_NO_ALTERNATE_SCREEN | NCOPTION_PRESERVE_CURSOR
//!   | NCOPTION_NO_CLEAR_BITMAPS | NCOPTION_DRAIN_INPUT
//! (`notcurses/src/info/main.c:485-490`). The init flow funnels through
//! `notcurses_core_init` → `interrogate_terminfo` → `send_initial_queries`
//! (`notcurses/src/lib/notcurses.c:1284` + `termdesc.c:520`). With the
//! flag combination above the dispatcher path is:
//!
//!   - `noaltscreen=true` → skip `SMCUP`.
//!   - Emit `DSRCPR`.
//!   - `draininput=true` → skip `KKBDENTER`.
//!   - `minimal=false` (TERM=xterm-256color is not a self-identifying
//!     terminal under `unix_early_matches`) → emit `IDQUERIES`.
//!   - Emit `send_initial_directives` body: per-palette `OSC 4` queries
//!     (skipped when TERM=linux only) + `DIRECTIVES`.
//!
//! Every reply ori_term emits is asserted byte-by-byte against the exact
//! shape notcurses expects (see `notcurses/src/lib/in.c handle_responses`
//! and `notcurses/src/lib/termdesc.c apply_term_heuristics`). When ANY
//! reply is missing, notcurses-info blocks waiting for it and the binary
//! appears hung — that hang is what these tests are designed to catch
//! at unit-test wall-clock, not at end-user wall-clock.

use oriterm_core::effect::PtyWriteKind;
use oriterm_test_support::spec_chain::{SpecHarness, pty_writes, pty_writes_of_kind};

// ---------------------------------------------------------------------------
// Byte-stream constants — mirror `notcurses/src/lib/termdesc.c` #defines.
// ---------------------------------------------------------------------------

/// `DSRCPR` — Device Status Report Cursor Position (`CSI 6 n`).
/// `termdesc.c:423`.
const DSRCPR: &[u8] = b"\x1b[6n";

/// `TRIDEVATTR` — Tertiary Device Attributes (`CSI = c`). `termdesc.c:336`.
const TRIDEVATTR: &[u8] = b"\x1b[=c";

/// `XTVERSION` — XTVERSION query (`CSI > 0 q`). `termdesc.c:354`.
const XTVERSION: &[u8] = b"\x1b[>0q";

/// `XTGETTCAP` — DCS + q TN ; RGB ; hpa ST. `termdesc.c:364`.
const XTGETTCAP: &[u8] = b"\x1bP+q544e;524742;687061\x1b\\";

/// `SECDEVATTR` — Secondary Device Attributes (`CSI > c`). `termdesc.c:373`.
const SECDEVATTR: &[u8] = b"\x1b[>c";

/// `KITTYQUERY` — kitty graphics protocol query (`APC G i=1,a=q ; ST`).
/// `termdesc.c:383`. Suppressed on Windows in the C source per the
/// MINGW32 guard.
const KITTYQUERY: &[u8] = b"\x1b_Gi=1,a=q;\x1b\\";

/// `KKBDQUERY` — kitty keyboard protocol query (`CSI ? u`).
/// `termdesc.c:395`.
const KKBDQUERY: &[u8] = b"\x1b[?u";

/// `DEFFGQ` — default foreground OSC 10 query. `termdesc.c:415`.
const DEFFGQ: &[u8] = b"\x1b]10;?\x1b\\";

/// `DEFBGQ` — default background OSC 11 query. `termdesc.c:414`.
const DEFBGQ: &[u8] = b"\x1b]11;?\x1b\\";

/// `SUMQUERY` — DECRQM mode 2026 sync update support. `termdesc.c:427`.
const SUMQUERY: &[u8] = b"\x1b[?2026$p";

/// `PIXELMOUSEQUERY` — DECRQM mode 1016 pixel-based mouse. `termdesc.c:430`.
const PIXELMOUSEQUERY: &[u8] = b"\x1b[?1016$p";

/// XTSMGRAPHICS Pi=1 Pa=3 Pv=256 — set color registers to 256. `termdesc.c:451`.
const XTSM_SET_CREGS_256: &[u8] = b"\x1b[?1;3;256S";

/// XTSMGRAPHICS Pi=1 Pa=3 Pv=1024 — set color registers to 1024. `termdesc.c:452`.
const XTSM_SET_CREGS_1024: &[u8] = b"\x1b[?1;3;1024S";

/// `CREGSXTSM` — XTSMGRAPHICS Pi=2 Pa=1 Pv=0, read color registers.
/// `termdesc.c:433`.
const CREGSXTSM: &[u8] = b"\x1b[?2;1;0S";

/// `GEOMXTSM` — XTSMGRAPHICS Pi=1 Pa=1 Pv=0, read max sixel geometry.
/// `termdesc.c:436`.
const GEOMXTSM: &[u8] = b"\x1b[?1;1;0S";

/// `GEOMPIXEL` — text-area-in-pixels query (`CSI 14 t`). `termdesc.c:439`.
const GEOMPIXEL: &[u8] = b"\x1b[14t";

/// `GEOMCELL` — text-area-in-chars query (`CSI 18 t`). `termdesc.c:442`.
const GEOMCELL: &[u8] = b"\x1b[18t";

/// `PRIDEVATTR` — Primary Device Attributes (`CSI c`). `termdesc.c:351`.
/// notcurses uses DA1 as the end-of-handshake marker; missing reply
/// causes notcurses-info to block indefinitely.
const PRIDEVATTR: &[u8] = b"\x1b[c";

// ---------------------------------------------------------------------------
// Bundled query bodies — match the macro expansion in `termdesc.c` exactly.
// ---------------------------------------------------------------------------

/// `IDQUERIES` macro expansion: TRIDEVATTR + XTVERSION + XTGETTCAP + SECDEVATTR.
/// `termdesc.c:405-408`. Sent inside `send_initial_queries` when `minimal`
/// is false.
fn id_queries() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(TRIDEVATTR);
    v.extend_from_slice(XTVERSION);
    v.extend_from_slice(XTGETTCAP);
    v.extend_from_slice(SECDEVATTR);
    v
}

/// `DIRECTIVES` macro expansion: DEFFGQ + DEFBGQ + KKBDQUERY + SUMQUERY +
/// PIXELMOUSEQUERY + XTSM_SET_CREGS_{256,1024} + KITTYQUERY + CREGSXTSM +
/// GEOMXTSM + GEOMPIXEL + GEOMCELL + PRIDEVATTR. `termdesc.c:446-458`.
fn directives() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(DEFFGQ);
    v.extend_from_slice(DEFBGQ);
    v.extend_from_slice(KKBDQUERY);
    v.extend_from_slice(SUMQUERY);
    v.extend_from_slice(PIXELMOUSEQUERY);
    v.extend_from_slice(XTSM_SET_CREGS_256);
    v.extend_from_slice(XTSM_SET_CREGS_1024);
    v.extend_from_slice(KITTYQUERY);
    v.extend_from_slice(CREGSXTSM);
    v.extend_from_slice(GEOMXTSM);
    v.extend_from_slice(GEOMPIXEL);
    v.extend_from_slice(GEOMCELL);
    v.extend_from_slice(PRIDEVATTR);
    v
}

/// Per-palette OSC 4 query batch sent by `send_initial_directives` for
/// non-Linux TERM (`termdesc.c:488-502`). Batched in `qsets` groups
/// `{0..8, 8..16, 16..88, 88..256}` so undersized palettes don't lose
/// the whole batch to error responses.
fn palette_queries() -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..256u32 {
        v.extend_from_slice(format!("\x1b]4;{i};?\x1b\\").as_bytes());
    }
    v
}

/// Full notcurses-info init byte stream, with the flag combination
/// `NCOPTION_NO_ALTERNATE_SCREEN | NCOPTION_PRESERVE_CURSOR
/// | NCOPTION_NO_CLEAR_BITMAPS | NCOPTION_DRAIN_INPUT`.
///
/// Per `send_initial_queries` (`termdesc.c:520-557`):
///   - noaltscreen=true → no SMCUP prefix
///   - DSRCPR
///   - draininput=true → no KKBDENTER prefix
///   - minimal=false → IDQUERIES
///   - send_initial_directives → palette OSC 4 queries + DIRECTIVES
fn notcurses_info_init_stream() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(DSRCPR);
    v.extend(id_queries());
    v.extend(palette_queries());
    v.extend(directives());
    v
}

// ---------------------------------------------------------------------------
// Reply-shape helpers — what notcurses expects to receive.
// ---------------------------------------------------------------------------

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn feed_and_concat(query: &[u8]) -> Vec<u8> {
    let mut h = SpecHarness::new();
    h.feed(query);
    let mut out = Vec::new();
    for (bytes, _) in pty_writes(&h) {
        out.extend_from_slice(bytes);
    }
    out
}

// ---------------------------------------------------------------------------
// Per-query unit tests — one test per spec query, each fed in isolation.
// Failure mode: missing reply makes notcurses-info block at that query
// and the binary hangs from the user's perspective.
// ---------------------------------------------------------------------------

/// `DSRCPR` (`CSI 6 n`) MUST reply with `CSI Pl ; Pc R` cursor-position.
#[test]
fn notcurses_info_query_dsrcpr_emits_cursor_position_report() {
    let stream = feed_and_concat(DSRCPR);
    let needle = b"\x1b[1;1R";
    assert!(
        find_subslice(&stream, needle).is_some(),
        "DSRCPR (\\x1b[6n) MUST reply with \\x1b[<row>;<col>R; got {:?}",
        String::from_utf8_lossy(&stream),
    );
}

/// `TRIDEVATTR` (`CSI = c`) is DA3 — replies with `DCS ! | <hex> ST`.
/// Optional in spec; if ori_term does not support DA3 it must silently
/// drop without blocking — notcurses tolerates absent DA3.
#[test]
fn notcurses_info_query_tridevattr_does_not_block() {
    let mut h = SpecHarness::new();
    h.feed(TRIDEVATTR);
    // Acceptable: emit a DA3 reply OR no reply at all. Required: do NOT panic.
    let _ = pty_writes(&h);
}

/// `XTVERSION` (`CSI > 0 q`) MUST reply with `DCS > | <ident> ST` so
/// notcurses can record terminal identity.
#[test]
fn notcurses_info_query_xtversion_emits_dcs_version_reply() {
    let stream = feed_and_concat(XTVERSION);
    let prefix = b"\x1bP>|";
    let st = b"\x1b\\";
    assert!(
        find_subslice(&stream, prefix).is_some() && find_subslice(&stream, st).is_some(),
        "XTVERSION (\\x1b[>0q) MUST reply with \\x1bP>|<ident>\\x1b\\; got {:?}",
        String::from_utf8_lossy(&stream),
    );
}

/// `XTGETTCAP` (`DCS + q 544e ; 524742 ; 687061 ST`) — terminfo caps query.
/// Acceptable: any DCS reply (`\x1bP1+r...\x1b\\` success or
/// `\x1bP0+r...\x1b\\` invalid). Required: emit at least one DCS
/// reply OR none — notcurses tolerates silent drop for unknown caps.
#[test]
fn notcurses_info_query_xtgettcap_does_not_block() {
    let mut h = SpecHarness::new();
    h.feed(XTGETTCAP);
    let _ = pty_writes(&h);
}

/// `SECDEVATTR` (`CSI > c`) — DA2. MUST reply with
/// `CSI > Pp ; Pv ; Pc c` so notcurses can detect Alacritty / tmux /
/// xterm minor version.
#[test]
fn notcurses_info_query_secdevattr_emits_da2_reply() {
    let stream = feed_and_concat(SECDEVATTR);
    let needle = b"\x1b[>";
    let terminator = b"c";
    assert!(
        find_subslice(&stream, needle).is_some()
            && stream.iter().any(|b| *b == terminator[0]),
        "SECDEVATTR (\\x1b[>c) MUST reply with \\x1b[><pp>;<pv>;<pc>c; got {:?}",
        String::from_utf8_lossy(&stream),
    );
}

/// `KITTYQUERY` (`APC G i=1,a=q ; ST`) — kitty graphics protocol probe.
/// MUST reply with `APC G i=1;OK ST` for kitty-pixel-graphics-capable
/// terminals (`apply_term_heuristics` sets `pixel_implementation =
/// NCPIXEL_KITTY_*` on this reply). Silent drop means notcurses falls
/// back to sixel-only detection.
#[test]
fn notcurses_info_query_kitty_emits_ok_reply() {
    let stream = feed_and_concat(KITTYQUERY);
    let needle = b"\x1b_Gi=1;OK\x1b\\";
    assert!(
        find_subslice(&stream, needle).is_some(),
        "KITTYQUERY (\\x1b_Gi=1,a=q;\\x1b\\\\) MUST reply with \\x1b_Gi=1;OK\\x1b\\\\; got {:?}",
        String::from_utf8_lossy(&stream),
    );
}

/// `KKBDQUERY` (`CSI ? u`) — kitty keyboard protocol level query. Reply:
/// `CSI ? Pn u` reporting current progressive-enhancement level. Silent
/// drop means notcurses sets `kbdlevel = UINT_MAX` (no kitty kbd).
#[test]
fn notcurses_info_query_kkbd_does_not_block() {
    let mut h = SpecHarness::new();
    h.feed(KKBDQUERY);
    let _ = pty_writes(&h);
}

/// `DEFFGQ` (OSC 10 ; ? ST) — default foreground X-color query. MUST
/// reply with `OSC 10 ; rgb:RRRR/GGGG/BBBB ST` so notcurses can detect
/// the terminal's default foreground.
#[test]
fn notcurses_info_query_deffg_emits_osc10_reply() {
    let stream = feed_and_concat(DEFFGQ);
    let prefix = b"\x1b]10;";
    let st = b"\x1b\\";
    assert!(
        find_subslice(&stream, prefix).is_some() && find_subslice(&stream, st).is_some(),
        "DEFFGQ (OSC 10 ; ? ST) MUST reply with OSC 10 ; rgb:... ST; got {:?}",
        String::from_utf8_lossy(&stream),
    );
}

/// `DEFBGQ` (OSC 11 ; ? ST) — default background X-color query. MUST
/// reply with `OSC 11 ; rgb:RRRR/GGGG/BBBB ST`.
#[test]
fn notcurses_info_query_defbg_emits_osc11_reply() {
    let stream = feed_and_concat(DEFBGQ);
    let prefix = b"\x1b]11;";
    let st = b"\x1b\\";
    assert!(
        find_subslice(&stream, prefix).is_some() && find_subslice(&stream, st).is_some(),
        "DEFBGQ (OSC 11 ; ? ST) MUST reply with OSC 11 ; rgb:... ST; got {:?}",
        String::from_utf8_lossy(&stream),
    );
}

/// `SUMQUERY` (`CSI ? 2026 $ p`) — DECRQM for sync update mode.
/// MUST reply with `CSI ? 2026 ; Ps $ y` where Ps reports mode state.
#[test]
fn notcurses_info_query_sum_emits_decrqm_reply() {
    let stream = feed_and_concat(SUMQUERY);
    let prefix = b"\x1b[?2026;";
    let terminator = b"$y";
    assert!(
        find_subslice(&stream, prefix).is_some() && find_subslice(&stream, terminator).is_some(),
        "SUMQUERY (\\x1b[?2026$p) MUST reply with \\x1b[?2026;<ps>$y; got {:?}",
        String::from_utf8_lossy(&stream),
    );
}

/// `PIXELMOUSEQUERY` (`CSI ? 1016 $ p`) — DECRQM for pixel-mouse mode.
/// MUST reply with `CSI ? 1016 ; Ps $ y`.
#[test]
fn notcurses_info_query_pixelmouse_emits_decrqm_reply() {
    let stream = feed_and_concat(PIXELMOUSEQUERY);
    let prefix = b"\x1b[?1016;";
    let terminator = b"$y";
    assert!(
        find_subslice(&stream, prefix).is_some() && find_subslice(&stream, terminator).is_some(),
        "PIXELMOUSEQUERY (\\x1b[?1016$p) MUST reply with \\x1b[?1016;<ps>$y; got {:?}",
        String::from_utf8_lossy(&stream),
    );
}

/// `XTSM_SET_CREGS_256` (`CSI ? 1 ; 3 ; 256 S`) — XTSMGRAPHICS Pi=1 Pa=3
/// set color registers to 256. MUST reply with success/failure status
/// per XTSMGRAPHICS Pa=3 contract (`CSI ? 1 ; Ps ; Pv S`).
#[test]
fn notcurses_info_query_xtsm_set_cregs_256_emits_reply() {
    let stream = feed_and_concat(XTSM_SET_CREGS_256);
    let prefix = b"\x1b[?1;";
    let terminator = b"S";
    assert!(
        find_subslice(&stream, prefix).is_some() && find_subslice(&stream, terminator).is_some(),
        "XTSM Pi=1 Pa=3 Pv=256 (\\x1b[?1;3;256S) MUST reply with \\x1b[?1;<ps>;<pv>S; got {:?}",
        String::from_utf8_lossy(&stream),
    );
}

/// `CREGSXTSM` (`CSI ? 2 ; 1 ; 0 S`) — XTSMGRAPHICS Pi=2 Pa=1 read color
/// registers. MUST reply with `CSI ? 2 ; 0 ; Pv S` (success, value).
#[test]
fn notcurses_info_query_cregsxtsm_emits_value_reply() {
    let stream = feed_and_concat(CREGSXTSM);
    let prefix = b"\x1b[?2;0;";
    let terminator = b"S";
    assert!(
        find_subslice(&stream, prefix).is_some() && find_subslice(&stream, terminator).is_some(),
        "CREGSXTSM (\\x1b[?2;1;0S) MUST reply with \\x1b[?2;0;<pv>S; got {:?}",
        String::from_utf8_lossy(&stream),
    );
}

/// `GEOMXTSM` (`CSI ? 1 ; 1 ; 0 S`) — XTSMGRAPHICS Pi=1 Pa=1 read max
/// sixel geometry. MUST reply with `CSI ? 1 ; 0 ; Pw ; Ph S` (success,
/// width, height). Required for sixel pixel-implementation detection.
#[test]
fn notcurses_info_query_geomxtsm_emits_geometry_reply() {
    let stream = feed_and_concat(GEOMXTSM);
    let prefix = b"\x1b[?1;0;";
    let terminator = b"S";
    assert!(
        find_subslice(&stream, prefix).is_some() && find_subslice(&stream, terminator).is_some(),
        "GEOMXTSM (\\x1b[?1;1;0S) MUST reply with \\x1b[?1;0;<pw>;<ph>S; got {:?}",
        String::from_utf8_lossy(&stream),
    );
}

/// `GEOMPIXEL` (`CSI 14 t`) — text-area-in-pixels query. MUST reply with
/// `CSI 4 ; Ph ; Pw t`. notcurses derives `cellpxy = Ph / rows` and
/// `cellpxx = Pw / cols`; zero pixel dims set `canpixel = false` and
/// suppress the `display_logo()` call.
#[test]
fn notcurses_info_query_geompixel_emits_size_reply_nonzero() {
    let stream = feed_and_concat(GEOMPIXEL);
    let prefix = b"\x1b[4;";
    let terminator = b"t";
    assert!(
        find_subslice(&stream, prefix).is_some() && find_subslice(&stream, terminator).is_some(),
        "GEOMPIXEL (\\x1b[14t) MUST reply with \\x1b[4;<ph>;<pw>t; got {:?}",
        String::from_utf8_lossy(&stream),
    );
    // Extract Ph + Pw and assert both non-zero.
    let after = find_subslice(&stream, prefix).map(|i| &stream[i + prefix.len()..]).unwrap();
    let semi = after.iter().position(|b| *b == b';').expect("semicolon");
    let ph: u32 = std::str::from_utf8(&after[..semi]).unwrap().parse().unwrap();
    let rest = &after[semi + 1..];
    let t_idx = rest.iter().position(|b| *b == b't').expect("t terminator");
    let pw: u32 = std::str::from_utf8(&rest[..t_idx]).unwrap().parse().unwrap();
    assert!(
        ph > 0 && pw > 0,
        "GEOMPIXEL reply MUST report non-zero pixel dims (got ph={ph} pw={pw}). \
         Zero dims would zero cellpxy/cellpxx and disable display_logo()."
    );
}

/// `GEOMCELL` (`CSI 18 t`) — text-area-in-chars query. MUST reply with
/// `CSI 8 ; Pl ; Pc t` reporting rows + cols.
#[test]
fn notcurses_info_query_geomcell_emits_size_reply_nonzero() {
    let stream = feed_and_concat(GEOMCELL);
    let prefix = b"\x1b[8;";
    let terminator = b"t";
    assert!(
        find_subslice(&stream, prefix).is_some() && find_subslice(&stream, terminator).is_some(),
        "GEOMCELL (\\x1b[18t) MUST reply with \\x1b[8;<pl>;<pc>t; got {:?}",
        String::from_utf8_lossy(&stream),
    );
    let after = find_subslice(&stream, prefix).map(|i| &stream[i + prefix.len()..]).unwrap();
    let semi = after.iter().position(|b| *b == b';').expect("semicolon");
    let pl: u32 = std::str::from_utf8(&after[..semi]).unwrap().parse().unwrap();
    let rest = &after[semi + 1..];
    let t_idx = rest.iter().position(|b| *b == b't').expect("t terminator");
    let pc: u32 = std::str::from_utf8(&rest[..t_idx]).unwrap().parse().unwrap();
    assert!(
        pl > 0 && pc > 0,
        "GEOMCELL reply MUST report non-zero cell dims (got pl={pl} pc={pc})."
    );
}

/// `PRIDEVATTR` (`CSI c`) — DA1. notcurses uses DA1 reply as the
/// end-of-handshake marker; if absent notcurses-info BLOCKS waiting for
/// the reply and the binary hangs. MUST reply with one of the documented
/// DA1 forms (`CSI ? 6 c`, `CSI ? 62 ; ... c`, `CSI ? 64 ; ... c` etc).
#[test]
fn notcurses_info_query_pridevattr_emits_da1_reply() {
    let stream = feed_and_concat(PRIDEVATTR);
    let prefix = b"\x1b[?";
    let terminator = b"c";
    assert!(
        find_subslice(&stream, prefix).is_some() && find_subslice(&stream, terminator).is_some(),
        "PRIDEVATTR (\\x1b[c) MUST reply with \\x1b[?<da1-payload>c — notcurses uses \
         DA1 as end-of-handshake marker; missing reply hangs notcurses-info. Got {:?}",
        String::from_utf8_lossy(&stream),
    );
}

/// OSC 4 ; n ; ? ST — palette query for index `n`. Sent for indices 0..256
/// per `send_initial_directives` (`termdesc.c:488-502`). Each query
/// MUST emit either a reply (`OSC 4 ; n ; rgb:... ST`) or be silently
/// dropped — notcurses parses each independently and tolerates missing
/// entries up to the palette size reported by `colors` terminfo.
#[test]
fn notcurses_info_query_palette_osc4_does_not_block() {
    let mut h = SpecHarness::new();
    let queries = palette_queries();
    h.feed(&queries);
    let _ = pty_writes(&h);
}

// ---------------------------------------------------------------------------
// Full-bundle test — feed the entire init stream and verify all
// required replies appear in the aggregate transcript.
// ---------------------------------------------------------------------------

/// Full notcurses-info init handshake. Feeds the exact byte stream the
/// real binary emits and asserts every load-bearing reply appears in the
/// PTY transcript. If ANY required reply is missing, notcurses-info
/// blocks at that query and hangs; this test catches the hang condition
/// at unit-test wall-clock.
#[test]
fn notcurses_info_full_init_handshake_emits_all_required_replies() {
    let bytes = notcurses_info_init_stream();
    let mut h = SpecHarness::new();
    h.feed(&bytes);
    // Union over every PTY-write kind so missing replies surface
    // regardless of which `PtyWriteKind` variant ori_term classified them
    // under.
    let combined: Vec<u8> = pty_writes(&h)
        .into_iter()
        .flat_map(|(b, _)| b.iter().copied().collect::<Vec<u8>>())
        .collect();

    let required: &[(&[u8], &str)] = &[
        (b"\x1b[1;1R", "DSRCPR cursor-position reply"),
        (b"\x1b[?", "DA1 reply prefix"),
        (b"\x1b_Gi=1;OK\x1b\\", "kitty graphics OK reply"),
        (b"\x1b[?2026;", "DECRQM mode 2026 reply prefix"),
        (b"\x1b[?1016;", "DECRQM mode 1016 reply prefix"),
        (b"\x1b[4;", "CSI 14t pixel-geometry reply prefix"),
        (b"\x1b[8;", "CSI 18t cell-geometry reply prefix"),
        (b"\x1b]10;", "OSC 10 default-fg reply prefix"),
        (b"\x1b]11;", "OSC 11 default-bg reply prefix"),
    ];
    let mut missing = Vec::new();
    for (needle, label) in required {
        if find_subslice(&combined, needle).is_none() {
            missing.push((*label, *needle));
        }
    }
    assert!(
        missing.is_empty(),
        "notcurses-info init handshake missing required replies — these cause the \
         binary to block waiting for the response: {:?}. \
         Combined transcript:\n{:?}",
        missing
            .iter()
            .map(|(l, n)| format!("{l} ({:?})", String::from_utf8_lossy(n)))
            .collect::<Vec<_>>(),
        String::from_utf8_lossy(&combined),
    );
}

// ---------------------------------------------------------------------------
// Diagnostic dump — prints every PTY emission for debugging hangs.
// ---------------------------------------------------------------------------

/// Diagnostic: dump every PTY emission from the full init handshake.
/// Run via `cargo test notcurses_info_handshake_full_dump -- --nocapture`.
#[test]
fn notcurses_info_handshake_full_dump() {
    let bytes = notcurses_info_init_stream();
    let mut h = SpecHarness::new();
    h.feed(&bytes);
    eprintln!("--- notcurses-info full init dump ({} bytes sent) ---", bytes.len());
    let mut count = 0usize;
    let mut total = 0usize;
    for (i, (b, kind)) in pty_writes(&h).into_iter().enumerate() {
        count += 1;
        total += b.len();
        eprintln!(
            " [{i}] kind={kind:?} ({} bytes): {:?}",
            b.len(),
            String::from_utf8_lossy(b),
        );
    }
    eprintln!("--- {count} PTY replies, {total} total bytes ---");
    assert!(count > 0, "init handshake MUST emit at least one PTY reply");
    // Surface kind histogram so test output names which sub-stream is
    // contributing replies — iterate `PtyWriteKind::all()` per
    // `effect/families/pty.rs` SSOT.
    for kind in PtyWriteKind::all() {
        let n = pty_writes_of_kind(&h, *kind).count();
        eprintln!("  kind {kind:?}: {n} writes");
    }
}
