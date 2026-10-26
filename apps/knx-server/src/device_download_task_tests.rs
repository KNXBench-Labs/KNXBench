//! Offline regressions for the server download worker's lifecycle.

use super::*;
use std::time::Duration;
use tokio::sync::Notify;

fn session(
    task: JoinHandle<()>,
    status: DownloadStatus,
    last: Option<usize>,
) -> DeviceDownloadSession {
    DeviceDownloadSession {
        id: 1,
        target: "1.1.67".parse().unwrap(),
        device_name: "synthetic device".into(),
        steps: 3,
        data_octets: 1,
        shared: Arc::new(Shared {
            stop: Arc::new(DownloadStop::already_committed()),
            after_keep: Mutex::new(None),
            status: Mutex::new(status),
            transfer: Mutex::new(None),
            events: Mutex::new(Vec::new()),
            first_mutation_step: Some(2),
            last_started: Mutex::new(last),
            read_back: Mutex::new(0),
            plan: super::synthetic_plan(),
            granted: Mutex::new(None),
            backup_file: Mutex::new(None),
            on_end: Mutex::new(None),
        }),
        task: Some(task),
        report: synthetic_report(),
    }
}

pub(super) fn synthetic_report() -> DownloadReport {
    use knx_app::download_report::*;
    DownloadReport {
        kind: REPORT_KIND,
        format: REPORT_FORMAT,
        knxbench_version: "test".into(),
        program: ReportedProgram {
            id: "M-0083_A-0001-01-0000".into(),
            name: None,
            application_version: None,
            source_sha256: None,
            manufacturer: "0083".into(),
            mask: "0701".into(),
        },
        scope: "complete",
        support: "untested",
        inferences: vec![],
        warnings: vec![],
        plan: ReportedPlan {
            steps: 3,
            data_octets: 1,
            group_links: 0,
        },
        outcome: None,
        post_download_check: None,
        failure: None,
        parameter_values: Default::default(),
    }
}

async fn wait_until_finished(session: &DeviceDownloadSession) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while !session.task.as_ref().unwrap().is_finished() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("synthetic worker ended");
}

fn finished() -> DownloadStatus {
    DownloadStatus::Finished {
        written: Written::Yes,
        restart: Restart::Unconfirmed,
        restart_note: Some("synthetic restart not acknowledged".into()),
        post_download_check: PostDownloadCheck::not_checked(),
    }
}

#[tokio::test]
async fn polling_a_panicked_worker_does_not_leave_it_running() {
    let task = tokio::spawn(async { panic!("synthetic worker failure") });
    let session = session(task, DownloadStatus::Running, None);
    wait_until_finished(&session).await;
    let (status, next, events) = session.snapshot_since(0);
    assert!(matches!(
        status,
        DownloadStatus::Failed {
            written: Written::No,
            ..
        }
    ));
    assert_eq!(next, 0);
    assert!(events.is_empty());
    assert!(!session.is_running());
}

#[tokio::test]
async fn terminal_result_still_reserves_tunnel_until_worker_cleanup_ends() {
    let release = Arc::new(Notify::new());
    let waiter = Arc::clone(&release);
    let task = tokio::spawn(async move { waiter.notified().await });
    let mut session = session(task, finished(), Some(2));
    assert_eq!(session.status(), finished());
    assert!(session.is_running(), "cleanup still owns the tunnel");
    release.notify_one();
    session.join().await;
    assert!(!session.is_running());
    assert_eq!(session.status(), finished());
}

#[tokio::test]
async fn dropping_a_join_wait_does_not_detach_cleanup_or_release_the_reservation() {
    let release = Arc::new(Notify::new());
    let waiter = Arc::clone(&release);
    let task = tokio::spawn(async move { waiter.notified().await });
    let mut session = session(task, finished(), Some(2));
    assert!(tokio::time::timeout(Duration::ZERO, session.join())
        .await
        .is_err());
    assert!(
        session.task.is_some(),
        "the live worker must remain tracked"
    );
    assert!(session.is_running());
    release.notify_one();
    session.join().await;
    assert!(!session.is_running());
    assert_eq!(session.status(), finished());
}

#[tokio::test]
async fn joining_a_cleanup_panic_preserves_a_witnessed_terminal_result() {
    let task = tokio::spawn(async { panic!("synthetic cleanup failure") });
    let mut session = session(task, finished(), Some(2));
    session.join().await;
    assert_eq!(session.status(), finished());
}

#[tokio::test]
async fn a_task_failure_during_read_only_steps_does_not_claim_a_partial_write() {
    let task = tokio::spawn(async { panic!("synthetic pre-write failure") });
    let mut session = session(task, DownloadStatus::Running, Some(0));
    session.join().await;
    assert!(matches!(
        session.status(),
        DownloadStatus::Failed {
            written: Written::No,
            ..
        }
    ));
}

#[tokio::test]
async fn a_task_failure_after_mutation_started_keeps_a_conservative_partial_result() {
    let task = tokio::spawn(async { panic!("synthetic post-boundary failure") });
    let mut session = session(task, DownloadStatus::Running, Some(2));
    session.join().await;
    assert!(matches!(
        session.status(),
        DownloadStatus::Failed {
            written: Written::Partially,
            ..
        }
    ));
}

#[tokio::test]
async fn polling_an_aborted_worker_keeps_backup_and_marks_outcome_unconfirmed() {
    let task = tokio::spawn(std::future::pending::<()>());
    let session = session(task, DownloadStatus::Running, Some(2));
    let backup = std::path::PathBuf::from("synthetic.backup.json");
    *session.shared.backup_file.lock().unwrap() = Some(backup.clone());
    session.task.as_ref().unwrap().abort();
    wait_until_finished(&session).await;
    let (status, _, _) = session.snapshot_since(0);
    let json = serde_json::to_value(&status).unwrap();
    assert_eq!(json["state"], "failed");
    assert_eq!(json["written"], "partially");
    assert_eq!(json["stoppedInStep"], 3);
    assert!(json["error"].as_str().unwrap().contains("unknown outcome"));
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("cleanup is unconfirmed"));
    assert_eq!(session.backup_file(), Some(backup));
    assert!(!session.is_running());
}

#[tokio::test]
async fn a_worker_returning_without_a_terminal_result_is_not_success() {
    let task = tokio::spawn(async {});
    let mut session = session(task, DownloadStatus::Running, None);
    session.join().await;
    assert!(matches!(
        session.status(),
        DownloadStatus::Failed {
            written: Written::No,
            ..
        }
    ));
    assert!(!session.is_running());
}

#[tokio::test]
async fn a_report_exists_only_once_the_run_and_its_cleanup_have_ended() {
    let release = Arc::new(Notify::new());
    let waiter = Arc::clone(&release);
    let task = tokio::spawn(async move { waiter.notified().await });
    let mut session = session(task, finished(), Some(2));
    assert_eq!(session.report(), None, "cleanup still runs");
    release.notify_one();
    session.join().await;
    let report = session.report().expect("ended");
    assert_eq!(
        report.outcome,
        Some(ReportedOutcome::completed(
            Written::Yes,
            Restart::Unconfirmed
        ))
    );
    assert_eq!(report.program.id, "M-0083_A-0001-01-0000");
}

#[tokio::test]
async fn a_stopped_run_reports_where_it_stopped_and_what_it_wrote() {
    let task = tokio::spawn(async { panic!("synthetic failure") });
    let mut session = session(task, DownloadStatus::Running, Some(2));
    session.join().await;
    let outcome = session
        .report()
        .expect("ended")
        .outcome
        .expect("an outcome");
    assert!(!outcome.completed);
    assert_eq!(outcome.stopped_in_step, Some(3));
    assert_eq!(outcome.restart, None);
    assert_eq!(outcome.written, session_written(&session));
}

fn session_written(session: &DeviceDownloadSession) -> Written {
    match session.status() {
        DownloadStatus::Failed { written, .. } | DownloadStatus::Finished { written, .. } => {
            written
        }
        DownloadStatus::Running => panic!("still running"),
    }
}

#[tokio::test]
async fn an_unexpected_end_settles_programming_once_before_publishing_failure() {
    let task = tokio::spawn(async { panic!("synthetic post-write failure") });
    let session = session(task, DownloadStatus::Running, Some(2));
    let observed = Arc::new(Mutex::new(Vec::new()));
    let observed_by_callback = Arc::clone(&observed);
    *session.shared.on_end.lock().unwrap() = Some(Box::new(move |status| {
        observed_by_callback.lock().unwrap().push(status.clone());
    }));
    wait_until_finished(&session).await;
    let first = session.status();
    assert!(matches!(
        first,
        DownloadStatus::Failed {
            written: Written::Partially,
            ..
        }
    ));
    assert_eq!(
        *observed.lock().unwrap(),
        vec![first.clone()],
        "settled before status is returned"
    );
    assert_eq!(session.status(), first);
    assert_eq!(
        observed.lock().unwrap().len(),
        1,
        "no repeated settlement on polling"
    );
}
