use super::{align_up, write_layer_uniform_raw};

#[test]
fn align_up_basic() {
    assert_eq!(align_up(80, 256), 256);
    assert_eq!(align_up(256, 256), 256);
    assert_eq!(align_up(257, 256), 512);
    assert_eq!(align_up(1, 1), 1);
    assert_eq!(align_up(0, 256), 0);
}

#[test]
fn write_layer_uniform_identity_transform() {
    let transform = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let bounds = [10.0, 20.0, 100.0, 50.0];
    let opacity = 0.75;

    let mut buf = vec![0u8; 80];
    write_layer_uniform_raw(&mut buf, &transform, &bounds, opacity);

    // Column 0: [1.0, 0.0, 0.0, pad]
    assert_eq!(f32::from_le_bytes(buf[0..4].try_into().unwrap()), 1.0);
    assert_eq!(f32::from_le_bytes(buf[4..8].try_into().unwrap()), 0.0);
    assert_eq!(f32::from_le_bytes(buf[8..12].try_into().unwrap()), 0.0);
    // Padding at 12..16 should be zero.
    assert_eq!(&buf[12..16], &[0, 0, 0, 0]);

    // Column 1: [0.0, 1.0, 0.0, pad]
    assert_eq!(f32::from_le_bytes(buf[16..20].try_into().unwrap()), 0.0);
    assert_eq!(f32::from_le_bytes(buf[20..24].try_into().unwrap()), 1.0);
    assert_eq!(f32::from_le_bytes(buf[24..28].try_into().unwrap()), 0.0);

    // Column 2: [0.0, 0.0, 1.0, pad]
    assert_eq!(f32::from_le_bytes(buf[32..36].try_into().unwrap()), 0.0);
    assert_eq!(f32::from_le_bytes(buf[36..40].try_into().unwrap()), 0.0);
    assert_eq!(f32::from_le_bytes(buf[40..44].try_into().unwrap()), 1.0);

    // Bounds at offset 48.
    assert_eq!(f32::from_le_bytes(buf[48..52].try_into().unwrap()), 10.0);
    assert_eq!(f32::from_le_bytes(buf[52..56].try_into().unwrap()), 20.0);
    assert_eq!(f32::from_le_bytes(buf[56..60].try_into().unwrap()), 100.0);
    assert_eq!(f32::from_le_bytes(buf[60..64].try_into().unwrap()), 50.0);

    // Opacity at offset 64.
    assert_eq!(f32::from_le_bytes(buf[64..68].try_into().unwrap()), 0.75);

    // Trailing padding should be zero.
    assert_eq!(&buf[68..80], &[0u8; 12]);
}

#[test]
fn write_layer_uniform_translation() {
    // Transform with a 50px X, 30px Y translation.
    let transform = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [50.0, 30.0, 1.0]];
    let bounds = [0.0, 0.0, 200.0, 100.0];
    let opacity = 1.0;

    let mut buf = vec![0u8; 80];
    write_layer_uniform_raw(&mut buf, &transform, &bounds, opacity);

    // Column 2 should contain the translation.
    assert_eq!(f32::from_le_bytes(buf[32..36].try_into().unwrap()), 50.0);
    assert_eq!(f32::from_le_bytes(buf[36..40].try_into().unwrap()), 30.0);
    assert_eq!(f32::from_le_bytes(buf[40..44].try_into().unwrap()), 1.0);

    // Opacity.
    assert_eq!(f32::from_le_bytes(buf[64..68].try_into().unwrap()), 1.0);
}
