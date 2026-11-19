use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use cache::{Cache, CacheError, MemoryCacheStore};

#[tokio::test]
async fn test_singleflight_stampede_defense() {
    let store = Arc::new(MemoryCacheStore::new());
    let cache = Arc::new(Cache::new(store));

    let execution_count = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..20 {
        let cache_clone = Arc::clone(&cache);
        let exec_count = Arc::clone(&execution_count);

        handles.push(tokio::spawn(async move {
            cache_clone
                .remember(
                    "stampede_key",
                    Duration::from_secs(60),
                    move || async move {
                        // Small delay to simulate database fetch and ensure concurrency collision.
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        exec_count.fetch_add(1, Ordering::SeqCst);
                        Ok::<String, CacheError>("computed_result".to_string())
                    },
                )
                .await
        }));
    }

    for handle in handles {
        let result = handle.await.unwrap().unwrap();
        assert_eq!(result, "computed_result");
    }

    // Crucial assertion: factory executed exactly ONCE across all 20 concurrent requests!
    assert_eq!(execution_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_singleflight_failure_does_not_poison_subsequent_calls() {
    let store = Arc::new(MemoryCacheStore::new());
    let cache = Arc::new(Cache::new(store));

    // 1. Initial execution fails.
    let res = cache
        .remember("error_key", Duration::from_secs(60), || async {
            Err::<String, CacheError>(CacheError::Store("upstream db timeout".to_string()))
        })
        .await;
    assert!(res.is_err());

    // 2. Subsequent call succeeds without lock poisoning or hanging.
    let recovered = cache
        .remember("error_key", Duration::from_secs(60), || async {
            Ok::<String, CacheError>("recovered_value".to_string())
        })
        .await
        .unwrap();
    assert_eq!(recovered, "recovered_value");
}

#[tokio::test]
async fn test_singleflight_concurrent_followers_receive_error() {
    let store = Arc::new(MemoryCacheStore::new());
    let cache = Arc::new(Cache::new(store));

    let mut handles = Vec::new();
    for _ in 0..5 {
        let cache_clone = Arc::clone(&cache);
        handles.push(tokio::spawn(async move {
            cache_clone
                .remember("fail_key", Duration::from_secs(60), || async {
                    tokio::time::sleep(Duration::from_millis(30)).await;
                    Err::<String, CacheError>(CacheError::Store("concurrent error".to_string()))
                })
                .await
        }));
    }

    for handle in handles {
        let res = handle.await.unwrap();
        assert!(res.is_err());
    }
}
