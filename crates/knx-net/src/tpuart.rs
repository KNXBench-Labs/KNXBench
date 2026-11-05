//! Async NCN5130 transparent serial transport with explicit passive and active link modes.
//!
//! Requires transparent USB serial firmware, not the Busware console or KNX USB HID.
//! Linux ports use raw 19200 8E1. Opening/closing may change driver modem lines;
//! this adapter never explicitly toggles DTR/RTS. Active mode auto-acknowledges
//! the source individual address AND all group telegrams (NCN5130 Set Address).
//! Only standard TP1 frames are supported. No automatic application retries.
//! Requires end-to-end NCN5130 9-bit UART with parity errors preserved, not its 8-bit mode.
//! Confirmation requires a complete matching echo on an ordered conforming stream.
//! L_Data.con is unnumbered: adversarial replay after an identical echo is indistinguishable.

mod codec;
#[cfg(target_os = "linux")]
mod tty;

use crate::{
    ApplicationService, BusError, Destination, LDataFrame, LDataMessageKind, ScanTransport, Tpci,
    TunnelEvent,
};
use codec::{protocol, Service};
use knx_core::{ContactableAddress, IndividualAddress};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{broadcast, mpsc, oneshot, watch};
use tokio::time::{timeout, timeout_at, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpuartMode {
    /// U_Busmon: receives telegrams and ACKs, never sends data/ACK commands.
    BusMonitor,
    /// Normal link mode: sets the source and enables hardware auto-acknowledgement.
    Active,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TpuartDiagnostics {
    /// Raw octets discarded before this session's first reset, never live telegrams.
    pub startup_discarded_bytes: u64,
    /// First terminal error, or explicit disconnect; retained after Closed.
    pub closure: Option<String>,
    /// Raw bus acknowledgements seen in passive mode (not L_Data.con).
    pub acknowledgements: u64,
    pub last_acknowledgement: Option<u8>,
}

#[derive(Clone, Copy)]
struct Timing {
    initialize: Duration,
    startup_quiet: Duration,
    send: Duration,
    frame: Duration,
    shutdown: Duration,
}
impl Default for Timing {
    fn default() -> Self {
        Self {
            initialize: Duration::from_secs(2),
            startup_quiet: Duration::from_millis(20),
            send: Duration::from_secs(2),
            frame: Duration::from_millis(250),
            shutdown: Duration::from_millis(300),
        }
    }
}

struct Request {
    wire: Vec<u8>,
    reply: oneshot::Sender<Result<(), BusError>>,
}

struct Events {
    sender: broadcast::Sender<TunnelEvent>,
    closed: bool,
}
impl Events {
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        if self.closed {
            let (sender, receiver) = broadcast::channel(1);
            let _ = sender.send(TunnelEvent::Closed);
            receiver
        } else {
            self.sender.subscribe()
        }
    }
    fn close(&mut self) {
        self.closed = true;
        let _ = self.sender.send(TunnelEvent::Closed);
    }
}

/// A single owned serial link. Drop/cancel closes it; concurrent sends are serialized.
/// A send succeeds only after positive L_Data.con, not after writing UART bytes.
pub struct TpuartClient {
    source: IndividualAddress,
    mode: TpuartMode,
    send_wait: Duration,
    requests: mpsc::Sender<Request>,
    events: Arc<Mutex<Events>>,
    stop: watch::Sender<bool>,
    done: watch::Receiver<bool>,
    diagnostics: Arc<Mutex<TpuartDiagnostics>>,
}

impl TpuartClient {
    /// Validate all caller data before opening a tty. Never selects/enumerates devices.
    pub async fn open(
        path: impl AsRef<Path>,
        source: IndividualAddress,
        mode: TpuartMode,
    ) -> Result<Self, BusError> {
        Self::validate_source(source)?;
        if path.as_ref().as_os_str().is_empty() {
            return Err(protocol("serial path must not be empty"));
        }
        #[cfg(target_os = "linux")]
        {
            let stream = tty::Tty::open(path.as_ref()).map_err(BusError::Io)?;
            Self::connect_stream(stream, source, mode, Timing::default()).await
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = mode;
            Err(protocol("serial TP-UART is supported only on Linux"))
        }
    }

    pub fn validate_source(source: IndividualAddress) -> Result<(), BusError> {
        if matches!(source.raw(), 0 | 0xffff) {
            return Err(protocol(
                "source must be a nonzero, non-broadcast individual address",
            ));
        }
        ContactableAddress::new(source).map_err(|e| protocol(e.to_string()))?;
        Ok(())
    }

    /// Preflight the identical codec used by send, without opening any resource.
    pub fn validate_frame(frame: &LDataFrame) -> Result<(), BusError> {
        Self::validate_source(frame.source)?;
        if let Destination::Individual(address) = frame.destination {
            ContactableAddress::new(address).map_err(|e| protocol(e.to_string()))?;
        }
        codec::encode(frame).map(|_| ())
    }

    async fn connect_stream<S>(
        mut stream: S,
        source: IndividualAddress,
        mode: TpuartMode,
        timing: Timing,
    ) -> Result<Self, BusError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        Self::validate_source(source)?;
        let initialize_deadline = Instant::now() + timing.initialize;
        let startup_discarded_bytes =
            settle_startup_input(&mut stream, timing.startup_quiet, initialize_deadline).await?;
        if Instant::now() >= initialize_deadline {
            return Err(BusError::Timeout);
        }
        let (requests, queue) = mpsc::channel(16);
        let (sender, _) = broadcast::channel(256);
        let events = Arc::new(Mutex::new(Events {
            sender,
            closed: false,
        }));
        let (stop, stopped) = watch::channel(false);
        let (done_tx, done) = watch::channel(false);
        let diagnostics = Arc::new(Mutex::new(TpuartDiagnostics {
            startup_discarded_bytes,
            ..Default::default()
        }));
        let (ready_tx, ready) = oneshot::channel();
        let client = Self {
            source,
            mode,
            send_wait: timing.send,
            requests,
            events: events.clone(),
            stop,
            done,
            diagnostics: diagnostics.clone(),
        };
        tokio::spawn(run(
            stream,
            source,
            mode,
            timing,
            queue,
            stopped,
            done_tx,
            events,
            diagnostics,
            ready_tx,
            initialize_deadline,
        ));
        match ready
            .await
            .map_err(|_| protocol("initialization task closed"))?
        {
            Ok(()) => Ok(client),
            Err(_) => {
                // A failed open returns only after bounded cleanup released the tty.
                let _ = client.disconnect().await;
                Err(client.closed_error())
            }
        }
    }

    pub fn assigned_address(&self) -> IndividualAddress {
        self.source
    }
    pub fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        // Subscription and terminal publication share one lock. A subscriber
        // either precedes Closed or receives a private replay after cleanup.
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .subscribe()
    }
    pub fn diagnostics(&self) -> TpuartDiagnostics {
        self.diagnostics
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub async fn send(
        &self,
        destination: Destination,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        self.send_frame(destination, Tpci::UnnumberedData, service)
            .await
    }

    pub async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        if self.mode == TpuartMode::BusMonitor {
            return Err(protocol("passive bus monitor cannot send"));
        }
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: self.source,
            destination,
            transport,
            service,
            control: None,
        };
        Self::validate_frame(&frame)?;
        let wire = codec::encode(&frame)?;
        if *self.stop.borrow() || *self.done.borrow() {
            return Err(self.closed_error());
        }
        // Cancellation anywhere (including queue admission) invalidates the session.
        // A late uncorrelated confirmation must never acknowledge a subsequent send.
        let mut cancel = CancelSend {
            stop: self.stop.clone(),
            armed: true,
        };
        let (reply, result) = oneshot::channel();
        let wait = async {
            self.requests
                .send(Request { wire, reply })
                .await
                .map_err(|_| self.closed_error())?;
            result.await.map_err(|_| self.closed_error())?
        };
        match timeout(self.send_wait, wait).await {
            Ok(result) => {
                cancel.armed = false;
                result
            }
            Err(_) => Err(BusError::Timeout), // CancelSend invalidates the whole link.
        }
    }

    fn closed_error(&self) -> BusError {
        protocol(
            self.diagnostics()
                .closure
                .unwrap_or_else(|| "transport closed".into()),
        )
    }

    /// Bounded actor shutdown resets the transceiver, then closes/restores the tty.
    /// A reset failure is reported; it is not evidence that auto-ACK was disabled.
    pub async fn disconnect(&self) -> Result<(), BusError> {
        self.stop.send_replace(true);
        let mut done = self.done.clone();
        while !*done.borrow_and_update() {
            done.changed().await.map_err(|_| self.closed_error())?;
        }
        match self.diagnostics().closure.as_deref() {
            Some("client disconnected") => Ok(()),
            _ => Err(self.closed_error()),
        }
    }
}
impl Drop for TpuartClient {
    fn drop(&mut self) {
        self.stop.send_replace(true);
    }
}
struct CancelSend {
    stop: watch::Sender<bool>,
    armed: bool,
}
impl Drop for CancelSend {
    fn drop(&mut self) {
        if self.armed {
            self.stop.send_replace(true);
        }
    }
}
impl ScanTransport for TpuartClient {
    fn assigned_address(&self) -> IndividualAddress {
        self.source
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.subscribe()
    }
    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        self.send_frame(destination, transport, service).await
    }
}

/// Drain pre-session USB/UART backlog, not protocol input after the first reset.
/// The quiet interval is a bounded settling policy, not a KNX silence guarantee.
async fn settle_startup_input<S: AsyncRead + Unpin>(
    stream: &mut S,
    quiet: Duration,
    deadline: Instant,
) -> Result<u64, BusError> {
    if quiet.is_zero() {
        return Err(protocol("startup quiet interval must be nonzero"));
    }
    let mut discarded = 0u64;
    let mut buffer = [0u8; 256];
    loop {
        if Instant::now() >= deadline {
            return Err(BusError::Timeout);
        }
        let idle_deadline = (Instant::now() + quiet).min(deadline);
        match timeout_at(idle_deadline, stream.read(&mut buffer)).await {
            Ok(Ok(0)) => {
                return Err(protocol("serial stream closed during startup settling"));
            }
            Ok(Ok(n)) => {
                discarded = discarded.saturating_add(n as u64);
                // An always-ready stream must not starve cancellation or the budget.
                tokio::task::yield_now().await;
            }
            Ok(Err(error)) => return Err(BusError::Io(error)),
            Err(_) if Instant::now() >= deadline => return Err(BusError::Timeout),
            Err(_) => return Ok(discarded),
        }
    }
}

async fn read_services<R: AsyncRead + Unpin>(
    mut reader: R,
    tx: mpsc::Sender<Result<Service, BusError>>,
    frame_wait: Duration,
) {
    let mut parser = codec::Parser::default();
    let mut deadline = None;
    loop {
        let next = async { reader.read_u8().await.map_err(BusError::Io) };
        let byte = match deadline {
            Some(at) => timeout_at(at, next)
                .await
                .map_err(|_| protocol("receive frame assembly timed out"))
                .and_then(|r| r),
            None => next.await,
        };
        let result = byte.and_then(|b| parser.push(b));
        match result {
            Ok(service) => {
                if parser.assembling() {
                    deadline.get_or_insert_with(|| Instant::now() + frame_wait);
                } else {
                    deadline = None;
                }
                if let Some(service) = service {
                    if tx.send(Ok(service)).await.is_err() {
                        return;
                    }
                }
            }
            Err(error) => {
                let _ = tx.send(Err(error)).await;
                return;
            }
        }
    }
}

struct Link<W> {
    writer: W,
    incoming: mpsc::Receiver<Result<Service, BusError>>,
    events: Arc<Mutex<Events>>,
    diagnostics: Arc<Mutex<TpuartDiagnostics>>,
    mode: TpuartMode,
    reset_attempted: bool,
}
impl<W: AsyncWrite + Unpin> Link<W> {
    async fn next(&mut self) -> Result<Service, BusError> {
        self.incoming
            .recv()
            .await
            .ok_or_else(|| protocol("serial reader closed"))?
    }
    fn background(&self, service: Service) -> Result<(), BusError> {
        match service {
            Service::Telegram(frame, _) => {
                let _ = self
                    .events
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .sender
                    .send(TunnelEvent::Telegram(frame));
            }
            Service::FrameState => {}
            Service::Acknowledgement(byte) if self.mode == TpuartMode::BusMonitor => {
                let mut diagnostics = self.diagnostics.lock().unwrap_or_else(|e| e.into_inner());
                diagnostics.acknowledgements = diagnostics.acknowledgements.saturating_add(1);
                diagnostics.last_acknowledgement = Some(byte);
            }
            Service::State(7) => {}
            other => return Err(protocol(format!("unexpected service {other:?}"))),
        }
        Ok(())
    }
    async fn write(&mut self, bytes: &[u8]) -> Result<(), BusError> {
        self.writer.write_all(bytes).await.map_err(BusError::Io)
    }
    async fn expect(&mut self, expected: Service) -> Result<(), BusError> {
        loop {
            let service = self.next().await?;
            if service == expected {
                return Ok(());
            }
            self.background(service)?;
        }
    }
    async fn initialize(
        &mut self,
        source: IndividualAddress,
        deadline: Instant,
    ) -> Result<(), BusError> {
        if Instant::now() >= deadline {
            return Err(BusError::Timeout);
        }
        self.reset_attempted = true;
        self.write(&[1]).await?;
        self.expect(Service::Reset).await?;
        self.write(&[2]).await?;
        self.expect(Service::State(7)).await?;
        match self.mode {
            TpuartMode::BusMonitor => self.write(&[5]).await?,
            TpuartMode::Active => {
                let [high, low] = source.raw().to_be_bytes();
                self.write(&[0xf1, high, low, 0]).await?;
                // Reset defaults: no polling, CRC or marker. Do not enable them.
                self.expect(Service::Configure(0x21)).await?;
            }
        }
        Ok(())
    }
    async fn transmit(&mut self, request: &mut Request) -> Result<(), BusError> {
        // Detect queued/unsolicited confirmations before beginning another request.
        while let Ok(service) = self.incoming.try_recv() {
            self.background(service?)?;
        }
        self.write(&request.wire).await?;
        let expected_echo: Vec<_> = request
            .wire
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| pair[1])
            .collect();
        let mut echoed = false;
        // NCN5130 returns every transmitted octet before its unnumbered con.
        // This admits an ordered conforming stream, NOT general replay protection:
        // an old con after an identical matching echo is indistinguishable.
        loop {
            tokio::select! {
                _ = request.reply.closed() => return Err(protocol("send cancelled; delivery is uncertain")),
                service = self.next() => match service? {
                    Service::Confirmation(true) if echoed => return Ok(()),
                    Service::Confirmation(true) => return Err(protocol("positive L_Data.con without complete matching transmission echo; delivery is uncertain")),
                    Service::Confirmation(false) => return Err(protocol("negative L_Data.con; not acknowledged")),
                    Service::Telegram(frame, tp) => {
                        echoed |= codec::matches_echo(&expected_echo, &tp);
                        self.background(Service::Telegram(frame, tp))?;
                    }
                    service => self.background(service)?,
                }
            }
        }
    }
    async fn reset(&mut self) -> Result<(), BusError> {
        self.write(&[1]).await?;
        loop {
            if self.next().await? == Service::Reset {
                return Ok(());
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
    stream: S,
    source: IndividualAddress,
    mode: TpuartMode,
    timing: Timing,
    mut queue: mpsc::Receiver<Request>,
    mut stopped: watch::Receiver<bool>,
    done: watch::Sender<bool>,
    events: Arc<Mutex<Events>>,
    diagnostics: Arc<Mutex<TpuartDiagnostics>>,
    ready: oneshot::Sender<Result<(), BusError>>,
    initialize_deadline: Instant,
) {
    let (reader, writer) = tokio::io::split(stream);
    let (incoming_tx, incoming) = mpsc::channel(256);
    let reader = tokio::spawn(read_services(reader, incoming_tx, timing.frame));
    let mut link = Link {
        writer,
        incoming,
        events,
        diagnostics: diagnostics.clone(),
        mode,
        reset_attempted: false,
    };
    let initialized = if Instant::now() >= initialize_deadline {
        Err(BusError::Timeout)
    } else {
        tokio::select! {
            biased;
            _ = stopped.changed() => Err(protocol("initialization cancelled")),
            result = timeout_at(initialize_deadline, link.initialize(source, initialize_deadline)) => result.unwrap_or(Err(BusError::Timeout)),
        }
    };
    let mut reason = initialized.as_ref().err().map(ToString::to_string);
    let initialized = initialized.is_ok();
    let _ = ready.send(match &reason {
        Some(reason) => Err(protocol(reason.clone())),
        None => Ok(()),
    });
    if initialized {
        reason = loop {
            tokio::select! {
                biased;
                _ = stopped.changed() => break None,
                service = link.next() => if let Err(error) = service.and_then(|s| link.background(s)) { break Some(error.to_string()); },
                request = queue.recv() => {
                    let Some(mut request) = request else { break None; };
                    let result = tokio::select! {
                        biased;
                        _ = stopped.changed() => Err(protocol("send cancelled; delivery is uncertain")),
                        result = timeout(timing.send, link.transmit(&mut request)) => result.unwrap_or(Err(BusError::Timeout)),
                    };
                    let failure = result.as_ref().err().map(ToString::to_string);
                    let _ = request.reply.send(result);
                    if failure.is_some() { break failure; }
                }
            }
        };
    }
    // Stop admitting sends before cleanup. Queued requests never reach the wire.
    queue.close();
    let cleanup = if link.reset_attempted {
        timeout(timing.shutdown, link.reset()).await
    } else {
        // No protocol command began, so cancellation/expiry must not cause one.
        Ok(Ok(()))
    };
    let reason = match cleanup {
        Ok(Ok(())) => reason.unwrap_or_else(|| "client disconnected".into()),
        Ok(Err(e)) => format!(
            "{}; reset on close failed: {e}",
            reason.unwrap_or_else(|| "client disconnected".into())
        ),
        Err(_) => format!(
            "{}; reset on close timed out (auto-ACK state unknown)",
            reason.unwrap_or_else(|| "client disconnected".into())
        ),
    };
    diagnostics
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .closure = Some(reason.clone());
    while let Ok(request) = queue.try_recv() {
        let _ = request.reply.send(Err(protocol(reason.clone())));
    }
    reader.abort();
    let _ = reader.await;
    // Drop both halves before publishing Closed/done (tty settings/fd are released).
    drop(link.writer);
    link.events
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .close();
    done.send_replace(true);
}

#[cfg(test)]
mod tests;
