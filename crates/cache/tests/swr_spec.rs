use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};

use cache::{Cache, CacheError, MemoryCacheStore};

#[tokio::test]
async fn test_stale_while_revalidate_grace_period() {
    let clock_time = Arc::new(AtomicU64::new(1000));
    let clock_clone = Arc::clone(&clock_time);

    let store = Arc::new(MemoryCacheStore::with_clock(1000, move || {
        clock_clone.load(Ordering::Relaxed)
    }));

    let clock_clone2 = Arc::clone(&clock_time);
    let cache = Cache::new(store)
        .with_default_ttl(Duration::from_millis(500))
        .with_grace_period(Duration::from_millis(1000))
        .with_clock(move || clock_clone2.load(Ordering::Relaxed));

    let refresh_count = Arc::new(AtomicUsize::new(0));

    // 1. Initial write: TTL 500ms, Grace 1000ms. (Logically expires at 1500ms, physically at 2500ms).
    let initial: String = cache
        .remember("swr_item", Duration::from_millis(500), || async {
            Ok::<String, CacheError>("version_1".to_string())
        })
        .await
        .unwrap();
    assert_eq!(initial, "version_1");

    // 2. Read at 1200ms: fresh hit.
    clock_time.store(1200, Ordering::Relaxed);
    let v_fresh: String = cache.get("swr_item").await.unwrap().unwrap();
    assert_eq!(v_fresh, "version_1");

    // 3. Advance clock to 1800ms: Logically expired, but within grace period!
    clock_time.store(1800, Ordering::Relaxed);

    let count_ref = Arc::clone(&refresh_count);
    let served_stale: String = cache
        .remember("swr_item", Duration::from_millis(500), move || async move {
            count_ref.fetch_add(1, Ordering::SeqCst);
            Ok::<String, CacheError>("version_2".to_string())
        })
        .await
        .unwrap();

    // Serves stale "version_1" immediately to avoid latency penalty.
    assert_eq!(served_stale, "version_1");

    // Allow background revalidation task to finish.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify background factory ran.
    assert_eq!(refresh_count.load(Ordering::SeqCst), 1);

    // Subsequent read sees the freshly revalidated "version_2".
    let updated: String = cache.get("swr_item").await.unwrap().unwrap();
    assert_eq!(updated, "version_2");

    // 4. Advance clock past physical expiration (e.g. 5000ms).
    clock_time.store(5000, Ordering::Relaxed);
    let expired: Option<String> = cache.get("swr_item").await.unwrap();
    assert!(expired.is_none());
}
