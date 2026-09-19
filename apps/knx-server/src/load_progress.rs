//! The one project load a server run may have in flight, and the snapshot clients poll for.
//!
//! ADR-0023 in three sentences. A load is an *operation* that belongs to
//! the server, not to the request that started it: it gets a monotonic id,
//! it runs to completion whether or not the client is still listening, and
//! a second one while it runs is refused rather than queued. Its phases
//! come from the pipeline itself — `knx-etsproj` and `knx-app` announce
//! theirs through [`knx_app::LoadObserver`], this module adds the store
//! and projection steps `domain.rs` drives — so nothing here has to guess
//! what is happening. And a percentage appears only where a real
//! completed/total exists; [`LoadSnapshot`] carries no timestamp at all,
//! so no caller downstream can quietly derive a bar from the clock.
//!
//! The client half of ownership (fix round 3, F9) is an opaque token the
//! caller generates and this module only ever stores and echoes back —
//! never inspects, never compares to anything itself. Three rounds of a
//! server-side heuristic (an id-only test, then an id-and-source test)
//! each let one client's poll adopt another client's operation; an exact
//! token the server never invents ends that class of bug rather than
//! narrowing it again. `None` — no token sent — is stored as `None` and
//! matches nothing: that operation belongs to nobody, on purpose.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Which of the two entry paths a load came in on. Not cosmetic: the
/// phases differ entirely, and a client showing "opening" for an ETS
/// import would be describing work nobody did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadKind {
    /// `POST /api/project/import` — an ETS `.knxproj`.
    Import,
    /// `POST /api/project/open` — this application's own `.knxdb`.
    Open,
}

impl LoadKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            LoadKind::Import => "import",
            LoadKind::Open => "open",
        }
    }
}

/// Where an operation stands. `Failed` is retained exactly as long as
/// `Succeeded` is — a failure the client missed is the one a client most
/// needs to find when it comes back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStatus {
    Running,
    Succeeded,
    Failed,
}

impl LoadStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            LoadStatus::Running => "running",
            LoadStatus::Succeeded => "succeeded",
            LoadStatus::Failed => "failed",
        }
    }
}

/// Every phase either entry path can be in. The first eight come from
/// `knx-etsproj`, the next four from `knx-app`, and the rest are this
/// crate's own work — see ADR-0023's table. The wire spelling of a
/// forwarded phase is the spelling its own crate chose, so the three
/// vocabularies cannot drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadPhase {
    /// Accepted, nothing has reported yet. Lives for microseconds in
    /// practice, and exists so a snapshot polled in that window says
    /// something true rather than something invented.
    Starting,
    /// A phase inside the ETS import pipeline (`knx-etsproj` or
    /// `knx-app`), forwarded with its own name.
    Load(knx_app::LoadStage),
    /// Opening a `.knxdb` and running any pending migration.
    OpenStore,
    /// Reading the normalized project out of the store.
    LoadStoredProject,
    /// Reading the opaque passthrough entries back.
    LoadOpaque,
    /// Reading the manufacturer manifest back.
    LoadManufacturerRefs,
    /// Projecting the loaded project into the tree the UI renders
    /// (`knx-projection`). Runs on both paths, last.
    BuildProjectTree,
}

impl LoadPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            LoadPhase::Starting => "starting",
            LoadPhase::Load(stage) => stage.as_str(),
            LoadPhase::OpenStore => "openStore",
            LoadPhase::LoadStoredProject => "loadStoredProject",
            LoadPhase::LoadOpaque => "loadOpaque",
            LoadPhase::LoadManufacturerRefs => "loadManufacturerRefs",
            LoadPhase::BuildProjectTree => "buildProjectTree",
        }
    }
}

impl From<knx_app::LoadStage> for LoadPhase {
    fn from(stage: knx_app::LoadStage) -> Self {
        LoadPhase::Load(stage)
    }
}

/// What `GET /api/project/load-progress` answers with. Note what is not
/// here: a start time, an end time, a duration, an estimate. ADR-0023
/// forbids progress derived from elapsed time, and the cheapest way to
/// keep that promise is to hand nobody the clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadSnapshot {
    pub operation_id: u64,
    pub kind: LoadKind,
    /// The file name the operation is working on — not the full path: the
    /// browser shows this, and a path tells a screenshot more about the
    /// machine than it needs to.
    pub source: String,
    pub phase: LoadPhase,
    /// Items done within the current phase, where a real total exists.
    /// `None` for every phase that cannot count, which is most of them.
    pub completed: Option<u64>,
    pub total: Option<u64>,
    pub status: LoadStatus,
    /// Set exactly when `status` is [`LoadStatus::Failed`].
    pub error: Option<String>,
    /// The opaque token the caller that started this operation generated
    /// for it, or `None` when it sent none. Stored verbatim and compared
    /// by nobody in this crate — the caller's own equality check on the
    /// echoed value is the entire ownership test (fix round 3, F9).
    pub client_token: Option<String>,
}

/// A second load while one is running. The caller turns this into a `409`
/// — a state conflict the caller can resolve by waiting, not a malformed
/// request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlreadyRunning {
    pub operation_id: u64,
}

/// The registry: at most one operation, and an id counter that never goes
/// backwards. Held in `AppState` as an `Arc` so a [`LoadHandle`] can
/// outlive the request that made it — which is the point, since the work
/// runs on a blocking thread the request may stop waiting for.
#[derive(Debug, Default)]
pub struct LoadOperations {
    current: Mutex<Option<LoadSnapshot>>,
    next_id: AtomicU64,
    /// Every phase the current operation has entered, in order. Test-only:
    /// a snapshot holds one phase at a time, which is all a polling client
    /// needs and not enough to assert an *order* on. Nothing in a release
    /// build allocates this.
    #[cfg(test)]
    phases: Mutex<Vec<&'static str>>,
}

impl LoadOperations {
    /// Claims the single load slot. Fails if one is already running —
    /// whoever holds it is named in the error so the caller can say which
    /// operation is in the way.
    ///
    /// `client_token` is stored verbatim and returned in every snapshot of
    /// this operation; this module never reads it back. `None` means the
    /// caller sent none, which is stored as `None` rather than guessed at.
    pub fn begin(
        self: &Arc<Self>,
        kind: LoadKind,
        source: impl Into<String>,
        client_token: Option<String>,
    ) -> Result<LoadHandle, AlreadyRunning> {
        let mut current = self.current.lock().expect("load progress mutex poisoned");
        if let Some(running) = current.as_ref().filter(|s| s.status == LoadStatus::Running) {
            return Err(AlreadyRunning {
                operation_id: running.operation_id,
            });
        }
        // Ids start at 1: `0` is the value an uninitialised counter would
        // have, and a client comparing ids should never have to wonder
        // whether it is looking at one.
        let operation_id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        *current = Some(LoadSnapshot {
            operation_id,
            kind,
            source: source.into(),
            phase: LoadPhase::Starting,
            completed: None,
            total: None,
            status: LoadStatus::Running,
            error: None,
            client_token,
        });
        Ok(LoadHandle {
            operations: Arc::clone(self),
            operation_id,
        })
    }

    /// The phases the current operation reported, in the order it entered
    /// them. See the field's comment for why this is test-only.
    #[cfg(test)]
    pub fn recorded_phases(&self) -> Vec<&'static str> {
        self.phases
            .lock()
            .expect("phase log mutex poisoned")
            .clone()
    }

    #[cfg(test)]
    fn record_phase(&self, operation_id: u64, phase: LoadPhase) {
        let current = self.current.lock().expect("load progress mutex poisoned");
        if current
            .as_ref()
            .is_some_and(|s| s.operation_id == operation_id)
        {
            self.phases
                .lock()
                .expect("phase log mutex poisoned")
                .push(phase.as_str());
        }
    }

    /// The current operation, running or finished, or `None` when this
    /// server run has never loaded anything.
    pub fn snapshot(&self) -> Option<LoadSnapshot> {
        self.current
            .lock()
            .expect("load progress mutex poisoned")
            .clone()
    }

    fn update(&self, operation_id: u64, change: impl FnOnce(&mut LoadSnapshot)) {
        let mut current = self.current.lock().expect("load progress mutex poisoned");
        // A handle whose operation has already been replaced writes
        // nothing: a late report from a superseded operation would
        // otherwise overwrite the phase of the one the user is watching.
        if let Some(snapshot) = current.as_mut().filter(|s| s.operation_id == operation_id) {
            change(snapshot);
        }
    }
}

/// The running operation's write end. Held by the blocking task doing the
/// work; dropping one that never reported an outcome marks it failed, so a
/// panicking import cannot leave the slot occupied forever.
#[derive(Debug)]
pub struct LoadHandle {
    operations: Arc<LoadOperations>,
    operation_id: u64,
}

impl LoadHandle {
    pub fn operation_id(&self) -> u64 {
        self.operation_id
    }

    /// Enters a phase. Any item count from the previous phase is cleared
    /// here rather than left to go stale — a count belongs to the phase
    /// that reported it, and a bar still showing 36/36 during the next
    /// phase is a bar that is lying about which work it measures.
    pub fn phase(&self, phase: impl Into<LoadPhase>) {
        let phase = phase.into();
        self.operations.update(self.operation_id, |snapshot| {
            snapshot.phase = phase;
            snapshot.completed = None;
            snapshot.total = None;
        });
        #[cfg(test)]
        self.operations.record_phase(self.operation_id, phase);
    }

    /// Reports `completed` of `total` real items within the current phase.
    pub fn items(&self, completed: u64, total: u64) {
        self.operations.update(self.operation_id, |snapshot| {
            snapshot.completed = Some(completed);
            snapshot.total = Some(total);
        });
    }

    pub fn succeed(&self) {
        self.finish(LoadStatus::Succeeded, None);
    }

    pub fn fail(&self, error: impl Into<String>) {
        self.finish(LoadStatus::Failed, Some(error.into()));
    }

    fn finish(&self, status: LoadStatus, error: Option<String>) {
        self.operations.update(self.operation_id, |snapshot| {
            if snapshot.status == LoadStatus::Running {
                snapshot.status = status;
                snapshot.error = error;
                snapshot.completed = None;
                snapshot.total = None;
            }
        });
    }
}

impl Drop for LoadHandle {
    fn drop(&mut self) {
        self.finish(
            LoadStatus::Failed,
            Some("the load ended without reporting an outcome".to_string()),
        );
    }
}

/// The bridge between the pipeline's vocabulary and this one. Every
/// `stage`/`items` call inside `knx-etsproj` and `knx-app` arrives here.
impl knx_app::LoadObserver for LoadHandle {
    fn stage(&self, stage: knx_app::LoadStage) {
        self.phase(stage);
    }

    fn items(&self, completed: u64, total: u64) {
        LoadHandle::items(self, completed, total);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_app::LoadStage;
    use knx_etsproj::ImportStage;

    fn operations() -> Arc<LoadOperations> {
        Arc::new(LoadOperations::default())
    }

    #[test]
    fn nothing_has_happened_yet_is_not_an_operation() {
        assert_eq!(operations().snapshot(), None);
    }

    // Fix round 3, F9: the whole replacement for the id-and-source
    // heuristic is that this module stores the caller's token and hands
    // it back unexamined. Pin both halves — an operation started with a
    // token echoes it, and one started without carries `None` rather than
    // this module inventing something to put there.
    #[test]
    fn a_client_token_is_stored_and_echoed_verbatim() {
        let ops = operations();
        ops.begin(LoadKind::Import, "a.knxproj", Some("token-a".to_string()))
            .unwrap();

        assert_eq!(
            ops.snapshot().unwrap().client_token.as_deref(),
            Some("token-a")
        );
    }

    #[test]
    fn an_operation_started_with_no_token_carries_none_not_a_guess() {
        let ops = operations();
        ops.begin(LoadKind::Import, "a.knxproj", None).unwrap();

        assert_eq!(ops.snapshot().unwrap().client_token, None);
    }

    #[test]
    fn a_second_load_while_one_runs_is_refused_and_names_the_one_in_the_way() {
        let ops = operations();
        let first = ops.begin(LoadKind::Import, "a.knxproj", None).unwrap();

        let refused = ops.begin(LoadKind::Open, "b.knxdb", None).unwrap_err();

        assert_eq!(
            refused,
            AlreadyRunning {
                operation_id: first.operation_id()
            }
        );
        assert_eq!(ops.snapshot().unwrap().source, "a.knxproj");
    }

    #[test]
    fn a_finished_operation_releases_the_slot_and_the_next_id_is_never_reused() {
        let ops = operations();
        let first = ops.begin(LoadKind::Import, "a.knxproj", None).unwrap();
        first.succeed();
        drop(first);

        let second = ops.begin(LoadKind::Open, "b.knxdb", None).unwrap();

        assert_eq!(second.operation_id(), 2);
        let snapshot = ops.snapshot().unwrap();
        assert_eq!(snapshot.kind, LoadKind::Open);
        assert_eq!(snapshot.status, LoadStatus::Running);
    }

    #[test]
    fn a_failure_is_retained_with_its_message_for_whoever_polls_next() {
        let ops = operations();
        let handle = ops.begin(LoadKind::Import, "broken.knxproj", None).unwrap();
        handle.phase(LoadStage::Parse(ImportStage::OpenContainer));
        handle.fail("not a zip archive");
        drop(handle);

        let snapshot = ops.snapshot().unwrap();
        assert_eq!(snapshot.status, LoadStatus::Failed);
        assert_eq!(snapshot.error.as_deref(), Some("not a zip archive"));
        assert_eq!(
            snapshot.phase.as_str(),
            "openContainer",
            "the phase it failed in is retained, not cleared"
        );
    }

    #[test]
    fn a_handle_dropped_without_an_outcome_fails_rather_than_holding_the_slot() {
        let ops = operations();
        drop(ops.begin(LoadKind::Import, "a.knxproj", None).unwrap());

        let snapshot = ops.snapshot().unwrap();
        assert_eq!(snapshot.status, LoadStatus::Failed);
        assert!(snapshot.error.is_some());
        // And the slot is free again, which is the point of the Drop impl.
        assert!(ops.begin(LoadKind::Open, "b.knxdb", None).is_ok());
    }

    #[test]
    fn entering_a_phase_clears_the_previous_phases_item_count() {
        let ops = operations();
        let handle = ops.begin(LoadKind::Import, "a.knxproj", None).unwrap();

        handle.phase(LoadStage::Parse(ImportStage::CollectContainerEntries));
        handle.items(36, 36);
        assert_eq!(ops.snapshot().unwrap().completed, Some(36));

        handle.phase(LoadStage::PersistOpaque);

        let snapshot = ops.snapshot().unwrap();
        assert_eq!(snapshot.phase.as_str(), "persistOpaque");
        assert_eq!(
            (snapshot.completed, snapshot.total),
            (None, None),
            "a count belongs to the phase that measured it"
        );
    }

    #[test]
    fn a_superseded_handle_cannot_overwrite_the_operation_that_replaced_it() {
        let ops = operations();
        let stale = ops.begin(LoadKind::Import, "a.knxproj", None).unwrap();
        stale.succeed();
        let current = ops.begin(LoadKind::Open, "b.knxdb", None).unwrap();
        current.phase(LoadPhase::LoadStoredProject);

        stale.phase(LoadStage::PersistOpaque);
        stale.fail("a late report from a finished operation");

        let snapshot = ops.snapshot().unwrap();
        assert_eq!(snapshot.operation_id, current.operation_id());
        assert_eq!(snapshot.phase.as_str(), "loadStoredProject");
        assert_eq!(snapshot.status, LoadStatus::Running);
    }

    #[test]
    fn a_forwarded_phase_keeps_the_spelling_its_own_crate_chose() {
        assert_eq!(
            LoadPhase::Load(LoadStage::Parse(ImportStage::ParseTopology)).as_str(),
            "parseTopology"
        );
        assert_eq!(
            LoadPhase::Load(LoadStage::EnrichFromProductDatabase).as_str(),
            "enrichFromProductDatabase"
        );
        assert_eq!(LoadPhase::BuildProjectTree.as_str(), "buildProjectTree");
    }
}
