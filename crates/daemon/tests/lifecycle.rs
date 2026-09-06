#![cfg(target_os = "linux")]

use std::time::Duration;

use bootcontrold::lifecycle::ActivityTracker;

#[tokio::test(start_paused = true)]
async fn active_operation_can_outlive_timeout_then_gets_a_full_idle_period() {
    let tracker = ActivityTracker::default();
    let operation = tracker.begin_operation();
    let waiter_tracker = tracker.clone();
    let waiter = tokio::spawn(async move {
        waiter_tracker.wait_for_idle(Duration::from_secs(1)).await;
    });
    tokio::task::yield_now().await;

    tokio::time::advance(Duration::from_secs(5)).await;
    assert!(!waiter.is_finished());

    drop(operation);
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_millis(999)).await;
    assert!(!waiter.is_finished());
    tokio::time::advance(Duration::from_millis(1)).await;
    tokio::task::yield_now().await;
    assert!(waiter.is_finished());
}

#[tokio::test(start_paused = true)]
async fn dbus_activity_restarts_the_full_idle_period() {
    let tracker = ActivityTracker::default();
    let waiter_tracker = tracker.clone();
    let waiter = tokio::spawn(async move {
        waiter_tracker.wait_for_idle(Duration::from_secs(1)).await;
    });
    tokio::task::yield_now().await;

    tokio::time::advance(Duration::from_millis(900)).await;
    tracker.record_activity();
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_millis(900)).await;
    assert!(!waiter.is_finished());
    tokio::time::advance(Duration::from_millis(100)).await;
    tokio::task::yield_now().await;
    assert!(waiter.is_finished());
}

#[tokio::test]
async fn error_and_cancellation_release_operation_guards() {
    async fn controlled_error(tracker: ActivityTracker) -> Result<(), &'static str> {
        let _operation = tracker.begin_operation();
        Err("controlled failure")
    }

    let tracker = ActivityTracker::default();
    assert_eq!(
        controlled_error(tracker.clone()).await,
        Err("controlled failure")
    );
    assert_eq!(tracker.active_operations(), 0);

    let cancelled_tracker = tracker.clone();
    let task = tokio::spawn(async move {
        let _operation = cancelled_tracker.begin_operation();
        std::future::pending::<()>().await;
    });
    tokio::task::yield_now().await;
    assert_eq!(tracker.active_operations(), 1);
    task.abort();
    let _ = task.await;
    assert_eq!(tracker.active_operations(), 0);
}

#[test]
fn overlapping_operations_keep_tracker_active_until_the_last_drop() {
    let tracker = ActivityTracker::default();
    let first = tracker.begin_operation();
    let second = tracker.begin_operation();
    assert_eq!(tracker.active_operations(), 2);

    drop(first);
    assert_eq!(tracker.active_operations(), 1);
    drop(second);
    assert_eq!(tracker.active_operations(), 0);
}
