//! Settlement receipts: what the real settler reports for each way a run's end reaches the project.

use knx_app::commissioning_activity::{
    CurrentRequest, DownloadPreparation, DownloadResult, DownloadSettlement, Format1, OneShotLog,
    PartBits, ProgramReceipt, SettledParts, SourceVerdict,
};

use super::super::download_settler_receipted;
use super::project_edits::install_rich_project;
use super::*;
use crate::domain;

/// What the real settler leaves for the worker after `status` ends `prepared`.
fn receipt_of(
    f: &Fixture,
    prepared: &PreparedDownload,
    before: impl FnOnce(&Fixture),
    status: &DownloadStatus,
) -> DownloadSettlement {
    let (settler, slot) =
        download_settler_receipted(&f.state, super::super::incarnation(&f.state), 1, prepared);
    before(f);
    settler(status);
    let receipt = slot.lock().unwrap().take();
    receipt.expect("the settler ran to its end and left a receipt")
}

/// The receipt is what the stored row accepts for this run: the real
/// settler's output never contradicts its own row.
fn assert_recordable(
    prepared: &PreparedDownload,
    status: &DownloadStatus,
    receipt: &DownloadSettlement,
) {
    let dir = tempfile::tempdir().unwrap();
    let log = Arc::new(OneShotLog::for_run(dir.path().join("history.sqlite")).unwrap());
    let preparation = DownloadPreparation::from_prepared(prepared).expect("a scope");
    let mut guard = log
        .start_download_prepared(7, address(), preparation)
        .unwrap();
    guard.mark_send_possible().unwrap();
    let result = match status {
        DownloadStatus::Finished {
            written, restart, ..
        } => DownloadResult::Finished {
            written: *written,
            restart: *restart,
        },
        DownloadStatus::Failed { written, .. } => DownloadResult::Failed { written: *written },
        DownloadStatus::Running => unreachable!("a run that ended"),
    };
    guard.record_result(&result).unwrap();
    guard
        .record_settlement(receipt)
        .expect("the receipt agrees with the row it settles");
    guard.record_cleanup(true).unwrap();
    let (rows, _) = log
        .history_page(0, 10)
        .expect("and the strict reader accepts it");
    assert_eq!(rows.len(), 1);
}

fn parts(
    programmed: ProgrammingParts,
    pending: ProgrammingParts,
    touched: ProgrammingParts,
) -> SettledParts {
    SettledParts {
        programmed: PartBits::from(programmed),
        pending: PartBits::from(pending),
        touched: PartBits::from(touched),
    }
}

/// The settled parts and the time of an applied receipt, and its project word.
fn applied(
    receipt: &DownloadSettlement,
) -> (&'static str, SourceVerdict, CurrentRequest, SettledParts) {
    match receipt {
        DownloadSettlement::Persisted {
            source,
            current_request,
            applied,
            settled_at,
            ..
        } => {
            assert_eq!(settled_at.len(), 24);
            ("persisted", *source, *current_request, *applied)
        }
        DownloadSettlement::SessionOnly {
            source,
            current_request,
            applied,
            settled_at,
            ..
        } => {
            assert_eq!(settled_at.len(), 24);
            ("sessionOnly", *source, *current_request, *applied)
        }
        DownloadSettlement::PersistenceFailed {
            source,
            current_request,
            computed,
            ..
        } => ("persistenceFailed", *source, *current_request, *computed),
        other => panic!("no parts in {other:?}"),
    }
}

fn run_receipt(
    f: &Fixture,
    scope: Option<PartialDownloadParts>,
    replace: bool,
    status: DownloadStatus,
) -> (DownloadSettlement, ProgrammingParts) {
    let mut prepared = prepare(&f.state);
    if let Some(parts) = scope {
        prepared = prepared.into_partial(parts).expect("partial plan");
    }
    let receipt = receipt_of(
        f,
        &prepared,
        |f| {
            if replace {
                restore(f);
            }
        },
        &status,
    );
    assert_recordable(&prepared, &status, &receipt);
    (receipt, belief(&f.state))
}

#[test]
fn an_unchanged_source_with_no_project_file_is_session_only_and_credits_everything() {
    let f = fixture(&OLD, &OLD);
    let (receipt, believed) = run_receipt(&f, None, false, finished());
    assert_eq!(believed, ProgrammingParts::ALL);
    assert_eq!(
        applied(&receipt),
        (
            "sessionOnly",
            SourceVerdict::Unchanged,
            CurrentRequest::Readable,
            parts(ProgrammingParts::ALL, NONE, ProgrammingParts::ALL)
        )
    );
}

#[test]
fn a_saved_project_is_persisted_and_the_reopened_flags_agree_with_the_receipt() {
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    let path = f._dir.path().join("project.knxdb");
    domain::save_project_as(&f.state, &path).expect("synthetic project saves");
    let (receipt, believed) = run_receipt(&f, None, false, finished());
    let (word, source, request, settled) = applied(&receipt);
    assert_eq!(
        (word, source, request),
        (
            "persisted",
            SourceVerdict::Unchanged,
            CurrentRequest::Readable
        )
    );
    assert_eq!(settled.programmed, PartBits::from(believed));
    let progress = domain::detached_progress(crate::load_progress::LoadKind::Open, &path);
    let reopened = Arc::new(AppState::default());
    domain::open_native_project(&reopened, &path, true, &progress).expect("reopens");
    assert_eq!(
        belief(&reopened),
        believed,
        "what was applied is what was saved"
    );
}

#[test]
fn a_replaced_file_with_the_same_plan_keeps_its_old_hash_and_says_only_that_the_plan_is_unchanged()
{
    let renamed = Source {
        name: "Renamed",
        ..OLD
    };
    let f = fixture(&OLD, &renamed);
    let prepared = prepare(&f.state);
    let before = DownloadPreparation::from_prepared(&prepared).unwrap();
    let (receipt, believed) = run_receipt(&f, None, true, finished());
    assert_eq!(believed, ProgrammingParts::ALL);
    assert_eq!(applied(&receipt).1, SourceVerdict::Unchanged);
    let replaced = DownloadPreparation::from_prepared(&prepare(&f.state)).unwrap();
    let hash = |preparation: &DownloadPreparation| match preparation {
        DownloadPreparation::Available {
            program: ProgramReceipt::Identified { source_sha256, .. },
            ..
        } => source_sha256.clone(),
        other => panic!("no identified program in {other:?}"),
    };
    assert_ne!(hash(&before), hash(&replaced), "the file really changed");
    assert_eq!(hash(&before).len(), 64);
}

#[test]
fn a_different_plan_credits_no_memory_and_the_receipt_says_changed_or_unknown() {
    let f = fixture(&OLD, &new_base());
    let (receipt, believed) = run_receipt(&f, None, true, finished());
    assert_eq!(believed, ADDRESS);
    assert_eq!(
        applied(&receipt),
        (
            "sessionOnly",
            SourceVerdict::ChangedOrUnknown,
            CurrentRequest::Readable,
            parts(ADDRESS, MEMORY, ProgrammingParts::ALL)
        )
    );
}

#[test]
fn a_partial_run_touches_only_what_it_wrote_and_an_edit_during_it_costs_the_address() {
    let f = fixture(&OLD, &OLD);
    let scope = PartialDownloadParts {
        parameters: true,
        group_addresses: false,
    };
    let (receipt, _) = run_receipt(&f, Some(scope), false, finished());
    assert_eq!(
        applied(&receipt).3,
        parts(PARAMETERS, NONE, PARAMETERS),
        "only the written scope"
    );
    // A complete run whose device address was edited meanwhile: the address
    // is no longer current, so it is pending, not credited.
    let f = fixture(&OLD, &OLD);
    let prepared = prepare(&f.state);
    let receipt = receipt_of(&f, &prepared, |f| edit_address(&f.state), &finished());
    assert_recordable(&prepared, &finished(), &receipt);
    let settled = applied(&receipt).3;
    assert!(!settled.programmed.address && settled.pending.address && settled.touched.address);
}

#[test]
fn an_owner_that_cannot_be_read_leaves_the_request_unavailable_and_credits_nothing() {
    let f = fixture(&OLD, &OLD);
    let prepared = prepare(&f.state);
    let receipt = receipt_of(
        &f,
        &prepared,
        |f| {
            let _ = f.state.product_db.as_ref().unwrap().with_legacy_mutation(
                knx_productdb::managed_catalogue::LegacyMutationKind::PathUpdate,
                |_| Ok::<_, ()>(()),
                |_| knx_productdb::managed_catalogue::LegacySettlementHint::Uncertain,
            );
        },
        &finished(),
    );
    assert_recordable(&prepared, &finished(), &receipt);
    let (_, source, request, settled) = applied(&receipt);
    assert_eq!(
        (source, request),
        (SourceVerdict::ChangedOrUnknown, CurrentRequest::Unavailable)
    );
    assert_eq!(settled.programmed, PartBits::from(NONE));
}

#[test]
fn another_project_no_project_and_a_missing_device_settle_nothing_and_say_so() {
    let f = fixture(&OLD, &OLD);
    let prepared = prepare(&f.state);
    let other = receipt_of(
        &f,
        &prepared,
        |f| {
            f.state
                .project_incarnation
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        },
        &finished(),
    );
    assert_eq!(other, DownloadSettlement::OtherProject { format: Format1 });
    assert_eq!(belief(&f.state), NONE);
    assert_recordable(&prepared, &finished(), &other);

    let none = receipt_of(
        &fixture(&OLD, &OLD),
        &prepared,
        |f| *f.state.project.lock().unwrap() = None,
        &finished(),
    );
    assert_eq!(none, DownloadSettlement::NoProject { format: Format1 });

    let f = fixture(&OLD, &OLD);
    let missing = receipt_of(
        &f,
        &prepared,
        |f| {
            f.state
                .project
                .lock()
                .unwrap()
                .as_mut()
                .unwrap()
                .devices
                .remove(DeviceId(1));
        },
        &finished(),
    );
    assert_eq!(
        missing,
        DownloadSettlement::DeviceMissing { format: Format1 }
    );
    assert_recordable(&prepared, &finished(), &missing);
}

#[test]
fn an_interrupted_run_is_not_source_checked_and_credits_nothing() {
    let f = fixture(&OLD, &new_base());
    mark(&f.state, ProgrammingParts::ALL, true);
    let status = DownloadStatus::Failed {
        written: Written::Partially,
        stopped_in_step: Some(6),
        error: "lost".into(),
        hint: None,
        failure: crate::device_download::synthetic_failure(Some(6)),
    };
    let (receipt, believed) = run_receipt(&f, None, true, status);
    let (_, source, _, settled) = applied(&receipt);
    assert_eq!(
        source,
        SourceVerdict::NotChecked,
        "a replaced source is not looked at"
    );
    assert_eq!(settled.programmed, PartBits::from(NONE));
    assert_eq!(settled.pending, settled.touched);
    assert!(
        settled.touched != PartBits::from(NONE),
        "something may have been written"
    );
    assert_ne!(believed, ProgrammingParts::ALL);
}

#[test]
fn a_run_that_wrote_nothing_is_not_attempted_and_touches_no_flag_and_no_source() {
    let f = fixture(&OLD, &new_base());
    mark(&f.state, ProgrammingParts::ALL, true);
    let prepared = prepare(&f.state);
    let revision = domain::current_project_revision(&f.state);
    let status = failed(Written::No);
    let receipt = receipt_of(&f, &prepared, restore, &status);
    assert_eq!(
        receipt,
        DownloadSettlement::NotAttempted { format: Format1 }
    );
    assert_recordable(&prepared, &status, &receipt);
    assert_eq!(belief(&f.state), ProgrammingParts::ALL);
    assert_eq!(domain::current_project_revision(&f.state), revision);
}

#[test]
fn a_settler_whose_server_is_gone_leaves_an_unavailable_receipt_for_a_run_that_wrote() {
    let f = fixture(&OLD, &OLD);
    let prepared = prepare(&f.state);
    let (settler, slot) = download_settler_receipted(&f.state, 0, 1, &prepared);
    drop(f);
    settler(&finished());
    assert_eq!(
        slot.lock().unwrap().clone(),
        Some(DownloadSettlement::Unavailable { format: Format1 })
    );
}

#[test]
fn a_settler_that_is_never_called_or_sees_a_running_status_leaves_no_receipt() {
    let f = fixture(&OLD, &OLD);
    let prepared = prepare(&f.state);
    let (settler, slot) = download_settler_receipted(&f.state, 0, 1, &prepared);
    settler(&DownloadStatus::Running);
    assert!(slot.lock().unwrap().is_none());
    let (_unused, untouched) = download_settler_receipted(&f.state, 0, 1, &prepared);
    assert!(untouched.lock().unwrap().is_none());
}

#[test]
fn an_unwritable_project_file_computes_the_settlement_without_applying_any_of_it() {
    let f = fixture(&OLD, &OLD);
    *f.state.store_path.lock().unwrap() = Some(f._dir.path().join("missing/dir/project.knxdb"));
    let revision = domain::current_project_revision(&f.state);
    let stack = f.state.command_stack.lock().unwrap().clone();
    let project = f.state.project.lock().unwrap().clone();
    let (receipt, believed) = run_receipt(&f, None, false, finished());
    assert!(
        matches!(receipt, DownloadSettlement::PersistenceFailed { .. }),
        "{receipt:?}"
    );
    let (word, _, _, computed) = applied(&receipt);
    assert_eq!(word, "persistenceFailed");
    assert_eq!(
        computed,
        parts(ProgrammingParts::ALL, NONE, ProgrammingParts::ALL),
        "computed, not applied"
    );
    assert_eq!(believed, NONE, "nothing was applied");
    assert_eq!(domain::current_project_revision(&f.state), revision);
    assert_eq!(*f.state.command_stack.lock().unwrap(), stack);
    assert!(*f.state.project.lock().unwrap() == project);
}

#[test]
fn invalid_catalogue_metadata_makes_the_program_unavailable_and_changes_no_flag_or_plan() {
    let mut believed = Vec::new();
    let mut plans = Vec::new();
    for version in [None, Some("v".repeat(40))] {
        let f = fixture(&OLD, &OLD);
        if let Some(version) = &version {
            let conn = knx_productdb::open_and_migrate(&f.live).unwrap();
            conn.execute(
                "UPDATE application_program SET application_version = ?1",
                [version],
            )
            .unwrap();
        }
        let prepared = prepare(&f.state);
        let preparation = DownloadPreparation::from_prepared(&prepared).unwrap();
        let program_unavailable = matches!(
            preparation,
            DownloadPreparation::Available {
                program: ProgramReceipt::Unavailable,
                ..
            }
        );
        assert_eq!(program_unavailable, version.is_some());
        plans.push(prepared.plan.clone());
        believed.push(full(&f, false));
    }
    assert_eq!(
        believed[0], believed[1],
        "admission and credit are untouched"
    );
    assert_eq!(plans[0], plans[1]);
}

#[test]
fn a_failed_persist_leaves_flags_stack_and_revision_untouched() {
    let f = fixture(&OLD, &OLD);
    // The owned store path cannot be opened: its directory does not exist.
    *f.state.store_path.lock().unwrap() = Some(f._dir.path().join("missing/dir/project.knxdb"));
    let revision = domain::current_project_revision(&f.state);
    let stack = f.state.command_stack.lock().unwrap().clone();
    let project = f.state.project.lock().unwrap().clone();

    run(&f, None, false, finished());

    assert_eq!(belief(&f.state), NONE, "nothing was applied");
    assert_eq!(domain::current_project_revision(&f.state), revision);
    assert_eq!(*f.state.command_stack.lock().unwrap(), stack);
    assert!(*f.state.project.lock().unwrap() == project);
}
