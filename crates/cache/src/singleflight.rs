//! Single-flight stampede defense coordinator preventing concurrent duplicate computations.

use std::{
    collections::HashMap,
    future::Future,
    sync::{Arc, Mutex},
};

use tokio::sync::watch;

use crate::error::CacheError;

type InFlightChannel = watch::Receiver<Option<Result<String, String>>>;
type InFlightRegistry = Arc<Mutex<HashMap<String, InFlightChannel>>>;

struct FlightGuard {
    key: String,
    in_flight: InFlightRegistry,
}

impl Drop for FlightGuard {
    fn drop(&mut self) {
        if let Ok(mut map) = self.in_flight.lock() {
            map.remove(&self.key);
        }
    }
}

/// Coordinates in-flight asynchronous operations to prevent cache stampedes.
#[derive(Clone, Default)]
pub struct SingleFlight {
    in_flight: InFlightRegistry,
}

impl SingleFlight {
    /// Creates a new single-flight coordinator.
    #[must_use]
    pub fn new() -> Self {
        Self {
            in_flight: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Executes `future` for `key` if no other execution is in-flight.
    ///
    /// If an execution is already active for `key`, this call waits for that execution to complete
    /// and returns a clone of the computed result without executing `future`.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if the underlying operation fails or is aborted.
    pub async fn execute<F, Fut>(&self, key: &str, future: F) -> Result<String, CacheError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<String, CacheError>>,
    {
        // 1. Check if an execution is already in flight for this key.
        // Lock is strictly held only to register or retrieve the channel, never across await.
        let (maybe_receiver, maybe_sender) = {
            let mut map = self.in_flight.lock().map_err(|_| {
                CacheError::LockPoisoned("SingleFlight registry poisoned".to_string())
            })?;

            if let Some(receiver) = map.get(key) {
                (Some(receiver.clone()), None)
            } else {
                let (sender, receiver) = watch::channel(None);
                map.insert(key.to_string(), receiver);
                (None, Some(sender))
            }
        };

        // 2. Leader path: execute the future and broadcast to waiting followers.
        if let Some(sender) = maybe_sender {
            let guard = FlightGuard {
                key: key.to_string(),
                in_flight: Arc::clone(&self.in_flight),
            };

            let execution_result = future().await;
            match &execution_result {
                Ok(val) => {
                    let drop_result = sender.send(Some(Ok(val.clone())));
                    drop(drop_result);
                }
                Err(error) => {
                    let drop_result = sender.send(Some(Err(error.to_string())));
                    drop(drop_result);
                }
            }
            drop(guard);
            return execution_result;
        }

        // 3. Follower path: wait on the leader's receiver.
        let mut follower_receiver = maybe_receiver.unwrap();
        loop {
            if let Some(ref result) = *follower_receiver.borrow() {
                return match result {
                    Ok(value) => Ok(value.clone()),
                    Err(message) => Err(CacheError::Store(message.clone())),
                };
            }

            if follower_receiver.changed().await.is_err() {
                return Err(CacheError::Store(
                    "In-flight single-flight leader was dropped before completion".to_string(),
                ));
            }
        }
    }
}
