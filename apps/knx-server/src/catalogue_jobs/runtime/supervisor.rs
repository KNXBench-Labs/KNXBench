//! Persistent worker/receive/reaper ownership; independent of HTTP waiters.
use super::*;

impl CatalogueRuntime {
    pub(super) async fn supervise(self: Arc<Self>, mut receiver: mpsc::Receiver<Command>) {
        let mut timer = tokio::time::interval(Duration::from_secs(1));
        loop {
            tokio::select! {
                biased;
                command = receiver.recv() => match command {
                    Some(Command::Setup(record)) => {
                        let root = Arc::clone(&self.root); let dir = self.incarnation_dir.clone();
                        let handle = match record.status.snapshot() { Ok(s) => internal_handle(&s.handle), Err(_) => {self.quarantine(&record);continue;} };
                        match tokio::task::spawn_blocking(move || JobWorkspace::create(root,&dir,handle)).await {
                            Ok(Ok(workspace)) => { let _ = record.workspace.set(Arc::new(Mutex::new(workspace))); }
                            _ => { if let Ok(e) = record.execution() { e.receive_failed(FailureKind::IoFailure); } self.settle_record(&record).await; }
                        }
                    }
                    Some(Command::Upload(record,body,reply)) => {
                        let r = Arc::clone(&record); let worker = tokio::task::spawn_blocking(move || receive_and_preview(r,body,reply));
                        self.join_worker(&record,worker).await;
                    }
                    Some(Command::Apply(record)) => {
                        let r = Arc::clone(&record); let worker = tokio::task::spawn_blocking(move || run_apply(r));
                        self.join_worker(&record,worker).await;
                    }
                    Some(Command::Dispose(record,reply)) => {
                        let result = self.reclaim_record(&record).await;
                        if result.is_ok() && reply.is_some() {
                            if let Ok(mut manager) = self.owner.manager.lock() {
                                if let Some(pos) = manager.state.terminals.iter().position(|t| t.runtime.as_ref().is_some_and(|r| Arc::ptr_eq(r,&record))) {
                                    let old = manager.state.terminals.remove(pos).unwrap(); manager.push_tombstone(old.meta,true);
                                }
                            }
                        }
                        if let Some(reply) = reply { let _ = reply.send(result); }
                    }
                    None => break,
                },
                _ = self.wake.notified() => self.expire_idle().await,
                _ = timer.tick() => { self.expire_idle().await; self.expire_terminals().await; },
            }
            if self.closing.load(Ordering::SeqCst) && receiver.is_empty() {
                self.expire_idle().await;
                // A closed-handle quarantine does not invent a clean outcome.
                // A still-live native session keeps this owner outstanding.
                let active = self.active();
                if active.as_ref().is_none_or(|r| {
                    r.execution().is_ok_and(|e| e.handles_closed())
                        && r.status
                            .snapshot()
                            .is_ok_and(|s| s.cleanup.disposition == CleanupDisposition::Quarantined)
                }) {
                    let _readers = self.detail_permit.acquire().await;
                    break;
                }
            }
        }
    }

    fn active(&self) -> Option<Arc<JobRecord>> {
        self.owner
            .manager
            .lock()
            .ok()
            .and_then(|m| m.state.active.as_ref().and_then(|a| a.runtime.clone()))
    }
    async fn expire_idle(&self) {
        if let Some(record) = self.active() {
            if let Ok(execution) = record.execution() {
                if catch_unwind(AssertUnwindSafe(|| execution.expire())).is_err() {
                    self.quarantine(&record);
                }
                if record
                    .status
                    .snapshot()
                    .is_ok_and(|s| s.phase == Some(Phase::Settling))
                {
                    let _ = catch_unwind(AssertUnwindSafe(|| execution.settle_idle()));
                }
                self.settle_record(&record).await;
            }
        }
    }
    async fn join_worker(&self, record: &Arc<JobRecord>, mut worker: tokio::task::JoinHandle<()>) {
        let mut timer = tokio::time::interval(Duration::from_secs(1));
        loop {
            tokio::select! {
                result = &mut worker => { if result.is_err() { if let Ok(e) = record.execution() { let _ = catch_unwind(AssertUnwindSafe(|| e.worker_lost())); } self.quarantine(record); } break; },
                _ = timer.tick() => { if let Ok(e) = record.execution() { let _ = catch_unwind(AssertUnwindSafe(|| e.expire())); } },
                _ = self.wake.notified() => { if self.closing.load(Ordering::SeqCst) { if let Ok(e) = record.execution() { let _ = e.request_stop(StopCause::ServerStop); } record.status.changed.notify_waiters(); } },
            }
        }
        self.settle_record(record).await;
    }
    async fn settle_record(&self, record: &Arc<JobRecord>) {
        if !record.execution().is_ok_and(|e| e.handles_closed()) {
            self.quarantine(record);
            return;
        }
        let terminal = record
            .status
            .current
            .lock()
            .ok()
            .and_then(|cell| cell.terminal.clone());
        let Some(terminal) = terminal else {
            return;
        };
        if let Ok(workspace) = record.workspace() {
            if terminal.commit.outcome == CommitOutcome::Unknown
                || terminal.cleanup.disposition == CleanupDisposition::Quarantined
            {
                self.quarantine(record);
            } else {
                let parent = self.root.root().parent().map(std::path::Path::to_path_buf);
                let uncommitted = terminal.commit.outcome == CommitOutcome::NotCommitted;
                let cleanup = tokio::task::spawn_blocking(move || {
                    let mut workspace = workspace.lock().map_err(|_| FailureKind::CleanupFailed)?;
                    workspace
                        .cleanup_transients()
                        .map_err(|_| FailureKind::CleanupFailed)?;
                    if uncommitted {
                        workspace
                            .remove_uncommitted_backup(
                                parent.as_deref().ok_or(FailureKind::CleanupFailed)?,
                            )
                            .map_err(|_| FailureKind::CleanupFailed)?;
                    }
                    Ok::<(), FailureKind>(())
                })
                .await;
                if !matches!(cleanup, Ok(Ok(()))) {
                    if let Ok(mut cell) = record.status.current.lock() {
                        if let Some(terminal) = cell.terminal.as_mut() {
                            terminal.cleanup = Cleanup {
                                disposition: CleanupDisposition::Quarantined,
                                failure_kind: Some(CleanupFailureKind::CleanupFailed),
                            };
                            if terminal.state != JobState::Succeeded {
                                terminal.state = JobState::Failed;
                                terminal.failure = Some(Failure {
                                    kind: FailureKind::CleanupFailed,
                                    context: None,
                                });
                            }
                            if terminal.backup.state != BackupState::None {
                                terminal.backup.state = BackupState::Quarantined;
                            }
                        }
                    }
                    self.quarantine(record);
                }
            }
        }
        if record.status.finish_joined().unwrap_or(false) {
            if let Ok(mut manager) = self.owner.manager.lock() {
                if manager
                    .state
                    .active
                    .as_ref()
                    .and_then(|a| a.runtime.as_ref())
                    .is_some_and(|a| Arc::ptr_eq(a, record))
                {
                    manager.finish_active(record.workspace.get().is_none());
                }
            }
        }
    }
    pub(super) fn quarantine(&self, record: &Arc<JobRecord>) {
        if let Ok(mut manager) = self.owner.manager.lock() {
            manager.mark_artifact_store_unavailable();
        }
        if let Ok(workspace) = record.workspace() {
            if let Ok(mut workspace) = workspace.try_lock() {
                workspace.quarantine();
            }
        }
    }
    async fn reclaim_record(&self, record: &Arc<JobRecord>) -> Result<(), FailureKind> {
        let _reader = Arc::clone(&self.detail_permit)
            .acquire_owned()
            .await
            .map_err(|_| FailureKind::ArtifactStoreUnavailable)?;
        let result = if let Some(workspace) = record.workspace.get().cloned() {
            tokio::task::spawn_blocking(move || {
                workspace
                    .lock()
                    .map_err(|_| FailureKind::ArtifactStoreUnavailable)?
                    .dispose()
                    .map_err(|_| FailureKind::ArtifactStoreUnavailable)
            })
            .await
            .map_err(|_| FailureKind::WorkerFailed)?
        } else {
            Ok(())
        };
        if result.is_err() {
            self.quarantine(record);
            return result;
        }
        let mut manager = self
            .owner
            .manager
            .lock()
            .map_err(|_| FailureKind::CatalogueUnavailable)?;
        if let Some(terminal) = manager
            .state
            .terminals
            .iter_mut()
            .find(|t| t.runtime.as_ref().is_some_and(|r| Arc::ptr_eq(r, record)))
        {
            terminal.cleanup_complete = true;
        }
        Ok(())
    }
    async fn expire_terminals(&self) {
        if let Ok(mut manager) = self.owner.manager.lock() {
            manager.expire_tombstones();
        }
        let expired = self
            .owner
            .manager
            .lock()
            .ok()
            .and_then(|m| m.state.terminals.front().and_then(|t| t.runtime.clone()))
            .filter(|r| {
                r.status.snapshot().is_ok_and(|s| {
                    r.status
                        .server_tick_ms()
                        .is_ok_and(|now| now >= s.lease.deadline_tick_ms)
                })
            });
        if let Some(record) = expired {
            if !record.readers_closed.swap(true, Ordering::SeqCst)
                && self.reclaim_record(&record).await.is_ok()
            {
                if let Ok(mut manager) = self.owner.manager.lock() {
                    if let Some(old) = manager.state.terminals.pop_front_if(|t| {
                        t.runtime.as_ref().is_some_and(|r| Arc::ptr_eq(r, &record))
                    }) {
                        manager.push_tombstone(old.meta, false);
                    }
                }
            }
        }
    }
}

fn receive_and_preview(
    record: Arc<JobRecord>,
    mut body: Body,
    reply: oneshot::Sender<Result<(), FailureKind>>,
) {
    let mut reply = Some(reply);
    let result = (|| -> Result<(), FailureKind> {
        let workspace = record.workspace()?;
        let mut workspace = workspace
            .lock()
            .map_err(|_| FailureKind::ArtifactStoreUnavailable)?;
        let initial = record.status.snapshot()?;
        if initial.phase != Some(Phase::Receiving) {
            return Err(FailureKind::InvalidPhase);
        }
        let file = workspace
            .create_file("source.part")
            .map_err(|_| FailureKind::IoFailure)?;
        let mut spool = crate::catalogue_jobs::spool::Spool::from_owned(
            workspace.directory(),
            initial.source.declared_bytes.get(),
            file,
        )
        .map_err(|_| FailureKind::IoFailure)?;
        let remaining = initial
            .lease
            .deadline_tick_ms
            .get()
            .checked_sub(record.status.server_tick_ms()?.get())
            .ok_or(FailureKind::ReceivingTimeout)?;
        let deadline = tokio::time::Instant::now()
            .checked_add(Duration::from_millis(remaining))
            .ok_or(FailureKind::CounterExhausted)?;
        let runtime = tokio::runtime::Handle::current();
        loop {
            let frame = runtime.block_on(async {
                tokio::select! {
                    frame = std::future::poll_fn(|cx| std::pin::Pin::new(&mut body).poll_frame(cx)) => Ok(frame),
                    _ = tokio::time::sleep(Duration::from_secs(60)) => Err(FailureKind::ReceivingIdleTimeout),
                    _ = tokio::time::sleep_until(deadline) => Err(FailureKind::ReceivingTimeout),
                    _ = record.status.changed.notified() => Err(FailureKind::Cancelled),
                }
            });
            let frame = match frame {
                Err(FailureKind::Cancelled) => {
                    if record.status.snapshot()?.gate == Gate::CancelAccepted {
                        return Err(FailureKind::Cancelled);
                    }
                    continue;
                }
                other => other?,
            };
            let Some(frame) = frame else {
                break;
            };
            let frame = frame.map_err(|_| FailureKind::UploadAborted)?;
            if frame.is_trailers() {
                return Err(FailureKind::InvalidRequest);
            }
            if let Ok(data) = frame.into_data() {
                for chunk in data.chunks(crate::catalogue_jobs::spool::IO_CHUNK_BYTES) {
                    if record.status.snapshot()?.gate == Gate::CancelAccepted {
                        return Err(FailureKind::Cancelled);
                    }
                    spool.write(chunk).map_err(|e| match e {
                        crate::catalogue_jobs::spool::SpoolError::Oversize => {
                            FailureKind::UploadLengthMismatch
                        }
                        _ => FailureKind::IoFailure,
                    })?;
                }
                let written = spool.written();
                record.status.update(|s| {
                    s.source.received_bytes = U64(written);
                    if let Some(p) = s.progress.as_mut() {
                        p.completed = U64(written);
                    }
                })?;
            }
        }
        let sealed = spool.finish().map_err(|e| match e {
            crate::catalogue_jobs::spool::SpoolError::Short => FailureKind::UploadLengthMismatch,
            _ => FailureKind::IoFailure,
        })?;
        workspace
            .register_sealed_source()
            .map_err(|_| FailureKind::IoFailure)?;
        let working_deadline = record
            .status
            .server_tick_ms()?
            .get()
            .checked_add(7_200_000)
            .ok_or(FailureKind::CounterExhausted)?;
        record.status.update(|s| {
            s.source.received_bytes = U64(sealed.bytes);
            s.source.sha256 = Some(sealed.sha256);
            s.phase = Some(Phase::Previewing);
            s.progress = None;
            s.lease = Lease {
                kind: LeaseKind::Working,
                deadline_tick_ms: U64(working_deadline),
            };
        })?;
        if let Some(reply) = reply.take() {
            let _ = reply.send(Ok(()));
        }
        let bytes = workspace.read_source(sealed.bytes, sealed.sha256)?;
        let execution = record.execution()?;
        if let Err(error) = execution.preview(&bytes, &mut workspace) {
            execution.operation_failed(error);
        }
        Ok(())
    })();
    if let Err(error) = result {
        if let Some(reply) = reply.take() {
            let _ = reply.send(Err(error));
        }
        if let Ok(execution) = record.execution() {
            execution.receive_failed(error);
        }
    }
}
fn run_apply(record: Arc<JobRecord>) {
    let result = (|| {
        let execution = record.execution()?;
        let workspace = record.workspace()?;
        let mut workspace = workspace
            .lock()
            .map_err(|_| FailureKind::ArtifactStoreUnavailable)?;
        let status = record.status.snapshot()?;
        let bytes = workspace.read_source(
            status.source.declared_bytes.get(),
            status.source.sha256.ok_or(FailureKind::ReportMismatch)?,
        )?;
        execution.apply(&bytes, &mut workspace)
    })();
    if let Err(error) = result {
        if let Ok(execution) = record.execution() {
            execution.operation_failed(error);
        }
    }
}
