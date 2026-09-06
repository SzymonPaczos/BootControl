//! Daemon activity tracking used by the socket-activated idle lifecycle.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::watch;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ActivityState {
    active_operations: usize,
    generation: u64,
}

struct Inner {
    state: Mutex<ActivityState>,
    changes: watch::Sender<ActivityState>,
}

/// Shared record of D-Bus traffic and currently running method calls.
#[derive(Clone)]
pub struct ActivityTracker {
    inner: Arc<Inner>,
}

impl Default for ActivityTracker {
    fn default() -> Self {
        let state = ActivityState::default();
        let (changes, _) = watch::channel(state);
        Self {
            inner: Arc::new(Inner {
                state: Mutex::new(state),
                changes,
            }),
        }
    }
}

impl ActivityTracker {
    /// Start an operation that keeps the daemon alive until its guard is dropped.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrold::lifecycle::ActivityTracker;
    ///
    /// let tracker = ActivityTracker::default();
    /// let operation = tracker.begin_operation();
    /// assert_eq!(tracker.active_operations(), 1);
    /// drop(operation);
    /// assert_eq!(tracker.active_operations(), 0);
    /// ```
    pub fn begin_operation(&self) -> OperationGuard {
        self.update(|state| {
            state.active_operations = state.active_operations.saturating_add(1);
        });
        OperationGuard {
            tracker: self.clone(),
            active: true,
        }
    }

    /// Record connection traffic that restarts the idle period.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrold::lifecycle::ActivityTracker;
    ///
    /// let tracker = ActivityTracker::default();
    /// tracker.record_activity();
    /// assert_eq!(tracker.active_operations(), 0);
    /// ```
    pub fn record_activity(&self) {
        self.update(|_| {});
    }

    /// Return the number of method calls currently protected from idle exit.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrold::lifecycle::ActivityTracker;
    ///
    /// let tracker = ActivityTracker::default();
    /// assert_eq!(tracker.active_operations(), 0);
    /// ```
    pub fn active_operations(&self) -> usize {
        self.lock_state().active_operations
    }

    /// Wait for a complete inactive period with no active operation or traffic.
    ///
    /// The countdown starts only while the active-operation count is zero. Any
    /// operation boundary or recorded D-Bus activity restarts the full period.
    ///
    /// # Arguments
    ///
    /// * `idle_timeout` - Required uninterrupted inactive duration.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use bootcontrold::lifecycle::ActivityTracker;
    ///
    /// let runtime = tokio::runtime::Runtime::new().unwrap();
    /// runtime.block_on(async {
    ///     ActivityTracker::default()
    ///         .wait_for_idle(Duration::from_millis(1))
    ///         .await;
    /// });
    /// ```
    pub async fn wait_for_idle(&self, idle_timeout: Duration) {
        let mut changes = self.inner.changes.subscribe();
        loop {
            let observed = *changes.borrow_and_update();
            if observed.active_operations > 0 {
                if changes.changed().await.is_err() {
                    return;
                }
                continue;
            }

            tokio::select! {
                _ = tokio::time::sleep(idle_timeout) => {
                    let current = *changes.borrow();
                    if current.active_operations == 0
                        && current.generation == observed.generation
                    {
                        return;
                    }
                }
                changed = changes.changed() => {
                    if changed.is_err() {
                        return;
                    }
                }
            }
        }
    }

    fn finish_operation(&self) {
        self.update(|state| {
            debug_assert!(state.active_operations > 0);
            state.active_operations = state.active_operations.saturating_sub(1);
        });
    }

    fn update(&self, mutation: impl FnOnce(&mut ActivityState)) {
        let mut state = self.lock_state();
        mutation(&mut state);
        state.generation = state.generation.wrapping_add(1);
        self.inner.changes.send_replace(*state);
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, ActivityState> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// RAII token that marks one daemon method as active.
pub struct OperationGuard {
    tracker: ActivityTracker,
    active: bool,
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        if self.active {
            self.tracker.finish_operation();
            self.active = false;
        }
    }
}
