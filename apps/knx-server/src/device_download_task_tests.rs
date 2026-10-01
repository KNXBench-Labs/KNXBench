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
            status: Mutex::new(status),
            events: Mutex::new(Vec::new()),
            first_mutation_step: Some(2),
            last_started: Mutex::new(last),
            granted: Mutex::new(None),
            backup_file: Mutex::new(None),
        }),
        task: Some(task),
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
