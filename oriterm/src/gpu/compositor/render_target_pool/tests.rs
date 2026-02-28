use super::round_up_to_bucket;

#[test]
fn bucket_rounds_up_to_power_of_two() {
    assert_eq!(round_up_to_bucket(1), 256);
    assert_eq!(round_up_to_bucket(100), 256);
    assert_eq!(round_up_to_bucket(256), 256);
    assert_eq!(round_up_to_bucket(257), 512);
    assert_eq!(round_up_to_bucket(512), 512);
    assert_eq!(round_up_to_bucket(513), 1024);
    assert_eq!(round_up_to_bucket(1024), 1024);
    assert_eq!(round_up_to_bucket(1025), 2048);
    assert_eq!(round_up_to_bucket(2048), 2048);
    assert_eq!(round_up_to_bucket(2049), 4096);
}

#[test]
fn bucket_minimum_is_256() {
    assert_eq!(round_up_to_bucket(0), 256);
    assert_eq!(round_up_to_bucket(1), 256);
    assert_eq!(round_up_to_bucket(128), 256);
    assert_eq!(round_up_to_bucket(255), 256);
}

#[test]
fn bucket_large_dimensions() {
    assert_eq!(round_up_to_bucket(4096), 4096);
    assert_eq!(round_up_to_bucket(4097), 8192);
    assert_eq!(round_up_to_bucket(8192), 8192);
    assert_eq!(round_up_to_bucket(8193), 16384);
}

#[test]
fn bucket_exact_powers_of_two() {
    // Every exact power of two from 256 up should return itself.
    for exp in 8..=14 {
        let val = 1u32 << exp;
        assert_eq!(round_up_to_bucket(val), val, "2^{exp} = {val}");
    }
}
