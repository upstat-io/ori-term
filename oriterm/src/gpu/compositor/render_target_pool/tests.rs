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
