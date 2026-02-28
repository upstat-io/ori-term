use super::GpuCompositor;

#[test]
fn direct_render_eligible_identity_full_opacity() {
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    assert!(GpuCompositor::is_direct_render_eligible(1.0, &identity));
}

#[test]
fn direct_render_ineligible_low_opacity() {
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    assert!(!GpuCompositor::is_direct_render_eligible(0.5, &identity));
}

#[test]
fn direct_render_ineligible_non_identity_transform() {
    let translated = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [10.0, 20.0, 1.0]];
    assert!(!GpuCompositor::is_direct_render_eligible(1.0, &translated));
}

#[test]
fn direct_render_ineligible_both() {
    let scaled = [[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 1.0]];
    assert!(!GpuCompositor::is_direct_render_eligible(0.8, &scaled));
}
