//! The real System-2 worker keeps runtime tasks running while its executor blocks a thread.
use super::failure_diagnostic::{fixture, settled_slow, Fixture};
use super::*;

/// Longer than the one-second TUNNELLING_ACK window (Tunnelling v01.07.01 AS §2.6.1).
const STALL: Duration = Duration::from_millis(1500);

/// One runtime worker, as in a container limited to one CPU. A synchronous
/// stall at the first mutation stands in for the durable checkpoint written
/// just before it; a spawned ticker stands in for the tunnel's receive task.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[ignore = "requires digest-pinned public package; set KNXBENCH_PUBLIC_UP258; no network or hardware"]
async fn a_blocking_executor_checkpoint_leaves_tunnel_tasks_running() {
    let Fixture {
        directory: _directory,
        hardware,
        state,
        router,
        device,
    } = fixture().await;
    let ticks = Arc::new(AtomicUsize::new(0));
    let ticker = tokio::spawn({
        let ticks = ticks.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(5)).await;
                ticks.fetch_add(1, Ordering::SeqCst);
            }
        }
    });
    *hardware.mutation_stall.lock().unwrap() = Some((STALL, ticks));
    let preview = prepare(&router, device).await;
    http(
        &router,
        "/api/device-system2/start",
        Some(start(&preview)),
        200,
    )
    .await;
    settled_slow(&state, &router, "finished").await;
    ticker.abort();
    let (before, after) = hardware
        .stall_ticks
        .lock()
        .unwrap()
        .expect("the executor reached its first mutation");
    assert!(
        after - before >= 10,
        "runtime tasks starved during the executor stall: {before} -> {after}"
    );
    assert!(hardware.writes.load(Ordering::SeqCst) > 0);
}

/// The same for the explicit recovery replay: a failed download leaves a
/// backup, and the stall is armed only for the recovery's first mutation.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[ignore = "requires digest-pinned public package; set KNXBENCH_PUBLIC_UP258; no network or hardware"]
async fn a_blocking_recovery_checkpoint_leaves_tunnel_tasks_running() {
    let Fixture {
        directory: _directory,
        hardware,
        state,
        router,
        device,
    } = fixture().await;
    // Object 1's second PID 7 read, after the first mutation, has two elements.
    hardware.arm_pid7(1, 1, 2, &[0xB7, 0xE5]);
    let preview = prepare(&router, device).await;
    let started = http(
        &router,
        "/api/device-system2/start",
        Some(start(&preview)),
        200,
    )
    .await;
    settled_slow(&state, &router, "failed").await;
    hardware.disarm_pid7();
    let failed = http(&router, "/api/device-system2/status", None, 200).await;
    assert_eq!(failed["backupAvailable"], true);
    let ticks = Arc::new(AtomicUsize::new(0));
    let ticker = tokio::spawn({
        let ticks = ticks.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(5)).await;
                ticks.fetch_add(1, Ordering::SeqCst);
            }
        }
    });
    *hardware.mutation_stall.lock().unwrap() = Some((STALL, ticks));
    let recovery = json!({
        "operationId": started["operationId"], "gateway": GATEWAY,
        "recoveryConfirmation": failed["recoveryPhrase"],
        "downloadConfirmation": failed["downloadPhrase"],
        "releaseConfirmation": failed["releasePhrase"]
    });
    http(&router, "/api/device-system2/recover", Some(recovery), 200).await;
    settled_slow(&state, &router, "restored").await;
    ticker.abort();
    let (before, after) = hardware
        .stall_ticks
        .lock()
        .unwrap()
        .expect("the recovery reached its first mutation");
    assert!(
        after - before >= 10,
        "runtime tasks starved during the recovery stall: {before} -> {after}"
    );
}
