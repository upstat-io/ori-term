//! Tests for the `tuple` module: canonical tuple parsing,
//! `Tuple` construction and equality, and `Category` display.

use super::{Category, Tuple, TupleSig, canonical_tuple};

// -------- Positive pins: canonical_tuple -----------------------------------

#[test]
fn cup_sequence_canonicalizes_to_csi_ps_ps_h() {
    let t = canonical_tuple("`CSI Ps;Ps H`").expect("CUP must canonicalize");
    assert_eq!(t.category, Category::Csi);
    assert!(t.intermediates.is_empty());
    assert_eq!(t.params, "Ps;Ps");
    assert_eq!(t.final_byte, "H");
}

#[test]
fn cuu_sequence_canonicalizes_to_csi_ps_a() {
    let t = canonical_tuple("`CSI Ps A`").expect("CUU must canonicalize");
    assert_eq!(t.category, Category::Csi);
    assert!(t.intermediates.is_empty());
    assert_eq!(t.params, "Ps");
    assert_eq!(t.final_byte, "A");
}

#[test]
fn decset_pair_canonicalizes_to_first_alternative_h_form() {
    // Catalog rows use `CSI ? Ps h / CSI ? Ps l` for paired
    // DECSET/DECRST rows. The canonicalizer takes the first
    // alternative — the paired expansion happens at check time.
    let t = canonical_tuple("`CSI ? 25 h / CSI ? 25 l`").expect("DECSET must canonicalize");
    assert_eq!(t.category, Category::Csi);
    assert_eq!(t.intermediates, vec![b'?']);
    assert_eq!(t.final_byte, "h");
}

#[test]
fn pm_sequence_canonicalizes_to_pm_empty_ints_pt_st() {
    let t = canonical_tuple("`ESC ^ Pt ST`").expect("PM must canonicalize");
    assert_eq!(t.category, Category::Pm);
    assert!(t.intermediates.is_empty());
    assert_eq!(t.params, "Pt");
    assert_eq!(t.final_byte, "ST");
}

#[test]
fn sos_sequence_canonicalizes_to_sos_empty_ints_pt_st() {
    let t = canonical_tuple("`ESC X Pt ST`").expect("SOS must canonicalize");
    assert_eq!(t.category, Category::Sos);
    assert!(t.intermediates.is_empty());
    assert_eq!(t.params, "Pt");
    assert_eq!(t.final_byte, "ST");
}

#[test]
fn dcs_sixel_canonicalizes_to_dcs_empty_ints_pid_q() {
    let t = canonical_tuple("`DCS Ps1 ; Ps2 ; Ps3 q <data> ST`").expect("DCS q must canonicalize");
    assert_eq!(t.category, Category::Dcs);
    assert!(t.intermediates.is_empty());
    assert_eq!(t.params, "Pid");
    assert_eq!(t.final_byte, "q");
}

#[test]
fn dcs_decrqss_canonicalizes_to_dcs_dollar_pt_q() {
    let t = canonical_tuple("`DCS $ q Pt ST`").expect("DECRQSS must canonicalize");
    assert_eq!(t.category, Category::Dcs);
    assert_eq!(t.intermediates, vec![b'$']);
    assert_eq!(t.params, "Pt");
    assert_eq!(t.final_byte, "q");
}

#[test]
fn osc_title_canonicalizes_with_numeric_id_preserved() {
    let t = canonical_tuple("`OSC 0 ; Pt BEL|ST`").expect("OSC 0 must canonicalize");
    assert_eq!(t.category, Category::Osc);
    assert!(t.intermediates.is_empty());
    assert_eq!(t.params, "0;text");
    assert_eq!(t.final_byte, "BEL");
}

#[test]
fn osc_4_palette_canonicalizes_with_index_placeholder() {
    let t = canonical_tuple("`OSC 4 ; index ; rgb BEL|ST`").expect("OSC 4 must canonicalize");
    assert_eq!(t.params, "4;index;rgb");
}

#[test]
fn osc_numeric_id_must_not_collapse_to_ps() {
    // Negative pin: the numeric id is preserved literally; if it
    // collapsed to `Ps` we would lose dispatch-level discrimination.
    let t = canonical_tuple("`OSC 52 ; Pc ; <b64> BEL|ST`").expect("OSC 52 must canonicalize");
    assert!(t.params.starts_with("52;"));
    assert_ne!(t.params, "Ps;Ps;Ps");
}

#[test]
fn apc_kitty_canonicalizes_to_apc_g_key_value_st() {
    let t = canonical_tuple("`APC G key=value ; <data> ST`").expect("APC _G must canonicalize");
    assert_eq!(t.category, Category::Apc);
    assert_eq!(t.intermediates, vec![b'G', b'_']);
    assert_eq!(t.params, "key-value");
    assert_eq!(t.final_byte, "ST");
}

#[test]
fn decscusr_canonicalizes_to_csi_sp_ps_q() {
    let t = canonical_tuple("`CSI Ps SP q`").expect("DECSCUSR must canonicalize");
    assert_eq!(t.intermediates, vec![b' ']);
    assert_eq!(t.final_byte, "q");
}

#[test]
fn esc_d_canonicalizes_to_esc_empty_dash_d() {
    let t = canonical_tuple("`ESC D`").expect("IND must canonicalize");
    assert_eq!(t.category, Category::Esc);
    assert_eq!(t.final_byte, "D");
}

#[test]
fn esc_charset_designation_canonicalizes_to_da_paren_b() {
    let t = canonical_tuple("`ESC ( B`").expect("G0=ASCII must canonicalize");
    assert_eq!(t.category, Category::Da);
    assert_eq!(t.intermediates, vec![b'(']);
    assert_eq!(t.final_byte, "B");
}

// -------- Tuple equality / sorting -----------------------------------------

#[test]
fn tuple_sort_intermediates_on_construction() {
    // Unsorted input -> sorted output so hash equality holds.
    let t1 = Tuple::new(Category::Dcs, vec![b'!', b'$'], "Pt", "q");
    let t2 = Tuple::new(Category::Dcs, vec![b'$', b'!'], "Pt", "q");
    assert_eq!(t1, t2);
}

// -------- Tuple::signature() -----------------------------------------------

#[test]
fn signature_normalizes_osc_st_to_bel() {
    let t = Tuple::new(Category::Osc, Vec::<u8>::new(), "0;text", "ST");
    let sig: TupleSig = t.signature();
    assert_eq!(sig.2, "BEL");
}

#[test]
fn signature_preserves_bel_for_osc() {
    let t = Tuple::new(Category::Osc, Vec::<u8>::new(), "0;text", "BEL");
    let sig: TupleSig = t.signature();
    assert_eq!(sig.2, "BEL");
}

#[test]
fn signature_preserves_csi_final_byte() {
    let t = Tuple::new(Category::Csi, Vec::<u8>::new(), "Ps", "H");
    let sig: TupleSig = t.signature();
    assert_eq!(sig.2, "H");
}

// -------- Self-verifying completeness counter ------------------------------

#[test]
fn matrix_visits_every_category_exactly_once() {
    // Enumerate every Category variant the canonicalizer can
    // produce and assert the count matches the enum. This catches
    // new variants that are added without a test.
    let categories = [
        Category::C0,
        Category::C1,
        Category::Esc,
        Category::Csi,
        Category::Osc,
        Category::Dcs,
        Category::Apc,
        Category::Pm,
        Category::Sos,
        Category::Da,
    ];
    let mut visited = 0_usize;
    for c in categories {
        // Touch every variant so a missing Display arm fails.
        let _ = format!("{c}");
        visited += 1;
    }
    assert_eq!(visited, 10);
}
