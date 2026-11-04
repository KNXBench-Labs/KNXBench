//! Reaps test-owned threads and the actual worker before propagating assertion failures.
use super::*;
use crate::device_system2::settlement_observation::Observation;
use std::{
    any::Any,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::mpsc,
    task::Poll,
};

pub(super) type Panic = Box<dyn Any + Send>;

// Catch assertion unwind at each poll, without spawning/detaching the case future.
// The future is dropped before cleanup, releasing any temporary guards it owned.
pub(super) async fn catch(future: impl Future<Output = ()>) -> Result<(), Panic> {
    let mut future = std::pin::pin!(future);
    std::future::poll_fn(
        |cx| match catch_unwind(AssertUnwindSafe(|| future.as_mut().poll(cx))) {
            Ok(Poll::Pending) => Poll::Pending,
            Ok(Poll::Ready(())) => Poll::Ready(Ok(())),
            Err(panic) => Poll::Ready(Err(panic)),
        },
    )
    .await
}
pub(super) fn panic_text(panic: &(dyn Any + Send)) -> &str {
    panic
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("non-string panic")
}

pub(super) struct Owned {
    state: crate::SharedState,
    hardware: Arc<Hardware>,
    observation: Arc<Observation>,
    deadline: tokio::time::Instant,
    pub joined_threads: usize,
    pub service_joined: bool,
    pub writer_release: Option<mpsc::Sender<()>>,
    pub writer: Option<std::thread::JoinHandle<Result<(), String>>>,
    pub editor: Option<std::thread::JoinHandle<Result<(), String>>>,
}
impl Owned {
    pub fn new(
        state: &crate::SharedState,
        hardware: &Arc<Hardware>,
        observation: &Arc<Observation>,
    ) -> Self {
        Self {
            state: state.clone(),
            hardware: hardware.clone(),
            observation: observation.clone(),
            deadline: tokio::time::Instant::now() + Duration::from_secs(180),
            joined_threads: 0,
            service_joined: false,
            writer_release: None,
            writer: None,
            editor: None,
        }
    }
    pub fn release_writer(&mut self) {
        if let Some(release) = self.writer_release.take() {
            let _ = release.send(());
        }
    }
    pub async fn finish(&mut self, failed: bool) -> Vec<String> {
        // Release BOTH barriers before joining anything. A dropped event receiver must
        // never make a writer panic while it holds the managed catalogue mutex.
        self.hardware.paused.store(false, Ordering::SeqCst);
        self.hardware.release.notify_one();
        self.release_writer();
        if failed {
            self.state.device_system2.control.close_admission();
            // Cooperative cancellation keeps the worker's blocking settlement JOIN
            // owned too; aborting its async parent could detach that blocking job.
        }
        let mut errors = Vec::new();
        for (name, thread) in [
            ("writer", self.writer.take()),
            ("editor", self.editor.take()),
        ] {
            if let Some(thread) = thread {
                let joined = tokio::task::spawn_blocking(move || thread.join()).await;
                if joined.is_ok() {
                    self.joined_threads += 1;
                }
                match joined {
                    Ok(Ok(Ok(()))) => {}
                    Ok(Ok(Err(error))) => errors.push(format!("{name}: {error}")),
                    Ok(Err(panic)) => {
                        errors.push(format!("{name} panicked: {}", panic_text(panic.as_ref())))
                    }
                    Err(error) => errors.push(format!("{name} join: {error}")),
                }
            }
        }
        // Take ownership even if a test assertion poisoned status. Never detach a live
        // service handle or retire the fixture while it still owns its journal/tunnel.
        let task = self
            .state
            .device_system2
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .current
            .as_mut()
            .and_then(|record| record.task.take());
        if let Some(mut task) = task {
            // Reuse a single case deadline; a failed assertion never restarts a
            // second timed grace period before the mandatory final ownership join.
            match tokio::time::timeout_at(self.deadline, &mut task).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => errors.push(format!("service supervisor join: {error}")),
                Err(_) => {
                    errors.push("service cleanup exceeded existing 180s wait".into());
                    self.state.device_system2.control.close_admission();
                    // The deadline is a failure, not permission to detach. Retain and
                    // JOIN the supervisor (and therefore its actual child) before return.
                    if let Err(error) = task.await {
                        errors.push(format!("service final join: {error}"));
                    }
                }
            }
            self.service_joined = true;
        } else {
            // The normal case already awaited join_finished; completion is also
            // observed on the actual child, not inferred from a missing record handle.
            self.service_joined = self.observation.worker_finished() == Some(true);
        }
        errors
    }
}
pub(super) fn propagate(result: Result<(), Panic>, cleanup: Vec<String>) {
    if let Err(panic) = result {
        if !cleanup.is_empty() {
            eprintln!("secondary cleanup errors: {cleanup:?}");
        }
        std::panic::resume_unwind(panic);
    }
    assert!(cleanup.is_empty(), "owned cleanup: {cleanup:?}");
}

// Read the immutable, committed recovery checkpoint at the actual first-write pause.
// This is the same chain reader used by the existing original-recovery authority proof.
pub(super) fn backup_at_pause(
    operation: &Path,
    original: &knx_app::property_validation_bundle::PreparedBundle,
) -> RecoveryRecord {
    let journal = operation.join("journal");
    let head: Value =
        serde_json::from_slice(&std::fs::read(journal.join("HEAD")).unwrap()).unwrap();
    let mut previous = String::new();
    let mut backup = None;
    for index in 0..=head["sequence"].as_u64().unwrap() {
        let bytes = std::fs::read(journal.join(format!("checkpoint-{index:08}.json"))).unwrap();
        let envelope: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(envelope["sequence"], index);
        assert_eq!(envelope["previous_sha256"], previous);
        let checkpoint: Checkpoint =
            serde_json::from_value(envelope["checkpoint"].clone()).unwrap();
        if let Checkpoint::RecoveryAcquired { recovery } = checkpoint {
            assert!(backup.replace(recovery).is_none(), "one ORIGINAL prestate");
        }
        previous = knx_productdb::sha256_hex(&bytes);
    }
    assert_eq!(head["sha256"], previous);
    let backup = backup.expect("actual original backup before the paused data write");
    assert_eq!(backup.original_bundle, *original);
    backup
}
