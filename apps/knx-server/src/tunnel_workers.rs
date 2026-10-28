//! Keeps synchronous persistence off the runtime workers that serve open KNXnet/IP tunnels.
//!
//! A tunnel's receive task has to acknowledge every TUNNELLING_REQUEST within
//! one second, or the gateway drops the connection (Tunnelling v01.07.01 AS
//! §2.6.1; issue #30, S2-04). Backups, activity-journal rows and executor
//! checkpoints are written synchronously (fsync, readback) while a tunnel is
//! open. On a runtime worker that blocks every other task of that worker, and
//! a server with few workers (a CPU-limited container) can starve the receive
//! task. The CLI avoids this with `tunnel_runtime()`; the server uses
//! `block_in_place`, which hands the worker's other tasks to a fresh worker
//! while the blocking code runs here, in order.
//!
//! Two shapes, chosen by who may cancel the work and how much must run there:
//!
//! * [`off_tunnel_workers`] runs a whole future there. For owned worker tasks
//!   that nothing cancels by dropping them (System-2 executors, the ordinary
//!   download task): while it runs, the future is not polled from outside.
//! * [`blocking_off_tunnel_workers`] runs one synchronous step there. For a
//!   request handler whose future must stay cancellable at its other await
//!   points (the service-control backup callback), or where only one durable
//!   step of an owned task needs it (the individual-address settler, which
//!   may save the project before disconnect).
//!
//! A current-thread runtime has no second thread; both keep the inline path.

/// Runs `operation` where its synchronous steps cannot stall tunnel tasks.
pub(crate) async fn off_tunnel_workers<F: std::future::Future>(operation: F) -> F::Output {
    let handle = tokio::runtime::Handle::current();
    match handle.runtime_flavor() {
        tokio::runtime::RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(move || handle.block_on(operation))
        }
        _ => operation.await,
    }
}

/// Runs the synchronous `step` where it cannot stall tunnel tasks.
pub(crate) fn blocking_off_tunnel_workers<R>(step: impl FnOnce() -> R) -> R {
    match tokio::runtime::Handle::try_current().map(|handle| handle.runtime_flavor()) {
        Ok(tokio::runtime::RuntimeFlavor::MultiThread) => tokio::task::block_in_place(step),
        _ => step(),
    }
}

#[cfg(test)]
mod tests {
    use super::{blocking_off_tunnel_workers, off_tunnel_workers};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;

    /// Stands in for a multi-second durable checkpoint.
    const STALL: Duration = Duration::from_millis(300);

    /// How the stalled task blocks its thread for `STALL`.
    #[derive(Clone, Copy)]
    enum Shape {
        Inline,
        Future,
        Step,
    }

    /// Ticks of a concurrently spawned task while one task blocks its thread
    /// for `STALL` in the given shape.
    async fn ticks_during_stall(shape: Shape) -> usize {
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
        while ticks.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        // Spawned, so it runs on the single worker like the server's tasks.
        let stalled = tokio::spawn({
            let ticks = ticks.clone();
            async move {
                let stall = move || {
                    let before = ticks.load(Ordering::SeqCst);
                    std::thread::sleep(STALL);
                    ticks.load(Ordering::SeqCst) - before
                };
                let ticked = match shape {
                    Shape::Inline => stall(),
                    Shape::Future => off_tunnel_workers(async move { stall() }).await,
                    Shape::Step => blocking_off_tunnel_workers(stall),
                };
                tokio::task::yield_now().await;
                ticked
            }
        });
        let ticked = stalled.await.expect("stalled task");
        ticker.abort();
        ticked
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn a_blocking_executor_leaves_runtime_tasks_running() {
        let ticked = ticks_during_stall(Shape::Future).await;
        // About 60 nominal; a loaded host still delivers far more than five.
        assert!(ticked >= 5, "only {ticked} ticks during the stall");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn a_blocking_step_leaves_runtime_tasks_running() {
        let ticked = ticks_during_stall(Shape::Step).await;
        assert!(ticked >= 5, "only {ticked} ticks during the stall");
    }

    /// Calibration: the same stall inline starves the only worker.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn the_same_stall_inline_starves_the_single_worker() {
        let ticked = ticks_during_stall(Shape::Inline).await;
        assert!(
            ticked <= 1,
            "{ticked} ticks although the worker was blocked"
        );
    }

    /// Unit tests run on a current-thread runtime: the inline path, no panic.
    #[tokio::test]
    async fn a_current_thread_runtime_keeps_the_inline_path() {
        assert_eq!(off_tunnel_workers(async { 7 }).await, 7);
        assert_eq!(blocking_off_tunnel_workers(|| 8), 8);
    }

    /// Outside any runtime the step simply runs.
    #[test]
    fn without_a_runtime_the_step_runs_inline() {
        assert_eq!(blocking_off_tunnel_workers(|| 9), 9);
    }
}
