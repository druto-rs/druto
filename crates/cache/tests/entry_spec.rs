use std::time::Duration;

use cache::{CacheEntry, current_unix_timestamp_ms};

#[test]
fn test_cache_entry_methods_and_errors() {
    let now = current_unix_timestamp_ms();
    let entry = CacheEntry::from_value_with_clock(
        &"data",
        Duration::from_millis(500),
        Duration::from_millis(500),
        vec!["tag1".to_string()],
        now,
    )
    .unwrap();

    // Fresh at creation.
    assert!(!entry.is_logically_expired(now));
    assert!(!entry.is_physically_expired(now));
    assert!(!entry.is_in_grace_period(now));

    // Logically expired, in grace period at now + 600ms.
    assert!(entry.is_logically_expired(now + 600));
    assert!(!entry.is_physically_expired(now + 600));
    assert!(entry.is_in_grace_period(now + 600));

    // Physically expired at now + 1200ms.
    assert!(entry.is_physically_expired(now + 1200));
    assert!(!entry.is_in_grace_period(now + 1200));

    // Storage serialization roundtrip.
    let serialized = entry.to_storage_string().unwrap();
    let deserialized = CacheEntry::from_storage_str(&serialized).unwrap();
    assert_eq!(entry, deserialized);

    // Deserialization failure into wrong type.
    assert!(entry.deserialize::<i32>().is_err());
    assert!(CacheEntry::from_storage_str("not json").is_err());
}
