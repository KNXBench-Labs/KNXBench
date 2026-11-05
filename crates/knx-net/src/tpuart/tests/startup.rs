//! Verify pre-session settling without weakening reset matching or post-reset decoding.

use super::*;
use std::{
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

async fn buffered_open(mode: TpuartMode, bad_reset: bool) -> (Result<u64, String>, Vec<u8>) {
    let (stream, mut peer) = tokio::io::duplex(512);
    let mut old = vec![0xf9, 0x05, 0xe1, 0x03, 0x8b];
    old.extend_from_slice(TP);
    peer.write_all(&old).await.unwrap();
    let mut worker = tokio::spawn(async move {
        let mut written = Vec::new();
        let mut reset_count = 0;
        while let Ok(byte) = peer.read_u8().await {
            written.push(byte);
            let response: &[u8] = match byte {
                1 => {
                    reset_count += 1;
                    if bad_reset && reset_count == 1 {
                        &[0xf9]
                    } else {
                        &[3]
                    }
                }
                2 => &[7],
                5 => &[],
                0xf1 => {
                    let mut address = [0; 3];
                    if peer.read_exact(&mut address).await.is_err() {
                        break;
                    }
                    written.extend_from_slice(&address);
                    &[0x21]
                }
                _ => &[],
            };
            if peer.write_all(response).await.is_err() {
                break;
            }
        }
        written
    });
    let outcome = match TpuartClient::connect_stream(stream, source(), mode, timing()).await {
        Ok(client) => {
            let count = client.diagnostics().startup_discarded_bytes;
            client
                .disconnect()
                .await
                .map(|()| count)
                .map_err(|e| e.to_string())
        }
        Err(error) => Err(error.to_string()),
    };
    let written = match timeout(Duration::from_secs(1), &mut worker).await {
        Ok(result) => result.unwrap(),
        Err(_) => {
            worker.abort();
            let _ = worker.await;
            panic!("startup peer was not released");
        }
    };
    if let Ok(discarded) = &outcome {
        assert_eq!(*discarded, old.len() as u64);
    }
    (outcome, written)
}

#[tokio::test]
async fn old_usb_bytes_settle_before_passive_reset() {
    let (opened, written) = buffered_open(TpuartMode::BusMonitor, false).await;
    assert!(opened.is_ok(), "passive session did not open: {opened:?}");
    assert_eq!(written, [1, 2, 5, 1]);
}

#[tokio::test]
async fn old_usb_bytes_settle_before_active_reset() {
    let (opened, written) = buffered_open(TpuartMode::Active, false).await;
    assert!(opened.is_ok(), "active session did not open: {opened:?}");
    assert_eq!(written, [1, 2, 0xf1, 0x11, 0x01, 0, 1]);
}

#[tokio::test]
async fn unknown_input_after_reset_is_still_rejected() {
    let (opened, written) = buffered_open(TpuartMode::BusMonitor, true).await;
    assert!(opened
        .unwrap_err()
        .contains("unknown/unsupported service 0xf9"));
    assert_eq!(written, [1, 1]);
}

#[derive(Clone, Copy)]
enum Input {
    Continuous,
    Pending,
    Eof,
    Error,
}
struct Probe {
    input: Input,
    polled: Arc<AtomicBool>,
    dropped: Arc<AtomicBool>,
    written: Arc<Mutex<Vec<u8>>>,
    started: Option<oneshot::Sender<()>>,
}
impl Drop for Probe {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}
impl AsyncRead for Probe {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.polled.store(true, Ordering::SeqCst);
        if let Some(started) = self.started.take() {
            let _ = started.send(());
        }
        match self.input {
            Input::Continuous => {
                buf.put_slice(&[0xf9]);
                Poll::Ready(Ok(()))
            }
            Input::Pending => Poll::Pending,
            Input::Eof => Poll::Ready(Ok(())),
            Input::Error => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "startup read failed",
            ))),
        }
    }
}
impl AsyncWrite for Probe {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.written.lock().unwrap().extend_from_slice(bytes);
        Poll::Ready(Ok(bytes.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
type ProbeFixture = (
    Probe,
    Arc<AtomicBool>,
    Arc<AtomicBool>,
    Arc<Mutex<Vec<u8>>>,
    oneshot::Receiver<()>,
);
fn probe(input: Input) -> ProbeFixture {
    let polled = Arc::new(AtomicBool::new(false));
    let dropped = Arc::new(AtomicBool::new(false));
    let written = Arc::new(Mutex::new(Vec::new()));
    let (started, receiver) = oneshot::channel();
    (
        Probe {
            input,
            polled: polled.clone(),
            dropped: dropped.clone(),
            written: written.clone(),
            started: Some(started),
        },
        polled,
        dropped,
        written,
        receiver,
    )
}

#[tokio::test]
async fn continuous_startup_input_exhausts_original_budget_without_commands() {
    let (stream, polled, dropped, written, _) = probe(Input::Continuous);
    let configured = Timing {
        initialize: Duration::from_millis(20),
        startup_quiet: Duration::from_millis(5),
        ..timing()
    };
    let opened = timeout(
        Duration::from_secs(1),
        TpuartClient::connect_stream(stream, source(), TpuartMode::BusMonitor, configured),
    )
    .await
    .unwrap();
    assert!(matches!(opened, Err(BusError::Timeout)));
    assert!(polled.load(Ordering::SeqCst));
    assert!(dropped.load(Ordering::SeqCst));
    assert!(written.lock().unwrap().is_empty());
}

#[tokio::test(start_paused = true)]
async fn startup_idle_time_does_not_restart_the_initialization_budget() {
    let (stream, mut peer) = tokio::io::duplex(512);
    let configured = Timing {
        startup_quiet: Duration::from_millis(60),
        ..timing()
    };
    let begin = Instant::now();
    let (opened, (first_reset_at, written)) = tokio::join!(
        TpuartClient::connect_stream(stream, source(), TpuartMode::BusMonitor, configured),
        async {
            let first = peer.read_u8().await.unwrap();
            let at = Instant::now();
            let mut written = vec![first];
            peer.read_to_end(&mut written).await.unwrap();
            (at, written)
        }
    );
    assert!(opened.is_err());
    assert_eq!(first_reset_at - begin, configured.startup_quiet);
    assert_eq!(
        Instant::now() - begin,
        configured.initialize + configured.shutdown
    );
    assert_eq!(written, [1, 1]);
}

#[tokio::test]
async fn startup_eof_releases_stream_without_reset() {
    let (stream, polled, dropped, written, _) = probe(Input::Eof);
    let opened =
        TpuartClient::connect_stream(stream, source(), TpuartMode::BusMonitor, timing()).await;
    assert!(opened.is_err());
    assert!(polled.load(Ordering::SeqCst));
    assert!(dropped.load(Ordering::SeqCst));
    assert!(written.lock().unwrap().is_empty());
}

#[tokio::test]
async fn startup_read_error_releases_stream_without_reset() {
    let (stream, polled, dropped, written, _) = probe(Input::Error);
    let opened =
        TpuartClient::connect_stream(stream, source(), TpuartMode::BusMonitor, timing()).await;
    assert!(
        matches!(opened, Err(BusError::Io(error)) if error.kind() == std::io::ErrorKind::BrokenPipe)
    );
    assert!(polled.load(Ordering::SeqCst));
    assert!(dropped.load(Ordering::SeqCst));
    assert!(written.lock().unwrap().is_empty());
}

#[tokio::test]
async fn cancelling_startup_settling_releases_stream_without_reset() {
    let (stream, polled, dropped, written, started) = probe(Input::Pending);
    let configured = Timing {
        initialize: Duration::from_secs(1),
        startup_quiet: Duration::from_millis(500),
        ..timing()
    };
    let task = tokio::spawn(TpuartClient::connect_stream(
        stream,
        source(),
        TpuartMode::BusMonitor,
        configured,
    ));
    started.await.unwrap();
    task.abort();
    assert!(matches!(task.await, Err(error) if error.is_cancelled()));
    assert!(polled.load(Ordering::SeqCst));
    assert!(dropped.load(Ordering::SeqCst));
    assert!(written.lock().unwrap().is_empty());
}

#[tokio::test]
async fn invalid_source_does_not_consume_startup_input() {
    let (stream, polled, dropped, written, _) = probe(Input::Continuous);
    let opened = TpuartClient::connect_stream(
        stream,
        IndividualAddress::from_raw(0),
        TpuartMode::BusMonitor,
        timing(),
    )
    .await;
    assert!(opened.is_err());
    assert!(!polled.load(Ordering::SeqCst));
    assert!(dropped.load(Ordering::SeqCst));
    assert!(written.lock().unwrap().is_empty());
}

async fn actor_stopped_before_reset(cancelled: bool) {
    let (stream, _, dropped, written, _) = probe(Input::Pending);
    let (_requests, queue) = mpsc::channel(1);
    let (stop, stopped) = watch::channel(false);
    if cancelled {
        stop.send_replace(true);
    }
    let (done_tx, done) = watch::channel(false);
    let (sender, _) = broadcast::channel(8);
    let events = Arc::new(Mutex::new(Events {
        sender,
        closed: false,
    }));
    let diagnostics = Arc::new(Mutex::new(TpuartDiagnostics::default()));
    let (ready_tx, ready) = oneshot::channel();
    let deadline = if cancelled {
        Instant::now() + Duration::from_secs(1)
    } else {
        Instant::now()
    };
    run(
        stream,
        source(),
        TpuartMode::BusMonitor,
        timing(),
        queue,
        stopped,
        done_tx,
        events.clone(),
        diagnostics,
        ready_tx,
        deadline,
    )
    .await;
    assert!(ready.await.unwrap().is_err());
    assert!(dropped.load(Ordering::SeqCst));
    assert!(written.lock().unwrap().is_empty());
    assert!(*done.borrow());
    assert!(events.lock().unwrap().closed);
    drop(stop);
}

#[tokio::test]
async fn expired_actor_budget_does_not_send_cleanup_as_its_first_command() {
    actor_stopped_before_reset(false).await;
}

#[tokio::test]
async fn cancelled_actor_does_not_send_cleanup_as_its_first_command() {
    actor_stopped_before_reset(true).await;
}
