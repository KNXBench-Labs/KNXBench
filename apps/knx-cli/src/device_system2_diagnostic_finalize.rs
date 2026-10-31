//! Settles System-2 write evidence and captures its closed diagnostic projection.

use knx_app::commissioning_activity::{WriteGuard, WriteOutcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FinalizeEvent {
    DisconnectStarted,
    DisconnectFinished,
    StoreReleased,
    GuardFinished,
    CaptureStarted,
    CaptureFinished,
}

pub(super) struct Released<I> {
    device: Result<(), String>,
    report: Result<(), String>,
    capture_input: I,
}

pub(super) async fn release<D, F, S, I>(
    disconnect: D,
    store: S,
    device: Result<(), String>,
    report: Result<(), String>,
    capture_input: I,
    mut observe: impl FnMut(FinalizeEvent),
) -> Released<I>
where
    D: FnOnce() -> F,
    F: std::future::Future<Output = ()>,
{
    observe(FinalizeEvent::DisconnectStarted);
    disconnect().await;
    observe(FinalizeEvent::DisconnectFinished);
    drop(store);
    observe(FinalizeEvent::StoreReleased);
    Released {
        device,
        report,
        capture_input,
    }
}

pub(super) fn finish<I>(
    guard: WriteGuard,
    outcome: WriteOutcome,
    released: Released<I>,
    mut observe: impl FnMut(FinalizeEvent),
    capture: impl FnOnce(I) -> Result<(), String>,
) -> Result<(), String> {
    let Released {
        device,
        report,
        capture_input,
    } = released;
    guard.finish(outcome);
    observe(FinalizeEvent::GuardFinished);
    observe(FinalizeEvent::CaptureStarted);
    let captured = capture(capture_input);
    observe(FinalizeEvent::CaptureFinished);
    if let Err(error) = device {
        if let Err(report) = report {
            eprintln!("shareable report not written: {report}");
        }
        if let Err(capture) = captured {
            eprintln!("diagnostic evidence not captured: {capture}");
        }
        return Err(error);
    }
    report?;
    captured
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use knx_app::commissioning_activity::{OneShotLog, WriteOutcome};
    use serde_json::Value;

    use super::{finish, release, FinalizeEvent};

    struct DropProbe {
        disconnected: Arc<AtomicBool>,
        dropped: Arc<AtomicBool>,
    }

    impl Drop for DropProbe {
        fn drop(&mut self) {
            assert!(self.disconnected.load(Ordering::SeqCst));
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    fn guarded_log(dir: &std::path::Path) -> (Arc<OneShotLog>, super::WriteGuard) {
        let log = Arc::new(OneShotLog::for_run(dir.join("activity.sqlite")).unwrap());
        let guard = log
            .start_write("system2Validation", Some("1.1.25".into()))
            .unwrap();
        guard.mark_send_possible().unwrap();
        (log, guard)
    }

    fn state(log: &OneShotLog) -> Value {
        let (rows, more) = log.history_page(0, 10).unwrap();
        assert!(!more);
        assert_eq!(rows.len(), 1);
        serde_json::to_value(&rows[0]).unwrap()["state"].clone()
    }

    #[tokio::test]
    async fn write_guard_is_settled_before_diagnostic_capture() {
        let dir = tempfile::tempdir().unwrap();
        let (log, guard) = guarded_log(dir.path());
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let disconnected = Arc::new(AtomicBool::new(false));
        let dropped = Arc::new(AtomicBool::new(false));
        let released = release(
            {
                let disconnected = disconnected.clone();
                move || async move {
                    disconnected.store(true, Ordering::SeqCst);
                }
            },
            DropProbe {
                disconnected: disconnected.clone(),
                dropped: dropped.clone(),
            },
            Ok(()),
            Ok(()),
            (),
            {
                let events = events.clone();
                move |event| events.lock().unwrap().push(event)
            },
        )
        .await;

        let result = finish(
            guard,
            WriteOutcome::Verified,
            released,
            {
                let events = events.clone();
                move |event| events.lock().unwrap().push(event)
            },
            |()| {
                assert!(disconnected.load(Ordering::SeqCst));
                assert!(dropped.load(Ordering::SeqCst));
                assert_eq!(state(&log), "verified");
                Ok(())
            },
        );

        assert_eq!(result, Ok(()));
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                FinalizeEvent::DisconnectStarted,
                FinalizeEvent::DisconnectFinished,
                FinalizeEvent::StoreReleased,
                FinalizeEvent::GuardFinished,
                FinalizeEvent::CaptureStarted,
                FinalizeEvent::CaptureFinished,
            ]
        );
    }

    #[tokio::test]
    async fn device_then_report_then_capture_error_precedence_is_stable() {
        for (device, report, capture, expected) in [
            (Ok(()), Ok(()), Ok(()), Ok(())),
            (
                Err("device".to_string()),
                Err("report".to_string()),
                Err("capture".to_string()),
                Err("device".to_string()),
            ),
            (
                Ok(()),
                Err("report".to_string()),
                Err("capture".to_string()),
                Err("report".to_string()),
            ),
            (
                Ok(()),
                Ok(()),
                Err("capture".to_string()),
                Err("capture".to_string()),
            ),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let (log, guard) = guarded_log(dir.path());
            let events = Arc::new(std::sync::Mutex::new(Vec::new()));
            let disconnected = Arc::new(AtomicBool::new(false));
            let dropped = Arc::new(AtomicBool::new(false));
            let released = release(
                {
                    let disconnected = disconnected.clone();
                    move || async move {
                        disconnected.store(true, Ordering::SeqCst);
                    }
                },
                DropProbe {
                    disconnected: disconnected.clone(),
                    dropped: dropped.clone(),
                },
                device,
                report,
                capture,
                {
                    let events = events.clone();
                    move |event| events.lock().unwrap().push(event)
                },
            )
            .await;
            let result = finish(
                guard,
                WriteOutcome::Verified,
                released,
                {
                    let events = events.clone();
                    move |event| events.lock().unwrap().push(event)
                },
                |capture| {
                    assert!(disconnected.load(Ordering::SeqCst));
                    assert!(dropped.load(Ordering::SeqCst));
                    assert_eq!(state(&log), "verified");
                    capture
                },
            );
            assert_eq!(result, expected);
            assert_eq!(
                events.lock().unwrap().as_slice(),
                [
                    FinalizeEvent::DisconnectStarted,
                    FinalizeEvent::DisconnectFinished,
                    FinalizeEvent::StoreReleased,
                    FinalizeEvent::GuardFinished,
                    FinalizeEvent::CaptureStarted,
                    FinalizeEvent::CaptureFinished,
                ]
            );
        }
    }
}
