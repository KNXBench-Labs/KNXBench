//! Drain contention, cancellation and real blocking-worker ownership controls.
use super::*;
use std::sync::Arc;
use tokio::sync::Notify;

fn owner() -> Arc<CatalogueUpdateJobs> {
    Arc::new(CatalogueUpdateJobs::new(Incarnation::from_bytes([3; 16])))
}

async fn pending(future: std::pin::Pin<&mut impl Future<Output = CatalogueDrain>>) {
    let mut future = future;
    std::future::poll_fn(|cx| {
        assert!(future.as_mut().poll(cx).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
}

fn poison_handles(jobs: &CatalogueUpdateJobs) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = jobs.handles.lock().unwrap();
        panic!("injected ownership mutation panic");
    }));
    assert!(result.is_err());
    assert!(jobs.handles.is_poisoned());
}

#[tokio::test(start_paused = true)]
async fn poisoned_empty_owner_cannot_certify_clean_absence() {
    let jobs = owner();
    poison_handles(&jobs);
    for _ in 0..2 {
        let result = jobs.drain(Duration::ZERO, std::future::pending()).await;
        assert_eq!(result.cleanup, DrainCleanup::Quarantined);
        assert_eq!(result.commit, DrainCommit::Unknown);
        assert_eq!(result.end, DrainEnd::TimedOut);
        assert!(
            result.work_outstanding,
            "unreconciled ownership is outstanding"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn poisoned_owner_joins_known_worker_without_clearing_uncertainty() {
    let jobs = owner();
    let (release, wait) = tokio::sync::oneshot::channel();
    let (done, ended) = tokio::sync::oneshot::channel();
    jobs.register_worker(tokio::spawn(async move {
        wait.await.unwrap();
        done.send(()).unwrap();
    }))
    .unwrap();
    poison_handles(&jobs);
    let waiting = jobs.drain(Duration::ZERO, std::future::pending()).await;
    release.send(()).unwrap();
    ended.await.unwrap();
    let joined = jobs.drain(Duration::ZERO, std::future::pending()).await;
    // The completion witness alone is not the join witness: inspect owner state.
    let handles = match jobs.handles.lock() {
        Err(poisoned) => poisoned.into_inner(),
        Ok(_) => panic!("poison must remain visible"),
    };
    assert!(handles.worker.is_none());
    drop(handles);
    assert!(waiting.work_outstanding);
    assert_eq!(waiting.cleanup, DrainCleanup::Quarantined);
    assert_eq!(joined.end, DrainEnd::TimedOut);
    assert!(joined.work_outstanding);
    assert_eq!(joined.cleanup, DrainCleanup::Quarantined);
    assert_eq!(joined.commit, DrainCommit::Unknown);
}

#[tokio::test(start_paused = true)]
async fn poisoned_registration_returns_submitted_worker_ownership() {
    let jobs = owner();
    poison_handles(&jobs);
    let (release, wait) = tokio::sync::oneshot::channel();
    let (done, ended) = tokio::sync::oneshot::channel();
    let worker = tokio::spawn(async move {
        wait.await.unwrap();
        done.send(()).unwrap();
    });
    let worker_id = worker.id();
    let registration = jobs.register_worker(worker);
    let observed = jobs.drain(Duration::ZERO, std::future::pending()).await;
    release.send(()).unwrap();
    ended.await.unwrap();
    // Clean up the fixture before asserting, even if registration detached it.
    let returned_id = match registration {
        Err(worker) => {
            let id = worker.id();
            worker.await.unwrap();
            Some(id)
        }
        Ok(()) => None,
    };
    assert_eq!(
        returned_id,
        Some(worker_id),
        "failed registration must return ownership"
    );
    assert!(observed.work_outstanding);
    assert_eq!(observed.cleanup, DrainCleanup::Quarantined);
}

#[tokio::test(start_paused = true)]
async fn occupied_registration_returns_new_worker_without_replacing_known_worker() {
    let jobs = owner();
    let (release_first, wait_first) = tokio::sync::oneshot::channel();
    let (done_first, ended_first) = tokio::sync::oneshot::channel();
    let first = tokio::spawn(async move {
        wait_first.await.unwrap();
        done_first.send(()).unwrap();
    });
    let first_id = first.id();
    jobs.register_worker(first).unwrap();
    let (release_second, wait_second) = tokio::sync::oneshot::channel();
    let (done_second, ended_second) = tokio::sync::oneshot::channel();
    let second = tokio::spawn(async move {
        wait_second.await.unwrap();
        done_second.send(()).unwrap();
    });
    let second_id = second.id();
    let registration = jobs.register_worker(second);
    let retained_id = jobs.handles.lock().unwrap().worker.as_ref().unwrap().id();
    release_first.send(()).unwrap();
    release_second.send(()).unwrap();
    ended_first.await.unwrap();
    ended_second.await.unwrap();
    let returned_id = match registration {
        Err(worker) => {
            let id = worker.id();
            worker.await.unwrap();
            Some(id)
        }
        Ok(()) => None,
    };
    let drained = jobs
        .drain(Duration::from_secs(1), std::future::pending())
        .await;
    assert_eq!(
        retained_id, first_id,
        "registration must not overwrite known ownership"
    );
    assert_eq!(
        returned_id,
        Some(second_id),
        "occupied registration must return ownership"
    );
    assert_eq!(drained.end, DrainEnd::Finished);
}

#[tokio::test(start_paused = true)]
async fn empty_drain_ignores_ready_escape_signals() {
    let jobs = owner();
    for _ in 0..32 {
        let result = jobs.drain(Duration::ZERO, std::future::ready(())).await;
        assert_eq!(result.end, DrainEnd::NothingRunning);
        assert!(!result.work_outstanding);
    }
}

#[tokio::test(start_paused = true)]
async fn competing_drain_deadline_includes_wait_for_join_permit() {
    let jobs = owner();
    let release = Arc::new(Notify::new());
    let worker_release = Arc::clone(&release);
    jobs.register_worker(tokio::spawn(async move { worker_release.notified().await }))
        .unwrap();
    let mut first = Box::pin(jobs.drain(Duration::from_secs(60), std::future::pending()));
    pending(first.as_mut()).await;
    // No standard mutex is held while either drain awaits.
    assert!(jobs.handles.try_lock().is_ok());
    assert!(jobs.manager.try_lock().is_ok());
    let mut second = Box::pin(jobs.drain(Duration::from_secs(5), std::future::pending()));
    pending(second.as_mut()).await;
    tokio::time::advance(Duration::from_secs(5)).await;
    let result = second.await;
    release.notify_one();
    let completed = first.await;
    assert_eq!(result.end, DrainEnd::TimedOut);
    assert!(result.work_outstanding);
    assert_eq!(completed.end, DrainEnd::Finished);
}

#[tokio::test(start_paused = true)]
async fn competing_drain_second_signal_includes_wait_for_join_permit() {
    let jobs = owner();
    let release = Arc::new(Notify::new());
    let worker_release = Arc::clone(&release);
    jobs.register_worker(tokio::spawn(async move { worker_release.notified().await }))
        .unwrap();
    let mut first = Box::pin(jobs.drain(Duration::from_secs(60), std::future::pending()));
    pending(first.as_mut()).await;
    let mut second = Box::pin(jobs.drain(
        Duration::from_secs(60),
        tokio::time::sleep(Duration::from_secs(1)),
    ));
    pending(second.as_mut()).await;
    tokio::time::advance(Duration::from_secs(1)).await;
    let result = second.await;
    release.notify_one();
    assert_eq!(first.await.end, DrainEnd::Finished);
    assert_eq!(result.end, DrainEnd::SecondSignal);
    assert!(result.work_outstanding);
}

#[tokio::test(start_paused = true)]
async fn competing_drain_waits_until_worker_is_joined_once() {
    let jobs = owner();
    let release = Arc::new(Notify::new());
    let worker_release = Arc::clone(&release);
    jobs.register_worker(tokio::spawn(async move { worker_release.notified().await }))
        .unwrap();
    let mut first = Box::pin(jobs.drain(Duration::from_secs(60), std::future::pending()));
    pending(first.as_mut()).await;
    let mut second = Box::pin(jobs.drain(Duration::from_secs(60), std::future::pending()));
    pending(second.as_mut()).await;
    release.notify_one();
    let (one, two) = tokio::join!(first, second);
    assert_eq!(one.end, DrainEnd::Finished);
    assert_eq!(two.end, DrainEnd::NothingRunning);
    assert!(!one.work_outstanding && !two.work_outstanding);
    assert!(jobs.handles.lock().unwrap().worker.is_none());
}

#[tokio::test]
async fn aborting_drain_task_retains_a_real_blocking_worker() {
    let jobs = owner();
    let (release, wait) = std::sync::mpsc::channel();
    let (started, running) = tokio::sync::oneshot::channel();
    jobs.register_worker(tokio::task::spawn_blocking(move || {
        started.send(()).unwrap();
        wait.recv().unwrap();
    }))
    .unwrap();
    running.await.unwrap();
    let first_jobs = Arc::clone(&jobs);
    let (polled, waiting) = tokio::sync::oneshot::channel();
    let first = tokio::spawn(async move {
        first_jobs
            .drain(Duration::from_secs(60), async {
                let _ = polled.send(());
                std::future::pending().await
            })
            .await
    });
    waiting.await.unwrap();
    first.abort(); // Only the drain waiter is aborted, never the blocking job.
    assert!(first.await.unwrap_err().is_cancelled());
    let observed = jobs.drain(Duration::ZERO, std::future::pending()).await;
    release.send(()).unwrap();
    let joined = jobs
        .drain(Duration::from_secs(60), std::future::pending())
        .await;
    assert_eq!(observed.end, DrainEnd::TimedOut);
    assert!(observed.work_outstanding);
    assert_eq!(joined.end, DrainEnd::Finished);
    assert!(!joined.work_outstanding);
}
