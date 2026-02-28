use super::wheel_scroll_lines;

#[test]
fn returns_positive_value() {
    let lines = wheel_scroll_lines();
    assert!(lines > 0, "scroll lines must be at least 1, got {lines}");
}

#[test]
fn returns_reasonable_range() {
    let lines = wheel_scroll_lines();
    // Windows allows up to 100 in the UI; anything beyond is unusual.
    assert!(lines <= 100, "scroll lines unexpectedly large: {lines}");
}
