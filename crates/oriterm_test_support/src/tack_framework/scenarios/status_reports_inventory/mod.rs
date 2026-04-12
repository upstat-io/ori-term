//! Pinned curated subset of sub-tests in tack's
//! `s) ANSI status reports` sub-submenu.
//!
//! Unlike the flat menu inventories in [`super::begin_testing_inventory`]
//! and [`super::tools_menu_inventory`], the status reports
//! sub-submenu is SEQUENTIAL and batched irregularly.
//!
//! One `n` key press does NOT consistently advance one sub-test.
//! Tack's batching depends on the terminal's responses to the
//! preceding `CSI` probes: terminals that respond correctly to
//! every query allow tack to run multiple sub-tests without
//! pausing, while terminals that produce "illegal responses"
//! force tack to stop between each.
//!
//! The Section 06.0.b integration test walks the sub-submenu via
//! direct `PtySession` send/drain calls (not via
//! `ScenarioSpec::menu_path`, because per-step `wait_for` anchors
//! cannot handle tack's batching) and accumulates every sub-test
//! label observed across the walk. It asserts that every label in
//! [`STATUS_REPORTS_INVENTORY`] was observed at least once — a
//! subset-presence check that is robust to tack's batching quirks.
//!
//! # Curated inventory (not exhaustive)
//!
//! Empirical discovery against tack v1.08 + `ori_term` terminfo
//! (2026-04-08) found **33 total sub-tests**.
//!
//! The shell probe against xterm-256color only found 32 because
//! xterm's `CPR` response mediation caused tack to skip the
//! "Screen size" sub-test at index 3. Per Section 06's REVISIT
//! clause (the plan's "if sub-test count grows past ~15, revisit"
//! rule), the inventory pins a CURATED subset of 8 key sub-tests
//! that Section 06.1 cross-references against `extra/ori_term.info`
//! caps `u6/u7/u8/u9`, plus a length-drift gate that asserts
//! [`LENGTH_DRIFT_FINAL_LABEL`] is observed (catching insertions or
//! removals at any index in the full 33-step walk without paying
//! the per-sub-test scenario cost).
//!
//! # Index field semantics
//!
//! The `index` field on [`StatusReportsSubTest`] is informational
//! only for the 06.0.b integration test.
//!
//! The test uses subset-presence assertions, not index-based
//! walking, because tack's `n` batching makes index-based walking
//! impossible. The `index` field IS load-bearing for the unit
//! tests in this module, which assert ascending-order invariants
//! and pin the final index to `TOTAL_SUB_TESTS - 1` as an
//! anti-drift guard.
//!
//! # Cap-coverage traceability
//!
//! The curated 8 entries cover the `DA`-family (`DA1`/`DA2`/`DA3`)
//! and the key `DSR` variants (terminal status, cursor position).
//!
//! Section 06.1 will cross-reference these against
//! `extra/ori_term.info` caps `u6/u7/u8/u9`. `DECRQSS` bounds
//! (first + last) guard the plan's coverage claim that "Section
//! 06.1 covers `DECRQSS` query round-trip" — adding 18 `DECRQSS`
//! middle entries would not improve cap coverage because
//! `ori_term`'s `DECRQSS` handler is generic across the sub-protocol.

/// One row of the curated status-reports inventory.
///
/// Each entry represents a single sub-test whose label must appear
/// in the integration test's accumulated walk grid. The `index`
/// field is informational (tack's batching prevents index-based
/// walking) but the final-index invariant is enforced by the unit
/// tests in this module.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatusReportsSubTest {
    /// 0-based index in tack's sub-submenu walk under `ori_term`
    /// terminfo. Informational for the integration test; enforced
    /// at the last-entry position by the unit tests.
    pub index: usize,
    /// Unique substring of tack's header label for this sub-test,
    /// as captured in the discovery walk. The substring MUST be
    /// unique enough that a substring match on the accumulated walk
    /// grid cannot false-positive against another sub-test's label
    /// (e.g. `"Cursor position"` vs `"Extended cursor position"` —
    /// choose the longer form if the shorter would collide).
    pub label: &'static str,
    /// Short mnemonic for the sub-test. Used by Section 06.1 to
    /// name its `#[test] fn`s and by the integration test's failure
    /// diagnostics.
    pub mnemonic: &'static str,
    /// The terminfo cap this sub-test exercises (if any), used by
    /// Section 06.1 to decide which response-field tokens to
    /// extract from the captured grid via `grid_find_field`.
    /// `None` for DECRQSS/DECRQDE entries that are generic
    /// protocol probes with no single cap in `extra/ori_term.info`.
    pub cap_target: Option<&'static str>,
}

/// The curated pinned inventory. 8 entries selected from tack
/// v1.08's 33-sub-test walk under `ori_term` terminfo.
///
/// Ordering mirrors tack's walk order (ascending by `index`).
pub const STATUS_REPORTS_INVENTORY: &[StatusReportsSubTest] = &[
    StatusReportsSubTest {
        index: 0,
        label: "Primary device attributes",
        mnemonic: "da1",
        cap_target: Some("u6"),
    },
    StatusReportsSubTest {
        index: 1,
        label: "Terminal status",
        mnemonic: "dsr_status",
        cap_target: None,
    },
    StatusReportsSubTest {
        index: 2,
        label: "Cursor position",
        mnemonic: "dsr_cpr",
        cap_target: Some("u7"),
    },
    StatusReportsSubTest {
        index: 4,
        label: "Secondary device attributes",
        mnemonic: "da2",
        cap_target: None,
    },
    StatusReportsSubTest {
        index: 8,
        label: "Data destination",
        mnemonic: "decrqss_first",
        cap_target: None,
    },
    StatusReportsSubTest {
        index: 18,
        label: "Tertiary device attributes",
        mnemonic: "da3",
        cap_target: None,
    },
    StatusReportsSubTest {
        index: 31,
        label: "Select modifier key reporting",
        mnemonic: "decrqss_last",
        cap_target: None,
    },
    StatusReportsSubTest {
        index: 32,
        label: "Window report",
        mnemonic: "decrqde_window",
        cap_target: None,
    },
];

/// The expected label at the FINAL index (32) of tack v1.08's
/// status-reports sub-submenu walk under `ori_term` terminfo.
///
/// The length-drift gate asserts this label is observed in the
/// accumulated walk grid. Insertions or removals anywhere in the
/// 33-entry sequence will cause a different sub-test to land at
/// the final index (or tack to fall off the end early) — either
/// case is caught by the assertion.
pub const LENGTH_DRIFT_FINAL_LABEL: &str = "Window report";

/// The total number of sub-tests tack v1.08 emits from the
/// `s) ANSI status reports` sub-submenu under `ori_term` terminfo.
///
/// Empirically measured on 2026-04-08 via a direct `PtySession`
/// walk probe. xterm-256color's 32-sub-test count is terminfo-
/// dependent: the extra sub-test under `ori_term` is
/// `(DSR) Screen size (CSI 6 n)` at index 3, which tack only emits
/// when it got a valid `CPR` response from the preceding
/// `(DSR) Cursor position` sub-test.
///
/// A future tack release that changes this count will cause the
/// length-drift scenario to either find a different label at the
/// final index or return to the tools menu early — both cases
/// fail the gate.
pub const TOTAL_SUB_TESTS: usize = 33;

#[cfg(test)]
mod tests;
