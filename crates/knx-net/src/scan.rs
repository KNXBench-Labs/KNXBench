//! Line scan: probes a range of addresses and the timeout policy behind it.
//!
//! Builds on `knx_core::scan::ScanPlan` (Task 1), which guarantees an
//! excluded address is never generated into the candidate list in the
//! first place, and on `cemi::Tpci` (Task 2), whose connection-oriented
//! variants (`Connect`/`NumberedData`/`Disconnect`) this probe rides on.
//! See `docs/superpowers/plans/2026-09-12-line-scan-implementation.md`.

use std::time::Duration;

use knx_core::scan::{ScanPlan, ScanPlanError};
use knx_core::IndividualAddress;
use tokio::sync::broadcast;

use crate::cemi::{ApplicationService, Destination, LDataMessageKind, Tpci};
use crate::client::{BusError, TunnelClient, TunnelEvent};

/// How a line scan probes one address, and how long it is willing to wait
/// for an answer before moving on. See the accessor docs below for which
/// parts of this are `[D]` (documented in the Standard) versus `[A]` (an
/// assumption resting on a `[V]` local measurement) — Global Constraint 4
/// forbids promoting one to the other. Fields are private and reached
/// through `new`/the accessors so an invalid value (see
/// `ProbePolicyError`) cannot be constructed at all, rather than silently
/// rewritten into a valid one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbePolicy {
    response_timeout: Duration,
    vacant_confirmations: u8,
    inter_probe_pause: Duration,
}

/// Why `ProbePolicy::new` refused a set of values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbePolicyError {
    /// `vacant_confirmations: 0` cannot mean what it says —
    /// `probe_address` always runs at least one pass, so a caller asking
    /// for zero confirmations has a bug, and deserves to be told rather
    /// than silently answered with 1 (review finding, T17 fix round 1,
    /// #12).
    ZeroVacantConfirmations,
}

impl std::fmt::Display for ProbePolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbePolicyError::ZeroVacantConfirmations => write!(
                f,
                "vacant_confirmations must be at least 1 (0 would mean a probe pass never runs)"
            ),
        }
    }
}

impl std::error::Error for ProbePolicyError {}

impl ProbePolicy {
    /// Builds a policy, rejecting `vacant_confirmations: 0` instead of
    /// quietly treating it as 1.
    pub fn new(
        response_timeout: Duration,
        vacant_confirmations: u8,
        inter_probe_pause: Duration,
    ) -> Result<Self, ProbePolicyError> {
        if vacant_confirmations == 0 {
            return Err(ProbePolicyError::ZeroVacantConfirmations);
        }
        Ok(Self {
            response_timeout,
            vacant_confirmations,
            inter_probe_pause,
        })
    }

    /// How long one probe pass waits for an answer before giving up.
    ///
    /// `[D]` `03_03_04 Transport Layer v01.02.03 AS`, clause 4 "Parameters
    /// of Transport Layer", page 16 of 38: "connection timeout: time
    /// interval of 6 s; timeout to breakdown a connection" — this is the
    /// Standard's own budget for a connection with no reply, not a client
    /// invention. Default 6000 ms (see `Default` below).
    ///
    /// A `[V]` measurement on one real installation found an occupied
    /// address's round trip taking as long as 6016.5 ms — a shorter
    /// timeout (e.g. 1000 ms) reports that device `Vacant`, which is a
    /// present device silently dropped from the result. A caller who
    /// still wants the faster, lossier setting may build one; this type
    /// does not forbid it, but it must be presented as "no answer within
    /// N ms", not as "absent".
    pub fn response_timeout(&self) -> Duration {
        self.response_timeout
    }

    /// How many probe passes (`T_Connect` -> read -> `T_Disconnect`) run
    /// before a silent address is reported `Vacant`. Always at least 1 —
    /// `ProbePolicy::new` refuses to build a policy with 0.
    ///
    /// `[A]`, resting on the same `[V]` measurement above: repeating a
    /// too-short `response_timeout` does not rescue a consistently slow
    /// device, so the default of 1 assumes `response_timeout` is already
    /// the Standard's own `connection_timeout` and needs no repetition.
    pub fn vacant_confirmations(&self) -> u8 {
        self.vacant_confirmations
    }

    /// Pause between probe passes: between confirmation attempts on one
    /// address, and between successive addresses in a `scan_line` run.
    ///
    /// `[A]` — not itself standardized; chosen to avoid hammering the bus
    /// with back-to-back connection attempts. Default 100 ms.
    pub fn inter_probe_pause(&self) -> Duration {
        self.inter_probe_pause
    }
}

impl Default for ProbePolicy {
    fn default() -> Self {
        Self {
            response_timeout: Duration::from_millis(6000),
            vacant_confirmations: 1,
            inter_probe_pause: Duration::from_millis(100),
        }
    }
}

/// Conservative wall-clock preview for a sequential line scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanEstimate {
    candidate_count: usize,
    worst_case: Duration,
}

impl ScanEstimate {
    pub fn new(plan: &ScanPlan, policy: &ProbePolicy) -> Self {
        let candidates = u32::try_from(plan.addresses().len()).unwrap_or(u32::MAX);
        let confirmations = u32::from(policy.vacant_confirmations());
        let timeout = policy
            .response_timeout()
            .saturating_mul(candidates.saturating_mul(confirmations));
        let between_confirmation_pauses =
            candidates.saturating_mul(confirmations.saturating_sub(1));
        let between_address_pauses = candidates.saturating_sub(1);
        let pauses = policy
            .inter_probe_pause()
            .saturating_mul(between_confirmation_pauses.saturating_add(between_address_pauses));
        Self {
            candidate_count: plan.addresses().len(),
            worst_case: timeout.saturating_add(pauses),
        }
    }

    pub fn candidate_count(self) -> usize {
        self.candidate_count
    }

    pub fn worst_case(self) -> Duration {
        self.worst_case
    }
}

/// What one probe learned about an individual address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// The device answered `A_DeviceDescriptor_Response`. `mask_version`
    /// is `Some` when the response carried the two-octet DD0 payload (the
    /// Mask Version — RESEARCH.md §8.5, `[D]` `03_01_02 Glossary
    /// v01.05.03 AS.md:236`); `None` for any other payload shape. Only
    /// the fact that the device answered is preserved for that shape —
    /// the raw octets themselves are not retained (Global Constraint 3:
    /// honestly labelled as "not kept", not misdescribed as "preserved").
    Occupied { mask_version: Option<u16> },
    /// The device answered with `T_Disconnect` and no descriptor —
    /// present on the bus, but not answering this probe. `[D]`
    /// `03_05_02 Management Procedures v02.01.02 AS` §2.19's own
    /// possible-reaction list distinguishes this from `Vacant`: reporting
    /// it as `Vacant` would delete a live device from the result, which
    /// this task's brief calls the single most damaging bug it can ship.
    OccupiedBusy,
    /// A positive `L_Data.con` came back for the `T_Connect` this probe
    /// sent to the address, and nothing else did within the window.
    ///
    /// The Layer-2 acknowledge is the Standard's own presence signal:
    /// `[D]` `03_05_02 Management Procedures v02.01.02 AS` §2.19 —
    /// *"If a device that occupies IA_test is present on the network …
    /// it shall have no other reaction on the bus than the Layer-2
    /// acknowledge that initiates the above A_Connect.Lcon"*; `[D]`
    /// `03_06_03 EMI_IMI v01.04.02 AS` §4.1.5.3.4 — the cEMI `L_Data.con`
    /// carries exactly that acknowledge outcome back to the client: its
    /// Confirm flag is cleared when the cEMI Server "is satisfied with
    /// the transmission", and *"if a message is sent onto a medium with
    /// immediate acknowledge (L2-acknowledge) the confirmation message is
    /// normally generated after receiving this immediate acknowledge"*.
    ///
    /// A device present but silent at the application layer is honestly
    /// occupancy, not absence: folding this into `Vacant` would delete a
    /// live device from the result the same way ignoring `OccupiedBusy`
    /// would, and folding it into `Occupied` would misreport that a
    /// descriptor was actually read.
    OccupiedSilent,
    /// No answer within `response_timeout`, `vacant_confirmations` times
    /// in a row: no descriptor, no `T_Disconnect`, and either a negative
    /// `L_Data.con` for the `T_Connect` (the Standard's own "not
    /// occupied" conclusion — same citations as `OccupiedSilent`) or no
    /// confirmation at all within the window. The two are not the same
    /// evidence — a negative confirm is an affirmative signal, an absent
    /// one is simply nothing having arrived yet — but this task's outcome
    /// set has no separate "no evidence collected" variant, so both are
    /// reported the same way; see `probe_once`'s own comment on the
    /// distinction.
    Vacant,
    /// The probe could not decide. The tunnel's event channel lagged
    /// (`tokio::sync::broadcast::error::RecvError::Lagged`) during the
    /// probe window, meaning one or more frames were dropped before this
    /// probe could read them — possibly including the very descriptor
    /// response, disconnect, or connection confirm that would have
    /// settled the verdict. Reporting this as `Vacant` would silently
    /// launder "we do not know" into "absent" (Global Constraint 3);
    /// this variant exists so the scan can say so honestly instead of
    /// guessing. See `probe_once`.
    Indeterminate,
    /// The scanner's own tunnelling connection
    /// (`TunnelClient::assigned_address`). Never probed — RESEARCH.md
    /// §8.5 Finding 2: the gateway hands this address over during
    /// connection setup, no heuristic needed.
    SelfAddress,
}

/// What a scan procedure needs from its transport, abstracted so tests can
/// supply an in-process fake instead of a real `UdpSocket`.
///
/// `TunnelClient::send` (as it existed before this task) hardcoded
/// `Tpci::UnnumberedData` and exposed only `(destination, service)` — not
/// enough for the `T_Connect`/numbered `T_Data_Connected`/`T_Disconnect`
/// sequence a line scan needs, and its concrete socket made a fake-tunnel
/// unit test impossible. `send_frame` below is the general path
/// `TunnelClient::send` now delegates to.
#[allow(async_fn_in_trait)]
pub trait ScanTransport {
    /// The individual address the gateway assigned this connection.
    fn assigned_address(&self) -> IndividualAddress;
    /// Subscribes to telegrams (and connection-closed notice) received on
    /// this transport.
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent>;
    /// Sends one `L_Data.req` with the given transport (TPCI) and
    /// application service.
    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError>;
}

impl ScanTransport for TunnelClient {
    fn assigned_address(&self) -> IndividualAddress {
        TunnelClient::assigned_address(self)
    }

    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        TunnelClient::subscribe(self)
    }

    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        TunnelClient::send_frame(self, destination, transport, service).await
    }
}

/// Why `scan_line` could not complete: either the plan itself was unsafe
/// to run (`ScanPlan::verify` failed — Global Constraint 1's assertion),
/// or the transport broke mid-scan. `Transport` carries every result
/// already collected before the failure — up to tens of minutes of
/// scanning — so a caller is not forced to throw a near-complete scan
/// away over one late gateway hiccup.
#[derive(Debug)]
pub enum ScanError {
    Plan(ScanPlanError),
    Transport {
        source: BusError,
        completed: Vec<(IndividualAddress, ProbeOutcome)>,
    },
}

impl From<ScanPlanError> for ScanError {
    fn from(err: ScanPlanError) -> Self {
        ScanError::Plan(err)
    }
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanError::Plan(err) => write!(f, "scan plan is not safe to run: {err}"),
            ScanError::Transport { source, completed } => write!(
                f,
                "scan transport failed after {} address(es) probed: {source}",
                completed.len()
            ),
        }
    }
}

impl std::error::Error for ScanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ScanError::Plan(err) => Some(err),
            ScanError::Transport { source, .. } => Some(source),
        }
    }
}

/// Runs `03_05_02 Management Procedures v02.01.02 AS` §2.19's
/// (`NM_IndividualAddress_Check`) connection-oriented device check
/// against one address: `T_Connect` -> `A_DeviceDescriptor_Read(0)` as
/// `T_Data_Connected` -> read the answer -> `T_Disconnect`, best-effort,
/// no confirmation expected (`03_03_04 Transport Layer v01.02.03 AS`
/// §3.8). Repeats up to `policy.vacant_confirmations()` times, pausing
/// `policy.inter_probe_pause()` between passes, before reporting
/// `Vacant`.
///
/// `addr == transport.assigned_address()` short-circuits to `SelfAddress`
/// without sending a single frame: that address is this scan's own
/// tunnelling connection, not a device to probe.
pub async fn probe_address(
    transport: &impl ScanTransport,
    addr: IndividualAddress,
    policy: &ProbePolicy,
) -> Result<ProbeOutcome, BusError> {
    if addr == transport.assigned_address() {
        return Ok(ProbeOutcome::SelfAddress);
    }

    let attempts = policy.vacant_confirmations();
    for attempt in 0..attempts {
        if let Some(outcome) = probe_once(transport, addr, policy.response_timeout()).await? {
            return Ok(outcome);
        }
        if attempt + 1 < attempts {
            tokio::time::sleep(policy.inter_probe_pause()).await;
        }
    }
    Ok(ProbeOutcome::Vacant)
}

/// What `probe_once`'s wait loop knows once its window has closed, given
/// only what a positive/negative `L_Data.con` for the `T_Connect` it sent
/// established (no application-layer answer arrived, or this would not
/// be getting called — see the loop below). A positive confirm is
/// decisive by itself; a negative one, or none at all, is not — both
/// leave `None`, a candidate for another confirmation pass and,
/// eventually, `Vacant`.
fn deadline_verdict(connect_confirm: Option<bool>) -> Option<ProbeOutcome> {
    match connect_confirm {
        Some(true) => Some(ProbeOutcome::OccupiedSilent),
        Some(false) | None => None,
    }
}

/// One `T_Connect` -> read -> `T_Disconnect` pass. `Ok(None)` means no
/// conclusive verdict yet within `timeout` — a candidate for another
/// confirmation pass. Never `None` once real evidence (a descriptor, a
/// disconnect, a positive connect confirm, or a channel lag) has been
/// seen.
///
/// Noticed, not missed: a received `T_Data_Connected` should get a
/// `T_ACK` back (Transport Layer v01.02.03 AS §2's receiver obligation).
/// This does not send one — the `T_Disconnect` this function sends right
/// after reading the answer wins the race against the device's own
/// acknowledge timer in practice, so nothing repeats and nothing breaks,
/// but a future reader should not mistake the omission for an oversight.
async fn probe_once(
    transport: &impl ScanTransport,
    addr: IndividualAddress,
    timeout: Duration,
) -> Result<Option<ProbeOutcome>, BusError> {
    // Subscribed before either frame is sent, so a reply that arrives
    // between the two sends (or immediately after the second) is never
    // missed.
    let mut events = transport.subscribe();

    transport
        .send_frame(
            Destination::Individual(addr),
            Tpci::Connect,
            ApplicationService::NoApplicationPdu,
        )
        .await?;
    transport
        .send_frame(
            Destination::Individual(addr),
            Tpci::NumberedData { seq: 0 },
            ApplicationService::DeviceDescriptorRead { descriptor_type: 0 },
        )
        .await?;

    // Whether an `L_Data.con` came back for the `T_Connect` above, and
    // which way: `Some(true)` positive (Layer-2 acknowledged — §2.19's
    // own presence signal, see `ProbeOutcome::OccupiedSilent`),
    // `Some(false)` negative, `None` no confirmation arrived within the
    // window at all. Recorded, never acted on early: ending the probe
    // the moment this arrives would very likely cut a scan from minutes
    // to seconds (the confirm arrives in milliseconds), which is a real
    // candidate for a later cycle — but not on today's untested inference
    // about how quickly a negative confirm actually comes back, so this
    // still waits out the full window regardless of what `connect_confirm`
    // says.
    let mut connect_confirm: Option<bool> = None;

    let deadline = tokio::time::Instant::now() + timeout;
    let outcome = loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break deadline_verdict(connect_confirm);
        }
        match tokio::time::timeout(remaining, events.recv()).await {
            Ok(Ok(TunnelEvent::Telegram(frame))) => match frame.kind {
                LDataMessageKind::Confirmation { error } => {
                    // A confirmation's `source` is *our own* tunnelling
                    // address — it echoes the source of the request it
                    // confirms (`03_06_03 EMI_IMI v01.04.02 AS`
                    // §4.1.5.3.4), not the probed device's. Matching on
                    // `destination`/`transport` instead is what actually
                    // identifies this as the confirm for our `T_Connect`
                    // to `addr`.
                    if frame.destination == Destination::Individual(addr)
                        && matches!(frame.transport, Tpci::Connect)
                    {
                        connect_confirm = Some(!error);
                    }
                    continue; // never a verdict by itself: keep waiting out the window
                }
                LDataMessageKind::Indication => {
                    if frame.source != addr {
                        continue; // not this probe's device: keep waiting out the deadline
                    }
                    match (frame.transport, &frame.service) {
                        (
                            Tpci::NumberedData { .. },
                            ApplicationService::DeviceDescriptorResponse { data, .. },
                        ) => {
                            break Some(ProbeOutcome::Occupied {
                                mask_version: mask_version_from(data),
                            });
                        }
                        (Tpci::Disconnect, _) => break Some(ProbeOutcome::OccupiedBusy),
                        _ => continue, // some other TPDU from this address: not the answer, keep waiting
                    }
                }
                LDataMessageKind::Request => continue, // our own outbound frame echoed back: not evidence
            },
            Ok(Ok(TunnelEvent::Closed)) => {
                return Err(BusError::Protocol(
                    "tunnel closed while waiting for a probe reply".to_string(),
                ));
            }
            Ok(Err(broadcast::error::RecvError::Lagged(_))) => {
                // A `Lagged(n)` means n events were dropped before this
                // receiver saw them — possibly including the very
                // descriptor response, disconnect, or connect confirm
                // this probe is waiting for. Continuing to wait out the
                // deadline and then reporting `Vacant` would be
                // indistinguishable from "we checked and it's absent",
                // which is not what happened, so this ends the pass
                // immediately as `Indeterminate` instead of gambling that
                // nothing important was in the gap.
                break Some(ProbeOutcome::Indeterminate);
            }
            Ok(Err(broadcast::error::RecvError::Closed)) => {
                return Err(BusError::Protocol(
                    "tunnel event channel closed while waiting for a probe reply".to_string(),
                ));
            }
            Err(_) => break deadline_verdict(connect_confirm), // response_timeout elapsed
        }
    };

    // Best-effort, no confirmation expected (§3.8) — a failure here does
    // not change the outcome already determined above, and a device that
    // already sent its own T_Disconnect (OccupiedBusy) simply has nothing
    // left to tear down.
    let _ = transport
        .send_frame(
            Destination::Individual(addr),
            Tpci::Disconnect,
            ApplicationService::NoApplicationPdu,
        )
        .await;

    Ok(outcome)
}

/// DD0's payload is the two-octet Mask Version (RESEARCH.md §8.5). Any
/// other length is not retained: this returns `None` and the raw octets
/// are dropped. Only the fact that the device answered at all survives,
/// via `ProbeOutcome::Occupied` itself — the enum shape this task
/// specifies has no field to keep unrecognized payload bytes in (a
/// retained-bytes variant is a Task 5 note, not this task's fix).
fn mask_version_from(data: &[u8]) -> Option<u16> {
    match data {
        [hi, lo] => Some(u16::from_be_bytes([*hi, *lo])),
        _ => None,
    }
}

/// Probes every address in `plan`, sequentially — the tunnelling protocol
/// allows only one outstanding request per direction (Tunnelling
/// v01.07.01 AS §2.6, already relied on by `client.rs`), so there is no
/// concurrent shape to exploit here even if this code wanted one.
///
/// Calls `plan.verify()` first and returns its error unrun: Global
/// Constraint 1's bus-safety property is that an excluded address is
/// asserted before a scan starts, not filtered out afterwards, and this
/// is where that assertion happens for a scan.
///
/// `progress` is called once per probed address (including `SelfAddress`)
/// as its outcome becomes known, so a caller can show something during
/// what may be several minutes of scanning — and, if the transport fails
/// partway through, the same results are also returned inside
/// `ScanError::Transport`, so a caller relying on the return value rather
/// than the callback does not lose them either.
pub async fn scan_line(
    transport: &impl ScanTransport,
    plan: &ScanPlan,
    policy: &ProbePolicy,
    mut progress: impl FnMut(IndividualAddress, ProbeOutcome),
) -> Result<Vec<(IndividualAddress, ProbeOutcome)>, ScanError> {
    plan.verify()?;

    let addresses = plan.addresses();
    let mut results = Vec::with_capacity(addresses.len());
    for (index, &addr) in addresses.iter().enumerate() {
        let outcome = match probe_address(transport, addr, policy).await {
            Ok(outcome) => outcome,
            Err(source) => {
                return Err(ScanError::Transport {
                    source,
                    completed: results,
                });
            }
        };
        progress(addr, outcome);
        results.push((addr, outcome));
        if outcome != ProbeOutcome::SelfAddress && index + 1 < addresses.len() {
            tokio::time::sleep(policy.inter_probe_pause()).await;
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cemi::{LDataFrame, LDataMessageKind};
    use std::sync::Mutex as StdMutex;
    use tokio::sync::Mutex as AsyncMutex;

    fn addr(area: u8, line: u8, device: u8) -> IndividualAddress {
        IndividualAddress::new(area, line, device).unwrap()
    }

    /// A scripted per-address reply, applied once per probe pass a fake
    /// transport receives a `T_Connect`/`A_DeviceDescriptor_Read` pair
    /// for.
    #[derive(Debug, Clone)]
    enum ScriptedReply {
        /// Answer with `A_DeviceDescriptor_Response`, seq 0, this mask
        /// version.
        Descriptor(u16),
        /// Answer with `T_Disconnect` and nothing else.
        Disconnect,
    }

    /// What `L_Data.con` (if any) `FakeTransport` sends back for a
    /// scripted address's `T_Connect`.
    #[derive(Debug, Clone, Copy)]
    enum ConnectConfirm {
        Positive,
        Negative,
    }

    struct FakeTransport {
        assigned: IndividualAddress,
        tx: broadcast::Sender<TunnelEvent>,
        /// One scripted reply per (address, pass) — consumed in order for
        /// that address, so a test can prove exactly how many passes ran.
        script: StdMutex<std::collections::HashMap<IndividualAddress, Vec<ScriptedReply>>>,
        /// `L_Data.con` to emit for an address's `T_Connect`, if any.
        confirm_script: StdMutex<std::collections::HashMap<IndividualAddress, ConnectConfirm>>,
        /// How many unrelated filler frames to broadcast the moment an
        /// address's `T_Connect` is sent — enough to overflow the
        /// channel's capacity before the probe's own subscriber ever
        /// reads one, deterministically producing `RecvError::Lagged`.
        lag_script: StdMutex<std::collections::HashMap<IndividualAddress, u32>>,
        /// Extra frames to emit the moment an address's `T_Connect` is
        /// sent, ahead of any scripted reply — used to model a frame from
        /// some other device on the bus arriving during the probe window.
        decoys: StdMutex<std::collections::HashMap<IndividualAddress, Vec<LDataFrame>>>,
        /// Addresses whose `T_Connect` send must fail outright — models a
        /// transport-layer failure (e.g. the socket dying) partway through
        /// a scan, for `ScanError::Transport { completed }` coverage.
        fail_on_connect: StdMutex<std::collections::HashSet<IndividualAddress>>,
        /// Every frame sent, in order, for tests to inspect.
        sent: AsyncMutex<Vec<(Destination, Tpci, ApplicationService)>>,
    }

    impl FakeTransport {
        fn new(assigned: IndividualAddress) -> Self {
            let (tx, _rx) = broadcast::channel(64);
            Self {
                assigned,
                tx,
                script: StdMutex::new(std::collections::HashMap::new()),
                confirm_script: StdMutex::new(std::collections::HashMap::new()),
                lag_script: StdMutex::new(std::collections::HashMap::new()),
                decoys: StdMutex::new(std::collections::HashMap::new()),
                fail_on_connect: StdMutex::new(std::collections::HashSet::new()),
                sent: AsyncMutex::new(Vec::new()),
            }
        }

        /// Makes `addr`'s `T_Connect` send fail with `BusError::Timeout`
        /// instead of succeeding — a transport failure, not a probe
        /// outcome.
        fn fail_connect(&self, addr: IndividualAddress) {
            self.fail_on_connect.lock().unwrap().insert(addr);
        }

        fn script_for(&self, addr: IndividualAddress, replies: Vec<ScriptedReply>) {
            self.script.lock().unwrap().insert(addr, replies);
        }

        fn confirm_for(&self, addr: IndividualAddress, confirm: ConnectConfirm) {
            self.confirm_script.lock().unwrap().insert(addr, confirm);
        }

        /// Broadcasts `dummy_frames` filler telegrams the moment `addr`'s
        /// `T_Connect` is sent — `dummy_frames` must exceed the channel's
        /// capacity (64) for a subscriber that has not yet read anything
        /// to be guaranteed a lag.
        fn flood_on_connect(&self, addr: IndividualAddress, dummy_frames: u32) {
            self.lag_script.lock().unwrap().insert(addr, dummy_frames);
        }

        /// Queues `frame` to be broadcast the moment `addr`'s `T_Connect`
        /// is sent, ahead of any scripted reply.
        fn decoy_on_connect(&self, addr: IndividualAddress, frame: LDataFrame) {
            self.decoys
                .lock()
                .unwrap()
                .entry(addr)
                .or_default()
                .push(frame);
        }

        async fn sent_frames(&self) -> Vec<(Destination, Tpci, ApplicationService)> {
            self.sent.lock().await.clone()
        }
    }

    impl ScanTransport for FakeTransport {
        fn assigned_address(&self) -> IndividualAddress {
            self.assigned
        }

        fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
            self.tx.subscribe()
        }

        async fn send_frame(
            &self,
            destination: Destination,
            transport: Tpci,
            service: ApplicationService,
        ) -> Result<(), BusError> {
            self.sent
                .lock()
                .await
                .push((destination, transport, service.clone()));

            let Destination::Individual(target) = destination else {
                return Ok(());
            };

            if matches!(transport, Tpci::Connect)
                && self.fail_on_connect.lock().unwrap().contains(&target)
            {
                return Err(BusError::Timeout);
            }

            if matches!(transport, Tpci::Connect) {
                if let Some(&flood) = self.lag_script.lock().unwrap().get(&target) {
                    for _ in 0..flood {
                        let _ = self.tx.send(TunnelEvent::Telegram(LDataFrame {
                            kind: LDataMessageKind::Indication,
                            source: target,
                            destination: Destination::Individual(self.assigned),
                            transport: Tpci::UnnumberedData,
                            service: ApplicationService::NoApplicationPdu,
                        }));
                    }
                }
                if let Some(decoys) = self.decoys.lock().unwrap().remove(&target) {
                    for frame in decoys {
                        let _ = self.tx.send(TunnelEvent::Telegram(frame));
                    }
                }
                if let Some(confirm) = self.confirm_script.lock().unwrap().get(&target).copied() {
                    let frame = LDataFrame {
                        kind: LDataMessageKind::Confirmation {
                            error: matches!(confirm, ConnectConfirm::Negative),
                        },
                        source: self.assigned,
                        destination: Destination::Individual(target),
                        transport: Tpci::Connect,
                        service: ApplicationService::NoApplicationPdu,
                    };
                    let _ = self.tx.send(TunnelEvent::Telegram(frame));
                }
                return Ok(());
            }

            // Only the second frame of a pass (the DeviceDescriptorRead)
            // triggers a scripted reply — T_Connect (handled above) and
            // T_Disconnect are otherwise fire-and-forget from this fake's
            // point of view.
            if !matches!(service, ApplicationService::DeviceDescriptorRead { .. }) {
                return Ok(());
            }

            let reply = {
                let mut script = self.script.lock().unwrap();
                script.get_mut(&target).and_then(|replies| {
                    if replies.is_empty() {
                        None
                    } else {
                        Some(replies.remove(0))
                    }
                })
            };

            match reply {
                Some(ScriptedReply::Descriptor(mask_version)) => {
                    let bytes = mask_version.to_be_bytes();
                    let frame = LDataFrame {
                        kind: LDataMessageKind::Indication,
                        source: target,
                        destination: Destination::Individual(self.assigned),
                        transport: Tpci::NumberedData { seq: 0 },
                        service: ApplicationService::DeviceDescriptorResponse {
                            descriptor_type: 0,
                            data: bytes.to_vec(),
                        },
                    };
                    let _ = self.tx.send(TunnelEvent::Telegram(frame));
                }
                Some(ScriptedReply::Disconnect) => {
                    let frame = LDataFrame {
                        kind: LDataMessageKind::Indication,
                        source: target,
                        destination: Destination::Individual(self.assigned),
                        transport: Tpci::Disconnect,
                        service: ApplicationService::NoApplicationPdu,
                    };
                    let _ = self.tx.send(TunnelEvent::Telegram(frame));
                }
                None => {} // no script entry left for this address: silence
            }
            Ok(())
        }
    }

    fn fast_policy() -> ProbePolicy {
        // Real-world default is 6s; tests use a short timeout so the
        // silence/timeout paths don't make the suite slow. The policy
        // ruling itself (6000/1/100) is asserted separately below.
        ProbePolicy::new(Duration::from_millis(50), 1, Duration::from_millis(5))
            .expect("1 is a valid vacant_confirmations")
    }

    #[test]
    fn default_policy_matches_the_ruling() {
        let policy = ProbePolicy::default();
        assert_eq!(policy.response_timeout(), Duration::from_millis(6000));
        assert_eq!(policy.vacant_confirmations(), 1);
        assert_eq!(policy.inter_probe_pause(), Duration::from_millis(100));
    }

    #[test]
    fn vacant_confirmations_zero_is_rejected_at_construction() {
        let result = ProbePolicy::new(Duration::from_millis(6000), 0, Duration::from_millis(100));
        assert_eq!(
            result,
            Err(ProbePolicyError::ZeroVacantConfirmations),
            "0 confirmations must be a construction error, not silently rewritten to 1"
        );
    }

    #[tokio::test]
    async fn occupied_address_reports_its_mask_version() {
        let target = addr(1, 1, 2);
        let transport = FakeTransport::new(addr(1, 1, 1));
        transport.script_for(target, vec![ScriptedReply::Descriptor(0x0705)]);

        let outcome = probe_address(&transport, target, &fast_policy())
            .await
            .unwrap();

        assert_eq!(
            outcome,
            ProbeOutcome::Occupied {
                mask_version: Some(0x0705)
            }
        );
    }

    #[tokio::test]
    async fn a_disconnect_with_no_descriptor_is_busy_not_vacant() {
        let target = addr(1, 1, 3);
        let transport = FakeTransport::new(addr(1, 1, 1));
        transport.script_for(target, vec![ScriptedReply::Disconnect]);

        let outcome = probe_address(&transport, target, &fast_policy())
            .await
            .unwrap();

        assert_eq!(
            outcome,
            ProbeOutcome::OccupiedBusy,
            "a received T_Disconnect with no descriptor answer must never be reported Vacant"
        );
    }

    #[tokio::test]
    async fn silence_is_vacant_and_every_confirmation_pass_actually_probes() {
        let target = addr(1, 1, 4);
        let transport = FakeTransport::new(addr(1, 1, 1));
        // No script entries at all -> every pass gets silence.
        let policy = ProbePolicy::new(Duration::from_millis(30), 2, Duration::from_millis(5))
            .expect("2 is a valid vacant_confirmations");

        let outcome = probe_address(&transport, target, &policy).await.unwrap();

        assert_eq!(outcome, ProbeOutcome::Vacant);
        let sent = transport.sent_frames().await;
        let read_count = sent
            .iter()
            .filter(|(_, _, service)| {
                matches!(service, ApplicationService::DeviceDescriptorRead { .. })
            })
            .count();
        assert_eq!(
            read_count, 2,
            "vacant_confirmations: 2 must run two full probe passes, not report vacant after one"
        );
    }

    #[tokio::test]
    async fn a_positive_l2_confirm_with_no_application_answer_is_occupied_but_silent() {
        let target = addr(1, 1, 5);
        let transport = FakeTransport::new(addr(1, 1, 1));
        transport.confirm_for(target, ConnectConfirm::Positive);
        // No ScriptedReply at all: the application layer never answers.

        let outcome = probe_address(&transport, target, &fast_policy())
            .await
            .unwrap();

        assert_eq!(
            outcome,
            ProbeOutcome::OccupiedSilent,
            "a positive L_Data.con for our T_Connect is the Standard's own presence signal \
             (03_05_02 Management Procedures v02.01.02 AS §2.19); an application-layer silence \
             on top of it must not be reported Vacant"
        );
    }

    #[tokio::test]
    async fn a_negative_l2_confirm_is_vacant_like_total_silence() {
        let target = addr(1, 1, 6);
        let transport = FakeTransport::new(addr(1, 1, 1));
        transport.confirm_for(target, ConnectConfirm::Negative);

        let outcome = probe_address(&transport, target, &fast_policy())
            .await
            .unwrap();

        assert_eq!(outcome, ProbeOutcome::Vacant);
    }

    #[tokio::test]
    async fn a_disconnect_from_an_unrelated_device_does_not_count_as_this_ones() {
        let target = addr(1, 1, 7);
        let bystander = addr(1, 1, 8);
        let transport = FakeTransport::new(addr(1, 1, 1));
        transport.script_for(target, vec![ScriptedReply::Descriptor(0x0705)]);
        // A third device's T_Disconnect, addressed to our own connection,
        // arrives during the window ahead of target's real answer.
        transport.decoy_on_connect(
            target,
            LDataFrame {
                kind: LDataMessageKind::Indication,
                source: bystander,
                destination: Destination::Individual(addr(1, 1, 1)),
                transport: Tpci::Disconnect,
                service: ApplicationService::NoApplicationPdu,
            },
        );

        let outcome = probe_address(&transport, target, &fast_policy())
            .await
            .unwrap();

        assert_eq!(
            outcome,
            ProbeOutcome::Occupied {
                mask_version: Some(0x0705)
            },
            "a T_Disconnect from an unrelated address must not be scored as this address's \
             OccupiedBusy"
        );
    }

    #[tokio::test]
    async fn a_lagged_event_channel_is_reported_indeterminate_not_vacant() {
        let target = addr(1, 1, 9);
        let transport = FakeTransport::new(addr(1, 1, 1));
        // FakeTransport's channel capacity is 64 (see `new`); flooding it
        // past that before the probe's own subscriber reads anything
        // guarantees RecvError::Lagged on its very first recv().
        transport.flood_on_connect(target, 128);
        // Even though the real answer is scripted, the lag must still win
        // — a lag can mean this exact reply was the one dropped.
        transport.script_for(target, vec![ScriptedReply::Descriptor(0x0705)]);

        let outcome = probe_address(&transport, target, &fast_policy())
            .await
            .unwrap();

        assert_eq!(
            outcome,
            ProbeOutcome::Indeterminate,
            "a lagged event channel means evidence may have been dropped; that must never be \
             reported as Vacant"
        );
    }

    #[tokio::test]
    async fn self_address_is_skipped_without_sending_a_single_frame() {
        let own = addr(1, 1, 1);
        let transport = FakeTransport::new(own);

        let outcome = probe_address(&transport, own, &fast_policy())
            .await
            .unwrap();

        assert_eq!(outcome, ProbeOutcome::SelfAddress);
        assert!(
            transport.sent_frames().await.is_empty(),
            "the scanner's own tunnelling connection must never be probed"
        );
    }

    #[tokio::test]
    async fn a_plan_that_fails_verify_aborts_before_any_frame_is_sent() {
        // The bus-safety test: a plan smuggling an excluded address must
        // never reach a single `send_frame` call. `ScanPlan`'s public API
        // can never produce this state — every constructor filters
        // exclusions while building the candidate list — so this uses
        // `unchecked_for_tests`, a `test-support`-gated escape hatch that
        // does not exist in a normal build (knx_core::scan's own doc
        // comment on it explains why it is not a production bypass).
        let smuggled = addr(1, 1, 220); // the one address this repo may name
        let plan = ScanPlan::unchecked_for_tests(
            vec![addr(1, 1, 1), smuggled, addr(1, 1, 2)],
            std::collections::HashSet::from([smuggled]),
        );

        let transport = FakeTransport::new(addr(1, 1, 1));
        let result = scan_line(&transport, &plan, &fast_policy(), |_, _| {}).await;

        assert!(matches!(
            result,
            Err(ScanError::Plan(ScanPlanError::ExcludedAddressInRange(a))) if a == smuggled
        ));
        assert!(
            transport.sent_frames().await.is_empty(),
            "verify() failing must abort before a single frame leaves the machine"
        );
    }

    /// Finding 6 (T17 fix round 2): the only `scan_line(` call in the test
    /// suite exercised the `Plan` abort path; nothing proved that a
    /// mid-scan *transport* failure keeps what it already found. This is
    /// the "never silently discard data" guarantee the CLI depends on —
    /// `apps/knx-cli/src/main.rs` prints `completed` on exactly this
    /// error variant.
    #[tokio::test]
    async fn a_transport_failure_mid_scan_preserves_every_result_gathered_so_far() {
        let occupied = addr(1, 1, 2);
        let vacant = addr(1, 1, 3);
        let failing = addr(1, 1, 4);

        let transport = FakeTransport::new(addr(1, 1, 1));
        transport.script_for(occupied, vec![ScriptedReply::Descriptor(0x0705)]);
        // `vacant` gets no script at all: silence, resolved to `Vacant`.
        transport.fail_connect(failing);

        let plan = ScanPlan::range(occupied, failing)
            .expect("occupied..=failing is a valid same-line range");

        let result = scan_line(&transport, &plan, &fast_policy(), |_, _| {}).await;

        match result {
            Err(ScanError::Transport { source, completed }) => {
                assert!(
                    matches!(source, BusError::Timeout),
                    "must surface the transport's own error, not paper over it"
                );
                assert_eq!(
                    completed,
                    vec![
                        (
                            occupied,
                            ProbeOutcome::Occupied {
                                mask_version: Some(0x0705)
                            }
                        ),
                        (vacant, ProbeOutcome::Vacant),
                    ],
                    "a transport failure on the third address must not discard the first \
                     two results, address and outcome both"
                );
            }
            other => panic!(
                "expected ScanError::Transport carrying the two results probed before the \
                 failure, got {other:?}"
            ),
        }
    }
}
