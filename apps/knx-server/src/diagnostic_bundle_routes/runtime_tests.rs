//! Diagnostic collection waits off the runtime without changing admitted bytes or provenance.

use super::*;
use std::future::Future;
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use tokio::sync::Notify;

const BACKSTOP: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Debug)]
enum Lock {
    Project,
    StorePath,
    Opaque,
}

/// Release and join even if a test unwinds; an inline mutant has a bounded escape.
struct Held {
    release: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<bool>>,
}

impl Held {
    fn finish(&mut self) -> bool {
        let _ = self.release.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap()
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn hold(
    state: &SharedState,
    lock: Lock,
    publish: impl FnOnce(&crate::AppState) + Send + 'static,
) -> Held {
    let state = Arc::clone(state);
    let (ready, acquired) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let wait = || {
            ready.send(()).unwrap();
            let timely = released.recv_timeout(BACKSTOP).is_ok();
            publish(&state);
            timely
        };
        match lock {
            Lock::Project => {
                let _guard = state.project.lock().unwrap();
                wait()
            }
            Lock::StorePath => {
                let _guard = state.store_path.lock().unwrap();
                wait()
            }
            Lock::Opaque => {
                let _guard = state.opaque.lock().unwrap();
                wait()
            }
        }
    });
    acquired.recv_timeout(BACKSTOP).unwrap();
    Held {
        release: Some(release),
        thread: Some(thread),
    }
}

/// The task's first poll enters the real owning async seam before this timer runs.
/// Time is a starvation backstop, not proof that an unpolled task reached a lock.
async fn responsive<T: Send + 'static>(
    mut held: Held,
    work: impl Future<Output = T> + Send + 'static,
    label: &str,
) -> T {
    let (entered, entry) = tokio::sync::oneshot::channel();
    let call = tokio::spawn(async move {
        let _ = entered.send(());
        work.await
    });
    entry.await.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    let early = call.is_finished();
    let timely = held.finish();
    let result = tokio::time::timeout(BACKSTOP, call).await.unwrap().unwrap();
    assert!(timely, "{label} blocked the runtime");
    assert!(!early, "{label} answered before the held lock was released");
    result
}

fn fixture() -> (tempfile::TempDir, SharedState, u32) {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("synthetic.knxproj");
    let topology = br#"<KNX xmlns="http://knx.org/xml/project/21"><Project Id="P-0001">
<Installations><Installation InstallationId="0" Name="Synthetic" BCUKey="4294967295">
<Topology><Area Id="P-0001-0_A-1" Address="1"><Line Id="P-0001-0_L-1" Address="1">
<Segment Id="P-0001-0_L-1_S-1" Number="0" MediumTypeRefId="MT-0">
<DeviceInstance Id="P-0001-0_DI-1" Name="Synthetic device" Address="5"
 ProductRefId="M-TEST_H-1_P-1" Hardware2ProgramRefId="M-TEST_H-1_HP-1"/>
</Segment></Line></Area></Topology></Installation></Installations></Project></KNX>"#;
    let info = br#"<KNX xmlns="http://knx.org/xml/project/21"><Project Id="P-0001">
<ProjectInformation Name="Synthetic runtime fixture" GroupAddressStyle="ThreeLevel"/>
</Project></KNX>"#;
    std::fs::write(
        &source,
        knx_testsupport::zip_with_entries(&[
            ("P-0001.signature", b"synthetic signature"),
            ("P-0001/0.xml", topology),
            ("P-0001/Project.xml", info),
        ]),
    )
    .unwrap();
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let imported = knx_app::import_ets_project(&source, &conn).unwrap();
    assert_eq!(
        imported.project.devices.iter().count(),
        1,
        "schema-21 fixture device"
    );
    let id = imported.project.devices.iter().next().unwrap().id.0;
    let mut opaque = knx_store::load_opaque(&conn).unwrap();
    knx_app::import_evidence::capture_primary(&mut opaque, &imported.report).unwrap();
    let refs = knx_store::load_manufacturer_refs(&conn).unwrap();
    let saved = dir.path().join("saved.knxdb");
    let store = knx_store::open_and_migrate(&saved).unwrap();
    knx_store::save_project_with_passthrough(&store, &imported.project, &opaque, &refs).unwrap();
    drop(store);
    let state = tests::state(dir.path(), false);
    *state.project.lock().unwrap() = Some(imported.project);
    *state.opaque.lock().unwrap() = opaque;
    state.store_path.lock().unwrap().set(Some(saved));
    // Calibrate positive admission, not just the missing-report/unsaved refusals.
    assert_eq!(admitted(import_report(&state)).entries, 1);
    (dir, state, id)
}

fn admitted<T>(result: Result<T, SourceFailure>) -> T {
    result.unwrap_or_else(|error| panic!("{}", error.message()))
}

async fn build(state: &SharedState, kind: SectionKind, device: u32) -> Vec<BundleSection> {
    let query: BundleQuery =
        serde_json::from_value(json!({"problemType":"projectImport"})).unwrap();
    admitted(
        build_section(
            state,
            &query,
            kind,
            None,
            Err(SourceFailure::unavailable()),
            SectionScope {
                recordings: &[],
                projectable: Ok(Some(device)),
                pinned_monitor: None,
                monitor: &mut None,
            },
            &mut System2Cache::default(),
        )
        .await,
    )
}

fn same(actual: &BundleSection, expected: &BundleSection) {
    assert_eq!(bundle::section_text(actual), bundle::section_text(expected));
    assert_eq!(
        serde_json::to_value(bundle::preview(actual)).unwrap(),
        serde_json::to_value(bundle::preview(expected)).unwrap()
    );
    assert_eq!(actual.files.len(), expected.files.len());
    for (actual, expected) in actual.files.iter().zip(&expected.files) {
        assert_eq!(actual.name, expected.name);
        assert_eq!(actual.bytes, expected.bytes);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn mounted_preview_text_and_consented_export_leave_the_runtime_free() {
    let (_dir, state, _) = fixture();
    let query = json!({"problemType":"projectImport", "sections":["environment", "importReport"]});
    let (status, bytes, _) = tests::call(
        &state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(query.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let preview = tests::json_of(&bytes);
    for uri in [
        "/api/diagnostics/bundle/preview",
        "/api/diagnostics/bundle/section",
        "/api/diagnostics/bundle",
    ] {
        let mut query = query.clone();
        query["review"] = preview["review"].clone();
        query["key"] = json!("importReport");
        query["consent"] = json!(true);
        let held = hold(&state, Lock::Project, |_| {});
        let owned = Arc::clone(&state);
        let (status, bytes, _) = responsive(
            held,
            async move { tests::call(&owned, "POST", uri, Some(query)).await },
            uri,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{uri}: {}", tests::json_of(&bytes));
        match uri {
            "/api/diagnostics/bundle/preview" => {
                assert_eq!(tests::json_of(&bytes)["review"], preview["review"])
            }
            "/api/diagnostics/bundle/section" => assert_eq!(
                tests::json_of(&bytes)["text"],
                bundle::section_text(&admitted(import_report(&state)))
            ),
            _ => {
                use std::io::Read;
                let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
                let mut report = Vec::new();
                zip.by_name("import/import-report.json")
                    .unwrap()
                    .read_to_end(&mut report)
                    .unwrap();
                assert_eq!(report, admitted(import_report(&state)).files[0].bytes);
            }
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn independent_section_project_and_store_waits_leave_the_runtime_free() {
    let (_dir, state, device) = fixture();
    for (kind, lock) in [
        (SectionKind::Environment, Lock::Project),
        (SectionKind::Environment, Lock::StorePath),
        (SectionKind::ImportReport, Lock::Project),
        (SectionKind::ImportReport, Lock::Opaque),
        (SectionKind::Device, Lock::Project),
        (SectionKind::Catalogue, Lock::Project),
        (SectionKind::Project, Lock::StorePath),
    ] {
        let expected = match kind {
            SectionKind::Environment => environment(&state),
            SectionKind::ImportReport => admitted(import_report(&state)),
            SectionKind::Device => admitted(device_section(&state, device)),
            SectionKind::Catalogue => catalogue(&state, Some(device)),
            SectionKind::Project => {
                let path = state.store_path.lock().unwrap().clone().unwrap();
                let (bytes, notes) = knx_app::secret_free_copy::native_project_for_bundle(&path)
                    .unwrap_or_else(|_| panic!("the synthetic saved project must be admitted"))
                    .into_parts();
                let mut expected = BundleSection::new(SectionKind::Project);
                expected.entries = 1;
                expected.markers = markers(true, true, true, true);
                expected.notes.push(NOTE_SAVED_ROOT.into());
                expected.notes.extend(notes);
                expected.files.push(BundleFile::new("project.knxdb", bytes));
                expected
            }
            _ => unreachable!(),
        };
        let held = hold(&state, lock, |_| {});
        let owned = Arc::clone(&state);
        let actual = responsive(
            held,
            async move { build(&owned, kind, device).await },
            &format!("{kind:?}/{lock:?}"),
        )
        .await;
        assert_eq!(actual.len(), 1);
        same(&actual[0], &expected);
    }
}

struct FinalGate {
    reached: Notify,
    resume: Notify,
}

tokio::task_local! {
    static FINAL_GATE: Arc<FinalGate>;
}

pub(super) async fn before_final_context() {
    if let Ok(gate) = FINAL_GATE.try_with(Arc::clone) {
        gate.reached.notify_one();
        gate.resume.notified().await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn final_context_wait_is_off_runtime_and_refuses_changes_across_collection() {
    let (_dir, state, _) = fixture();
    for change in [false, true] {
        let gate = Arc::new(FinalGate {
            reached: Notify::new(),
            resume: Notify::new(),
        });
        let owned = Arc::clone(&state);
        let call = tokio::spawn(FINAL_GATE.scope(Arc::clone(&gate), async move {
            tests::call(
                &owned,
                "POST",
                "/api/diagnostics/bundle/preview",
                Some(json!({"problemType":"userInterface", "sections":["environment"]})),
            )
            .await
        }));
        tokio::time::timeout(BACKSTOP, gate.reached.notified())
            .await
            .unwrap();
        let held = hold(&state, Lock::Project, move |state| {
            if change {
                state.project_incarnation.fetch_add(1, Ordering::Relaxed);
                state.project_revision.fetch_add(1, Ordering::Relaxed);
            }
        });
        gate.resume.notify_one();
        let (status, bytes, _) =
            responsive(held, async move { call.await.unwrap() }, "final context").await;
        assert_eq!(
            status,
            if change {
                StatusCode::CONFLICT
            } else {
                StatusCode::OK
            }
        );
        if change {
            assert!(String::from_utf8_lossy(&bytes).contains("bundlePreviewChanged"));
            assert!(!bytes.starts_with(b"PK"));
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn recorded_address_wait_rechecks_context_and_requires_a_unique_device() {
    let (_dir, state, device) = fixture();
    let recorder = crate::operation_recordings::store(&state.data_dir, false);
    let recording = recorder.start(
        knx_app::operation_recording::OperationStart::new(
            knx_app::operation_recording::OperationKind::Compare,
        )
        .target(json!({"individualAddress":"1.1.5"})),
    );
    recording.finish(knx_app::operation_recording::RecordedOutcome::succeeded(
        Value::Null,
    ));
    assert!(recorder.wait_until_idle(BACKSTOP));
    let operation = recorder.list().unwrap().pop().unwrap();
    let context = ProjectContext::capture(&state);
    for change in [false, true] {
        let held = hold(&state, Lock::Project, move |state| {
            if change {
                state.project_revision.fetch_add(1, Ordering::Relaxed);
            }
        });
        let owned = Arc::clone(&state);
        let operation = operation.clone();
        let context = context.clone();
        let found = responsive(
            held,
            async move {
                device_by_recorded_address(&owned, Some(&operation), Some(&context), &context)
                    .await
                    .unwrap()
            },
            "recorded address",
        )
        .await;
        assert_eq!(found, (!change).then_some(device));
    }
    let context = ProjectContext::capture(&state);
    assert_eq!(
        device_by_recorded_address(&state, Some(&operation), None, &context)
            .await
            .unwrap(),
        None
    );
    {
        let mut project = state.project.lock().unwrap();
        let project = project.as_mut().unwrap();
        let mut duplicate = project.devices.iter().next().unwrap().clone();
        duplicate.id = project.ids.next_device_id().unwrap();
        project.devices.insert(duplicate);
    }
    assert_eq!(
        device_by_recorded_address(&state, Some(&operation), Some(&context), &context)
            .await
            .unwrap(),
        None
    );
}
