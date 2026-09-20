//! The connection-oriented management session: the one door a write to a device goes through.
//!
//! Spec §11.1 asks for *"a connection-oriented session type"* carrying the
//! sequence numbering, the 3 s acknowledge time-out, the 6 s connection
//! time-out and `max_rep_count = 3` of **TL** clause 4. This is it, plus the
//! three things §10.3, §6.3 and §6.2 require of every connection:
//! authorise, assert Verify Mode, and never write without reading back.
//!
//! Two rules are structural rather than documented, and both are deliberate:
//!
//! * A session that was not handed a [`WriteAuthorisation`] has no write
//!   methods that will run — [`ManagementSession::read_only`] leaves the
//!   authorisation empty and every write returns
//!   [`SessionError::NoAuthorisation`] before touching the transport.
//! * A session whose transport is not a simulator refuses to write at all,
//!   in this phase, per §15 and §13 R11. That check is
//!   [`ManagementTransport::target_kind`], it defaults to
//!   [`TargetKind::Hardware`] for every real transport, and it is asserted
//!   at construction *and* again inside every write.
//!
//! Nothing here decides *what* to download; that is the sequencer's job.
//! This layer knows how to ask one device one thing and how to disbelieve
//! the answer.

pub mod download;
pub mod simulator;

use std::convert::Infallible;
use std::fmt;
use std::time::Duration;

use knx_core::commissioning::authorisation::{
    AccessLevel, Authorisation, AuthorisationPlan, LevelCount, FREE_ACCESS_KEY,
};
use knx_core::commissioning::error_code::{read_error_code, ErrorCodeReadError, SystemErrorClass};
use knx_core::commissioning::load_control::{
    event_payload, LoadControlPayload, LOAD_CONTROL_NR_OF_ELEM, LOAD_CONTROL_START_INDEX,
};
use knx_core::commissioning::load_state::{
    permitted_outcomes, LoadEvent, LoadState, MaskVersion, PermittedOutcomes, Stimulus,
    UnknownLoadState,
};
use knx_core::commissioning::memory::{
    chunks, service_for, write_limit, ApduLengthSource, ChunkError, MemoryService, WriteLimit,
};
use knx_core::commissioning::mutation::{TargetKind, WriteAuthorisation, WriteScope};
use knx_core::commissioning::programming_mode::{
    prog_mode_write, ProgModeWrite, CURR_PROG_MODE_ADDRESS,
};
use knx_core::commissioning::properties::{
    verify_mode_active, with_verify_mode, ObjectIndex, PID_DEVICE_CONTROL, PID_DOWNLOAD_COUNTER,
    PID_ERROR_CODE, PID_LOAD_STATE_CONTROL, PID_MANUFACTURER_ID, PID_MAX_APDU_LENGTH,
    PID_MCB_TABLE, PID_TABLE_REFERENCE,
};
use knx_core::{ContactableAddress, ExcludedAddress, GroupValue, IndividualAddress};
use tokio::sync::broadcast;

use crate::cemi::{ApplicationService, CemiError, Destination, LDataMessageKind, Tpci};
use crate::client::{BusError, TunnelEvent};
use crate::management::{
    ManagementTransport, ACKNOWLEDGE_TIMEOUT, CONNECTION_TIMEOUT, MAX_REP_COUNT,
};

/// Total transmissions of a connected request before it is given up on: the
/// original send plus `MAX_REP_COUNT` repetitions.
///
/// `[D]` TL §3, p. 15: *"the local Transport Layer shall repeat the
/// transmission of the T_DATA_CONNECTED_REQ_PDU up to 3 times"* — 3
/// repetitions of the original send, 4 transmissions total. Comparing the
/// running `attempts` count against `MAX_REP_COUNT` directly stops one
/// transmission short of that, because `attempts` already includes the
/// original send; this constant exists so that mistake cannot recur.
///
/// TL §4's clause 4 names `max_rep_count` (`3; maximum of T_Connect.req
/// repetitions`, p. 16) for a different service, `T_Connect`, not
/// `T_DATA_CONNECTED` — read alone it would be the wrong citation here.
/// The stronger ground is TL §5, p. 17, where the same `rep_count`
/// variable is defined as *"used to count the number of
/// T_DATA_CONNECTED_REQ repetitions"* — the service this constant actually
/// governs. The state machine's actions turn that into arithmetic, not
/// just wording: action A7 (p. 20, invoked on the original send, §5.5.3.1
/// p. 33) *"Clear[s] the rep_count"*, and action A9 (p. 20, invoked on
/// each repeat, §5.5.3.5 p. 35) *"Increment[s] the rep_count"*, so
/// `rep_count == max_rep_count` (the give-up clause, §5.5.3.6 p. 35) is
/// reached only after `max_rep_count` repeats past the original, cleared
/// send — 3 repetitions is 4 transmissions by construction.
const MAX_TRANSMISSIONS: u8 = MAX_REP_COUNT + 1;

/// The timings a session runs on, all of them injectable so that a test of
/// the §5.5 wait loop does not take thirty seconds to fail.
///
/// [`SessionTiming::default`] is the cited set. A test that wants the loop's
/// *shape* without its patience builds its own; a production caller has no
/// reason to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionTiming {
    /// How long to wait for the matching positive `L_Data.con` after
    /// `T_Connect` was accepted by the KNXnet/IP gateway.
    ///
    /// `[D]` **TL** clause 4: connection time-out 6 s. A
    /// `TUNNELLING_ACK` alone only confirms gateway acceptance; it does not
    /// prove that the device-side Transport Layer connection progressed.
    pub connection_timeout: Duration,
    /// How long to wait for one answer.
    ///
    /// `[D]` **TL** clause 4 via RESEARCH §8.5 and spec §11.1: the
    /// acknowledge time-out is 3 s.
    pub response_timeout: Duration,
    /// How often the wait loop reads `PID_LOAD_STATE_CONTROL`.
    ///
    /// `[D]` RES §4.23.2.4.1: *"The period for reading shall not exceed half
    /// the TL-timeout, i.e. 3 seconds."*
    pub poll_interval: Duration,
    /// How long a state transition may take before the client gives up.
    ///
    /// `[D]` RES §4.23.2.1: *"The transitions between the states shall be
    /// less than 30 seconds."* — a floor on the client's patience, per
    /// spec §5.5, not a deadline to enforce against the device.
    pub max_transition: Duration,
    /// How long to wait after a memory write before reading it back, when
    /// Verify Mode is unavailable.
    ///
    /// MP §3.16 names this — *"delay for programming the memory in the
    /// device"* — and quantifies it nowhere, and neither knowledge base
    /// gives a figure (spec §6.3, §12). **This default is invented by this
    /// project and is not a specification value.** It is deliberately
    /// generous: reading back too early reports a write as failed that
    /// merely had not landed yet.
    pub programming_delay: Duration,
    /// `t1`: the earliest point at which the MaC may expect a Basic
    /// Restart to have already succeeded, and may start connecting to the
    /// device again.
    ///
    /// MP §3.7.1.1.2, p. 79, Figure 19. This session does not reconnect on
    /// its own after [`ManagementSession::restart_basic`] returns — MP
    /// §2.3 step 4's *"connect to the new address"* is a fresh session's
    /// job — so this field exists to be read by whichever caller does
    /// that reconnecting, not to be waited on inside this crate today.
    pub restart_basic_t1: Duration,
    /// `t2`: how long before the MaS is expected to be responsive again
    /// with its post-restart conditions.
    ///
    /// MP §3.7.1.1.2, p. 79, Figure 19, *"unless … responded differently
    /// by the MaS in the A_Restart_Response-PDU if it is larger than
    /// 5 s"*. That escape hatch is MP §3.7.1.2.2, pp. 80-81's Master Reset
    /// Process Time, and this is also the floor under it —
    /// [`MasterResetResponse::recovery_wait`] never returns less than
    /// this, because the Standard's own words are *"a minimal time for
    /// the MaC to wait, not a maximal time."*
    pub restart_responsive_again: Duration,
    /// MP §3.7.3 exception (5), p. 90: after issuing an `A_Restart`, this
    /// session sends its own `T_Disconnect` regardless of what the device
    /// does, then waits this long for a possible device-initiated
    /// `T_Disconnect` before letting configuration continue — *"the
    /// configuration shall not be continued while this time-out has not
    /// elapsed"*, whether or not the device's disconnect ever arrives.
    ///
    /// [`ManagementSession::disconnect`] stays best-effort and unwaited
    /// everywhere else; only the post-restart path
    /// ([`ManagementSession::restart_basic`],
    /// [`ManagementSession::restart_master_reset`]) uses this field.
    pub post_restart_disconnect_wait: Duration,
}

impl Default for SessionTiming {
    fn default() -> Self {
        Self {
            connection_timeout: CONNECTION_TIMEOUT,
            response_timeout: ACKNOWLEDGE_TIMEOUT,
            poll_interval: Duration::from_secs(3),
            max_transition: Duration::from_secs(30),
            programming_delay: Duration::from_millis(500),
            restart_basic_t1: Duration::from_secs(1),
            restart_responsive_again: Duration::from_secs(5),
            post_restart_disconnect_wait: Duration::from_secs(6),
        }
    }
}

/// What happened when the session tried to turn Verify Mode on.
///
/// There is no `Off` variant that means "we did not try": the session either
/// asserted it or has not connected yet, and the difference between "the
/// device took it" and "the device is conformantly ignoring it" is the whole
/// point of spec §6.3's third case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerifyMode {
    /// Bit 2 read back set: writes are confirmed and the device returns what
    /// it read back (spec §6.2).
    Active,
    /// Bit 2 read back clear after being written.
    ///
    /// `[D, corpus]` PROF footnote 8/16: *"If Verify Mode is not
    /// implemented, it shall always be off."* Conformant, so not an error —
    /// but it switches every memory write to the explicit read-back path of
    /// MP §3.16's `DMP_MemWrite_RCo`.
    Unavailable,
}

impl fmt::Display for VerifyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            VerifyMode::Active => "Verify Mode active",
            VerifyMode::Unavailable => "Verify Mode unavailable (device kept bit 2 clear)",
        })
    }
}

/// Table 4's Erase Code `01h`: *"Confirmed Restart. No Resource value
/// shall be reset. This encoding shall allow using the Master Reset as a
/// confirmed alternative to the unconfirmed Basic Restart"* (MP
/// §3.7.1.2.3.1, p. 82). Not this module's only legal Erase Code — Table 4
/// lists several more, none of which this session polices, per cemi.rs's
/// own refusal to guess which octets belong to which restart type — but
/// the one [`ManagementSession::restart_master_reset`]'s doc example
/// names.
pub const ERASE_CODE_CONFIRMED_RESTART: u8 = 0x01;

/// The device's answer to a Master Reset request.
///
/// AL §3.3.7 Figure 41, decoded from an `A_Restart_Response-PDU`'s data
/// octets: the Error Code, then Process Time as a big-endian
/// `DPT_TimePeriodSec` (DPT 7.005, MP §3.7.1.2.2, pp. 80-81). A Basic
/// Restart has no such response — spec §8.1's confirmation there is the
/// Transport Layer acknowledge alone — so this type exists only for the
/// Master Reset half of `A_Restart`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MasterResetResponse {
    /// `00h` on success; MP §3.7.3's exception handling names the other
    /// values this session does not interpret further (Access Denied,
    /// Unsupported Erase Code, Invalid Channel Number).
    pub error_code: u8,
    /// The worst-case time the device reports it needs, decoded as
    /// written rather than validated against Table 4 — a device is free
    /// to report anything, and disbelieving it is this method's caller's
    /// job, not the decoder's.
    pub process_time: Duration,
}

impl MasterResetResponse {
    /// How long to wait before MP §3.7.1.2.2, p. 81's *"call the failed
    /// [Application Layer service] one last time"* — the greater of what
    /// the device reported and `timing`'s own standard process time,
    /// because *"the process time is thus a minimal time for the MaC to
    /// wait, not a maximal time"*. A negative confirmation
    /// (`error_code != 0`) is reported here with the same arithmetic: the
    /// device is required to answer `0000h` in that case (p. 81), so the
    /// floor alone decides it.
    pub fn recovery_wait(&self, timing: &SessionTiming) -> Duration {
        self.process_time.max(timing.restart_responsive_again)
    }
}

/// Decodes an `A_Restart_Response-PDU`'s data octets (AL §3.3.7 Figure 41):
/// one Error Code octet, then a big-endian `DPT_TimePeriodSec` Process
/// Time.
fn decode_master_reset_response(data: &[u8]) -> Result<MasterResetResponse, SessionError> {
    match *data {
        [error_code, hi, lo] => Ok(MasterResetResponse {
            error_code,
            process_time: Duration::from_secs(u64::from(u16::from_be_bytes([hi, lo]))),
        }),
        _ => Err(SessionError::MalformedRestartResponse { got: data.len() }),
    }
}

/// Why a session operation did not complete.
///
/// Long on purpose: spec §9.2's table exists because several failures look
/// identical on the wire, and the only way not to hide that is to name each
/// observation as what was observed rather than as a guessed cause.
///
/// Not `PartialEq`, deliberately: [`BusError`] carries a
/// [`std::io::Error`], and a comparison that quietly ignored the cause of an
/// I/O failure would be worse than no comparison. Tests match on shape.
#[derive(Debug)]
pub enum SessionError {
    /// The transport failed.
    Transport(BusError),
    /// A frame could not be encoded, which is a programming error in this
    /// crate and never a device's fault.
    Encode(CemiError),
    /// The target is on the project exclusion list (spec §2.1).
    Excluded(ExcludedAddress),
    /// The target is this connection's own address.
    SelfAddressed(IndividualAddress),
    /// A write was attempted on a session that holds no
    /// [`WriteAuthorisation`].
    NoAuthorisation {
        /// What the write would have been.
        scope: WriteScope,
    },
    /// A write was attempted against something that is not the simulator.
    ///
    /// Spec §15 and §13 R11: phase 2 writes to the simulator and to nothing
    /// else. Refused at the session, not at the call site, so that no
    /// procedure can be the one that forgot.
    NotASimulator {
        /// The device the write would have reached.
        target: IndividualAddress,
        /// What the transport says it is.
        transport: TargetKind,
        /// What the authorisation says it is.
        authorised: TargetKind,
    },
    /// The authorisation does not cover this write.
    Refused(knx_core::commissioning::mutation::AuthorisationRefused),
    /// Nothing was sent because the session is not connected.
    NotConnected,
    /// The device or the gateway tore the connection down.
    ConnectionLost {
        /// What was in flight.
        during: &'static str,
    },
    /// The matching `L_Data.con` reported that `T_Connect` failed on the
    /// bus. Unlike silence, this is an explicit negative confirmation.
    ConnectRejected {
        /// Device whose Transport Layer connection was rejected.
        target: IndividualAddress,
    },
    /// Nothing answered within the operation's applicable timeout, and the
    /// Transport Layer connection — if there was one — is still open.
    ///
    /// Reached two ways. A connected request that was acknowledged at the
    /// Transport Layer but never answered at the application layer: the
    /// T_ACK stopped the acknowledge timer, so TL's give-up clause never
    /// fires and the connection stays in `OPEN_IDLE`. And `T_Connect`
    /// itself, which waits [`SessionTiming::connection_timeout`] once and
    /// has no connection to lose.
    ///
    /// For a memory write this is spec §9.2's first row and stays
    /// ambiguous: a lost frame and a protected region are the same silence.
    ///
    /// The unacknowledged case is [`SessionError::ConnectionReleased`]
    /// instead, and that distinction is the whole point of the two
    /// variants: a caller must be able to tell a slow device from one that
    /// stopped being connected four transmissions ago.
    NoAnswer {
        /// What was being waited for.
        waiting_for: &'static str,
        /// How long each attempt waited.
        each: Duration,
        /// How many attempts were made.
        attempts: u8,
    },
    /// Every transmission of a connected request went unacknowledged, so the
    /// Transport Layer connection was released and this session no longer
    /// holds one.
    ///
    /// `[D]` TL §3.9, p. 15, the sentence after the one
    /// `MAX_TRANSMISSIONS` cites: *"If it fails, the local Transport Layer
    /// shall pass a T_Disconnect.ind primitive to the local user indicating
    /// that the connection is released (state = CLOSED)."*
    ///
    /// `[D]` The state machine says the same in arithmetic. TL §5.2, p. 19
    /// defines event `E18` as `ACKNOWLEDGE_TIME_OUT_ind (rep_count >=
    /// max_rep_count)`; TL §5.4.1's transition table, p. 22, maps `E18` in
    /// `OPEN_WAIT` to state `CLOSED` with action `A6`; and `A6`, TL §5.3,
    /// p. 19, is *"Send a N_Data_Individual.req with T_DISCONNECT_REQ_PDU
    /// [...] Send a T_Disconnect.ind to the user."* TL §5.5.3.6, p. 35
    /// draws the same `A6` for the `T_NAK_PDU` route to the same condition.
    ///
    /// Distinct from [`SessionError::ConnectionLost`], which is the far end
    /// tearing the connection down, and from [`SessionError::NoAnswer`],
    /// which leaves a live connection behind.
    ConnectionReleased {
        /// What was being waited for.
        waiting_for: &'static str,
        /// How long each transmission waited for its T_ACK.
        each: Duration,
        /// How many transmissions went out. Always `MAX_TRANSMISSIONS`,
        /// and reported rather than implied so that a test can pin it.
        attempts: u8,
    },
    /// The event channel dropped frames, so the answer may have been one of
    /// them. Not reported as a timeout, because that would claim a silence
    /// that was never observed.
    Lagged {
        /// What was being waited for.
        waiting_for: &'static str,
    },
    /// A property read or write answered `nr_of_elem = 0`.
    ///
    /// `[D]` AL §3.4.4.2 *"Error handling"*: *"If the remote application
    /// process has a problem, e.g., Interface Object or Property doesn't
    /// exist or the requester does not have the required access rights,
    /// then the nr_of_elem of the A_PropertyValue_Response-PDU shall be
    /// zero and shall contain no data."* All three causes, one shape.
    PropertyRefused {
        /// Which object was addressed.
        object_index: ObjectIndex,
        /// Which property.
        property_id: u8,
    },
    /// A memory response carried `number = 0`, the failure answer of
    /// AL §3.5.3/§3.5.4.
    MemoryRefused {
        /// The address that was asked about.
        address: u32,
    },
    /// A write was read back and the octets differ.
    ///
    /// `[D]` MP §3.16: *"different or no data received ⇒ error"*.
    ReadBackMismatch {
        /// Where.
        address: u32,
        /// What was sent.
        written: Vec<u8>,
        /// What came back.
        read: Vec<u8>,
    },
    /// A property write was read back and the octets differ.
    PropertyReadBackMismatch {
        /// Which object.
        object_index: ObjectIndex,
        /// Which property.
        property_id: u8,
        /// What was sent.
        written: Vec<u8>,
        /// What came back.
        read: Vec<u8>,
    },
    /// `PID_LOAD_STATE_CONTROL` answered an octet RES Table 92 does not
    /// define.
    UnknownLoadState(UnknownLoadState),
    /// `PID_ERROR_CODE` answered something DPT 20.011 does not define.
    ErrorCode(ErrorCodeReadError),
    /// A property answered with the wrong number of octets for its type.
    MalformedProperty {
        /// Which object.
        object_index: ObjectIndex,
        /// Which property.
        property_id: u8,
        /// How many octets were expected.
        expected: usize,
        /// How many arrived.
        got: usize,
    },
    /// `A_DeviceDescriptor_Response` for descriptor type 0 carried
    /// something other than the two mask-version octets AL §3.4.2.1
    /// Figure 38 gives it.
    MalformedDescriptor {
        /// How many octets followed the APCI.
        got: usize,
    },
    /// The state did not settle within [`SessionTiming::max_transition`].
    ///
    /// Spec §5.5: *"on expiry: the transition failed; do not assume which
    /// side failed"* — so this carries what was last seen and no verdict.
    TransitionTimedOut {
        /// Which object's Load State Machine.
        object_index: ObjectIndex,
        /// The event that was written.
        event: LoadEvent,
        /// The last state read, if any read succeeded at all.
        last_state: Option<LoadState>,
        /// How long the loop waited.
        waited: Duration,
    },
    /// The state after an event write is the state before it, which spec
    /// §9.2 lists four causes for — including one that is not an error.
    StateUnchanged {
        /// Which object's Load State Machine.
        object_index: ObjectIndex,
        /// The event that was written.
        event: LoadEvent,
        /// The state that did not change.
        state: LoadState,
    },
    /// The device reported a state RES Table 94 does not permit for the
    /// event that was written.
    IllegalTransition {
        /// Which object's Load State Machine.
        object_index: ObjectIndex,
        /// The event that was written.
        event: LoadEvent,
        /// Where it started.
        from: LoadState,
        /// What the device reported.
        observed: LoadState,
    },
    /// A region could not be split into writes.
    Chunk(ChunkError),
    /// `PID_TABLE_REFERENCE` read zero.
    ///
    /// `[D]` CP §3.5.2: *"if it is zero then allocation was not
    /// successful"*. Spec §9.2 gives it two meanings — a failed allocation,
    /// or an allocation attempted outside `Loading` and therefore ignored
    /// (§7.6) — and this error names both because the wire cannot tell them
    /// apart.
    AllocationFailed {
        /// Which object was being allocated.
        object_index: ObjectIndex,
    },
    /// An `A_Restart_Response-PDU` for a Master Reset carried something
    /// other than the Error Code octet and the two-octet Process Time
    /// AL §3.3.7 Figure 41 gives it (MP §3.7.1.2.2, pp. 80-81).
    MalformedRestartResponse {
        /// How many octets the response carried.
        got: usize,
    },
}

impl From<BusError> for SessionError {
    fn from(err: BusError) -> Self {
        SessionError::Transport(err)
    }
}

impl From<CemiError> for SessionError {
    fn from(err: CemiError) -> Self {
        SessionError::Encode(err)
    }
}

impl From<ChunkError> for SessionError {
    fn from(err: ChunkError) -> Self {
        SessionError::Chunk(err)
    }
}

impl From<knx_core::commissioning::mutation::AuthorisationRefused> for SessionError {
    fn from(err: knx_core::commissioning::mutation::AuthorisationRefused) -> Self {
        SessionError::Refused(err)
    }
}

impl std::error::Error for SessionError {}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::Transport(err) => write!(f, "transport failed: {err}"),
            SessionError::Encode(err) => write!(f, "could not encode the frame: {err}"),
            SessionError::Excluded(err) => write!(f, "{err}"),
            SessionError::SelfAddressed(addr) => write!(
                f,
                "{addr} is this connection's own address, not a device to manage"
            ),
            SessionError::NoAuthorisation { scope } => write!(
                f,
                "this session holds no authorisation, so it cannot perform a {scope} \
                 operation"
            ),
            SessionError::NotASimulator {
                target,
                transport,
                authorised,
            } => write!(
                f,
                "refusing to write to {target}: phase 2 writes to the simulator only, \
                 and this transport is {transport:?} with an authorisation for \
                 {authorised:?}"
            ),
            SessionError::Refused(err) => write!(f, "{err}"),
            SessionError::NotConnected => {
                write!(f, "no Transport Layer connection is open to the device")
            }
            SessionError::ConnectionLost { during } => {
                write!(f, "the connection was broken down during {during}")
            }
            SessionError::ConnectRejected { target } => write!(
                f,
                "the bus rejected the Transport Layer connection to {target}"
            ),
            SessionError::NoAnswer {
                waiting_for,
                each,
                attempts,
            } => write!(
                f,
                "no answer to {waiting_for} after {attempts} attempt(s) of {each:?}; \
                 this is a lost frame, a protected or absent target, or a device that \
                 is not listening, and the wire does not distinguish them"
            ),
            SessionError::ConnectionReleased {
                waiting_for,
                each,
                attempts,
            } => write!(
                f,
                "no T_ACK for {waiting_for} after {attempts} transmission(s) of \
                 {each:?}, so the Transport Layer connection was released and is \
                 now CLOSED (TL §3.9); this is a lost frame, a protected or absent \
                 target, or a device that is not listening, and the wire does not \
                 distinguish them — but nothing further may be sent on this \
                 connection until it is opened again"
            ),
            SessionError::Lagged { waiting_for } => write!(
                f,
                "frames were dropped while waiting for {waiting_for}, so the answer may \
                 have been among them; this is not evidence of silence"
            ),
            SessionError::PropertyRefused {
                object_index,
                property_id,
            } => write!(
                f,
                "property {property_id} of {object_index} answered with no elements: it \
                 does not exist, or this session's access level is insufficient"
            ),
            SessionError::MemoryRefused { address } => write!(
                f,
                "the device answered number = 0 for memory at {address:#x}: unreachable, \
                 protected, or an illegal octet count"
            ),
            SessionError::ReadBackMismatch {
                address,
                written,
                read,
            } => write!(
                f,
                "memory at {address:#x} reads back {read:02X?} after {written:02X?} was \
                 written"
            ),
            SessionError::PropertyReadBackMismatch {
                object_index,
                property_id,
                written,
                read,
            } => write!(
                f,
                "property {property_id} of {object_index} reads back {read:02X?} after \
                 {written:02X?} was written"
            ),
            SessionError::UnknownLoadState(err) => write!(f, "{err}"),
            SessionError::ErrorCode(err) => write!(f, "{err}"),
            SessionError::MalformedProperty {
                object_index,
                property_id,
                expected,
                got,
            } => write!(
                f,
                "property {property_id} of {object_index} answered {got} octet(s) where \
                 its type has {expected}"
            ),
            SessionError::MalformedDescriptor { got } => write!(
                f,
                "A_DeviceDescriptor_Response for type 0 carried {got} octet(s) where the \
                 mask version has two"
            ),
            SessionError::TransitionTimedOut {
                object_index,
                event,
                last_state,
                waited,
            } => {
                let last = match last_state {
                    Some(state) => format!("last read {state}"),
                    None => "no state was read at all".to_string(),
                };
                write!(
                    f,
                    "{object_index} did not settle within {waited:?} after {event} \
                     ({last}); which side failed is not determined"
                )
            }
            SessionError::StateUnchanged {
                object_index,
                event,
                state,
            } => write!(
                f,
                "{object_index} still reads {state} after {event} was written: the event \
                 may be illegal or unknown, or a genuine no-op, or the access level may \
                 be too low to write a property it is high enough to read, or the device \
                 may have no loadable application at all — in which case the property is \
                 conformantly read-only and this is not an error"
            ),
            SessionError::IllegalTransition {
                object_index,
                event,
                from,
                observed,
            } => write!(
                f,
                "{object_index} went from {from} to {observed} on {event}, which RES \
                 Table 94 does not permit"
            ),
            SessionError::Chunk(err) => write!(f, "{err}"),
            SessionError::AllocationFailed { object_index } => write!(
                f,
                "PID_TABLE_REFERENCE of {object_index} reads 0: the allocation failed, \
                 or it was attempted outside Loading and silently ignored"
            ),
            SessionError::MalformedRestartResponse { got } => write!(
                f,
                "A_Restart_Response carried {got} octets, not the Error Code plus \
                 two-octet Process Time MP §3.7.1.2.2 expects"
            ),
        }
    }
}

/// How strictly the read-back of a property write is to be compared.
///
/// Two values, both of them documented rather than convenient. There is no
/// `None`: a write that is not compared at all does not exist in this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Comparison {
    /// The read-back must equal what was sent, octet for octet.
    /// `[D]` AL §3.4.4.2 plus MP §3.16's *"different or no data received
    /// ⇒ error"*.
    ExactOctets,
    /// `PID_LOAD_STATE_CONTROL`: the property is `PDT_CONTROL`, so its
    /// read-back is the resulting *state* and never the ten-octet event.
    /// Verified by the caller's state check instead (spec §5.1, CP NOTE 9).
    ResultingStateInstead,
    /// `PID_DEVICE_CONTROL` while setting Verify Mode: bit 2 may legally
    /// come back clear, and no other bit may differ.
    VerifyModeIsAllowedToStayOff,
}

/// What a session knows about the connection it is holding open.
///
/// Every field here dies with the Transport Layer connection, which is why
/// they live together and are reset together: `[D]` AL §3.5.7 for the access
/// level, RES §4.2.14.7.3 for Verify Mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionState {
    /// The level the device granted, or "free level, unknown value".
    pub authorisation: Authorisation,
    /// Whether the device took bit 2.
    pub verify_mode: VerifyMode,
    /// How many `A_Authorize_Request` frames this connection sent.
    ///
    /// Counted because spec §14 item 14 asks for exactly one on the default
    /// path, and a default that quietly ran the profile-scoped procedure is
    /// the regression that test exists to catch.
    pub authorise_requests: u32,
}

/// Whether a T_ACK on its own completes an exchange.
///
/// Only the Verify-Mode-inactive memory write says [`AckIsEnough::Yes`]:
/// `[D]` AL §3.5.4 gives it no application-layer answer to wait for, and
/// everything else in these procedures is answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AckIsEnough {
    /// Wait for an answer the matcher accepts; a bare T_ACK is not the end.
    No,
    /// The T_ACK is the answer. Stop there.
    Yes,
}

/// What one exchange came back with.
enum Exchanged<R> {
    /// A frame the matcher accepted.
    Answer(R),
    /// The request was acknowledged at the Transport Layer and the caller had
    /// said that was all it was waiting for.
    Acknowledged {
        /// How many times the request went out, for a report and for the
        /// `max_rep_count` assertions.
        attempts: u8,
    },
}

/// A connection-oriented management session against one device.
///
/// Borrows its transport rather than owning it: a gateway connection is
/// shared with the rest of the application, and a session is a scope on top
/// of it, not a second connection.
pub struct ManagementSession<'t, T: ManagementTransport> {
    transport: &'t T,
    target: ContactableAddress,
    plan: AuthorisationPlan,
    timing: SessionTiming,
    /// `None` for a read-only session. There is no `bool` anywhere that
    /// could be flipped to make one writable.
    authorisation: Option<WriteAuthorisation>,
    /// Off by default, per spec §10.4's scoping ruling.
    two_key_extension: bool,
    /// The mask version, once Device Descriptor Type 0 has been read, so
    /// that transition narrowing is applied on a fact and not a guess.
    mask: Option<MaskVersion>,
    /// How many levels the device has, if a Profile was consulted.
    level_count: LevelCount,
    /// Our own send sequence number, 4 bit, wrapping.
    send_seq: u8,
    /// `None` when no connection is open.
    connection: Option<ConnectionState>,
    /// How many times [`Self::connect`] has re-opened the connection, so a
    /// test can assert that re-authorisation and Verify Mode actually
    /// happened again rather than being assumed.
    reconnects: u32,
}

impl<T: ManagementTransport> fmt::Debug for ManagementSession<'_, T> {
    /// Hand-written because the transport need not be `Debug`, and because
    /// the key must not be printed. [`AccessKey`](knx_core::commissioning::authorisation::AccessKey)
    /// redacts itself, and this prints the plan's shape rather than trusting
    /// that twice.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ManagementSession")
            .field("target", &self.target.address())
            .field(
                "authorisation",
                &self.authorisation.as_ref().map(|auth| auth.scope()),
            )
            .field("keyed", &matches!(self.plan, AuthorisationPlan::WithKey(_)))
            .field("two_key_extension", &self.two_key_extension)
            .field("mask", &self.mask)
            .field("connection", &self.connection)
            .field("reconnects", &self.reconnects)
            .finish()
    }
}

impl<'t, T: ManagementTransport> ManagementSession<'t, T> {
    /// A session that cannot write, whatever it is asked.
    ///
    /// Reads are not gated on [`TargetKind`]: spec §2.2 permits reads inside
    /// `1.1.24`–`1.1.32`, and the exclusion guard has already refused the
    /// one address that may never be contacted.
    pub fn read_only(
        transport: &'t T,
        target: IndividualAddress,
        plan: AuthorisationPlan,
        timing: SessionTiming,
    ) -> Result<Self, SessionError> {
        Self::build(transport, target, plan, timing, None)
    }

    /// A session that may write, against the simulator, with an
    /// authorisation naming this device and this class of write.
    ///
    /// Refuses at construction if either the authorisation or the transport
    /// is hardware. Both are checked again on every write: this one is the
    /// early, legible refusal, not the load-bearing one.
    pub fn authorised(
        transport: &'t T,
        plan: AuthorisationPlan,
        timing: SessionTiming,
        authorisation: WriteAuthorisation,
    ) -> Result<Self, SessionError> {
        let target = authorisation.target().address();
        let session = Self::build(transport, target, plan, timing, Some(authorisation))?;
        session.check_write_target()?;
        Ok(session)
    }

    fn build(
        transport: &'t T,
        target: IndividualAddress,
        plan: AuthorisationPlan,
        timing: SessionTiming,
        authorisation: Option<WriteAuthorisation>,
    ) -> Result<Self, SessionError> {
        let target = ContactableAddress::new(target).map_err(SessionError::Excluded)?;
        if target.address() == transport.assigned_address() {
            return Err(SessionError::SelfAddressed(target.address()));
        }
        Ok(Self {
            transport,
            target,
            plan,
            timing,
            authorisation,
            two_key_extension: false,
            mask: None,
            level_count: LevelCount::Unknown,
            send_seq: 0,
            connection: None,
            reconnects: 0,
        })
    }

    /// Enables MP §3.5.2's two-key comparison for this session.
    ///
    /// Spec §10.4: the procedure is scoped by its own clause to
    /// *"Profiles — System 2, BIM M112"*, and this document's download is
    /// System B, so it is an opt-in extension and never the default.
    pub fn with_two_key_extension(mut self) -> Self {
        self.two_key_extension = true;
        self
    }

    /// Records the device's mask version, from Device Descriptor Type 0.
    pub fn with_mask(mut self, mask: MaskVersion) -> Self {
        self.mask = Some(mask);
        self
    }

    /// Records the mask version a procedure's step 02 has just read.
    ///
    /// The builder form is for a caller who already knows the mask; this one
    /// is for the sequencer, which learns it from the device and must then
    /// narrow RES Table 94 with it for the rest of the session (spec §5.4).
    pub fn adopt_mask(&mut self, mask: MaskVersion) {
        self.mask = Some(mask);
    }

    /// The mask version this session narrows RES Table 94 with, if it knows
    /// one.
    pub fn mask(&self) -> Option<MaskVersion> {
        self.mask
    }

    /// Records how many access levels the device's Profile gives it.
    pub fn with_level_count(mut self, level_count: LevelCount) -> Self {
        self.level_count = level_count;
        self
    }

    /// The device this session talks to.
    pub fn target(&self) -> IndividualAddress {
        self.target.address()
    }

    /// What is true of the open connection, or `None` if none is open.
    pub fn connection(&self) -> Option<ConnectionState> {
        self.connection
    }

    /// How many times the connection has been re-opened.
    pub fn reconnects(&self) -> u32 {
        self.reconnects
    }

    /// Whether this session could write at all, ignoring what it is asked to
    /// write.
    pub fn may_write(&self) -> bool {
        self.authorisation.is_some() && self.check_write_target().is_ok()
    }

    fn check_write_target(&self) -> Result<(), SessionError> {
        let Some(auth) = &self.authorisation else {
            // Not this function's refusal to make: a read-only session has
            // nothing to check a target against.
            return Ok(());
        };
        let transport_kind = self.transport.target_kind();
        if transport_kind != TargetKind::Simulator || auth.kind() != TargetKind::Simulator {
            return Err(SessionError::NotASimulator {
                target: self.target.address(),
                transport: transport_kind,
                authorised: auth.kind(),
            });
        }
        Ok(())
    }

    /// The gate every write passes: an authorisation exists, it names this
    /// device and this scope, and neither side of the wire is hardware.
    fn authorise_write(&self, scope: WriteScope) -> Result<(), SessionError> {
        let Some(auth) = &self.authorisation else {
            return Err(SessionError::NoAuthorisation { scope });
        };
        self.check_write_target()?;
        auth.authorise(self.target.address(), scope)?;
        Ok(())
    }

    /// The scope this session's authorisation covers, for the writes that
    /// are a precondition of it rather than the point of it — setting
    /// Verify Mode being the only one.
    fn session_scope(&self) -> Option<WriteScope> {
        self.authorisation.as_ref().map(|auth| auth.scope())
    }

    // ---------------------------------------------------------------- wire

    fn next_seq(&mut self) -> u8 {
        let seq = self.send_seq & 0x0F;
        self.send_seq = (self.send_seq + 1) & 0x0F;
        seq
    }

    async fn send(&self, transport: Tpci, service: ApplicationService) -> Result<(), SessionError> {
        self.transport
            .send_frame(
                Destination::Individual(self.target.address()),
                transport,
                service,
            )
            .await
            .map_err(SessionError::Transport)
    }

    /// One connection-oriented request, with the acknowledge time-out and
    /// `max_rep_count` of **TL** clause 4, and the answer `matcher` picks.
    ///
    /// Subscribes before sending, always: the answer to a management request
    /// can arrive before the send call has returned. `matcher` returning
    /// `None` for a frame means "not this answer" and the loop keeps
    /// waiting — a frame is never discarded on the strength of being
    /// unexpected.
    async fn exchange<R>(
        &mut self,
        service: ApplicationService,
        waiting_for: &'static str,
        matcher: impl FnMut(&ApplicationService) -> Option<R>,
    ) -> Result<R, SessionError> {
        match self
            .exchange_inner(service, waiting_for, matcher, AckIsEnough::No)
            .await?
        {
            Exchanged::Answer(answer) => Ok(answer),
            // Only `AckIsEnough::Yes` asks for this variant, and this call
            // site does not; a T_ACK with no answer behind it is the same
            // time-out here as it has always been.
            Exchanged::Acknowledged { attempts } => Err(SessionError::NoAnswer {
                waiting_for,
                each: self.timing.response_timeout,
                attempts,
            }),
        }
    }

    /// A request whose only acknowledgement is the T_ACK, sent under exactly
    /// the same TL clause 4 rules as [`Self::exchange`].
    ///
    /// `[D]` AL §3.5.4: with Verify Mode inactive a memory write gets no
    /// application-layer answer, so the Transport Layer acknowledge is the
    /// whole of the confirmation — which is why it has to be waited for.
    /// Sending and walking away, as this used to do, meant TL's acknowledge
    /// time-out and `max_rep_count = 3` never applied to the one path that
    /// carries a download's data.
    async fn send_acknowledged(
        &mut self,
        service: ApplicationService,
        waiting_for: &'static str,
    ) -> Result<(), SessionError> {
        match self
            .exchange_inner(
                service,
                waiting_for,
                |_| None::<Infallible>,
                AckIsEnough::Yes,
            )
            .await?
        {
            Exchanged::Acknowledged { .. } => Ok(()),
            // The matcher never returns `Some`, and `Infallible` has no
            // value to have been returned: the type system says this arm is
            // uninhabited, so nothing needs to be invented for it.
            Exchanged::Answer(answer) => match answer {},
        }
    }

    async fn exchange_inner<R>(
        &mut self,
        service: ApplicationService,
        waiting_for: &'static str,
        mut matcher: impl FnMut(&ApplicationService) -> Option<R>,
        ack_is_enough: AckIsEnough,
    ) -> Result<Exchanged<R>, SessionError> {
        if self.connection.is_none() {
            return Err(SessionError::NotConnected);
        }
        let seq = self.next_seq();
        let mut attempts = 0u8;
        let mut acknowledged = false;
        let mut events = self.transport.subscribe();
        self.send(Tpci::NumberedData { seq }, service.clone())
            .await?;
        attempts += 1;

        loop {
            let deadline = tokio::time::Instant::now() + self.timing.response_timeout;
            loop {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
                    break;
                }
                match tokio::time::timeout(remaining, events.recv()).await {
                    Ok(Ok(TunnelEvent::Telegram(frame))) => {
                        if frame.kind != LDataMessageKind::Indication
                            || frame.source != self.target.address()
                        {
                            continue;
                        }
                        match frame.transport {
                            Tpci::Ack { seq: acked } if acked == seq => {
                                acknowledged = true;
                                if ack_is_enough == AckIsEnough::Yes {
                                    return Ok(Exchanged::Acknowledged { attempts });
                                }
                                continue;
                            }
                            Tpci::Disconnect => {
                                self.connection = None;
                                return Err(SessionError::ConnectionLost {
                                    during: waiting_for,
                                });
                            }
                            Tpci::NumberedData { seq: received } => {
                                // The receiver's obligation (TL §2): every
                                // T_Data_Connected gets a T_ACK. A session
                                // stays open for the whole download, so
                                // skipping this — as a one-shot probe can
                                // get away with — would have the device
                                // repeat every answer three times.
                                let _ = self
                                    .send(
                                        Tpci::Ack { seq: received },
                                        ApplicationService::NoApplicationPdu,
                                    )
                                    .await;
                                if let Some(answer) = matcher(&frame.service) {
                                    return Ok(Exchanged::Answer(answer));
                                }
                                continue;
                            }
                            _ => continue,
                        }
                    }
                    Ok(Ok(TunnelEvent::Closed)) => {
                        self.connection = None;
                        return Err(SessionError::ConnectionLost {
                            during: waiting_for,
                        });
                    }
                    Ok(Err(broadcast::error::RecvError::Lagged(_))) => {
                        return Err(SessionError::Lagged { waiting_for });
                    }
                    Ok(Err(broadcast::error::RecvError::Closed)) => {
                        self.connection = None;
                        return Err(SessionError::ConnectionLost {
                            during: waiting_for,
                        });
                    }
                    Err(_) => break,
                }
            }

            // `[D]` TL clause 4: repeat up to `max_rep_count` times, and
            // only while the frame has not been acknowledged — a repeat of
            // an acknowledged request would be a second request.
            //
            // The two ways out of this loop are not the same event, and
            // conflating them was C19's defect. A T_ACK arrived: action A8
            // (TL §5.3, p. 20) stopped the acknowledge timer, so
            // `ACKNOWLEDGE_TIME_OUT_ind` never fires, the state machine
            // sits in `OPEN_IDLE`, and the connection survives a silence
            // that belongs to the application layer.
            if acknowledged {
                return Err(SessionError::NoAnswer {
                    waiting_for,
                    each: self.timing.response_timeout,
                    attempts,
                });
            }
            if attempts >= MAX_TRANSMISSIONS {
                // No T_ACK for any of them: this is TL §5.2's `E18`,
                // `ACKNOWLEDGE_TIME_OUT_ind (rep_count >= max_rep_count)`
                // (p. 19), whose cell in the §5.4.1 transition table
                // (p. 22) is `OPEN_WAIT → CLOSED` under action `A6` —
                // *"Send a N_Data_Individual.req with
                // T_DISCONNECT_REQ_PDU [...] Send a T_Disconnect.ind to the
                // user"* (§5.3, p. 19). TL §3.9's prose (p. 15) is the same
                // instruction in words: *"the connection is released (state
                // = CLOSED)"*.
                //
                // [`Self::disconnect`] *is* A6's two halves — the
                // `T_DISCONNECT_REQ_PDU` and the local state change — and
                // is best-effort by the same clause that makes A6's send
                // unconfirmed.
                self.disconnect().await;
                return Err(SessionError::ConnectionReleased {
                    waiting_for,
                    each: self.timing.response_timeout,
                    attempts,
                });
            }
            self.send(Tpci::NumberedData { seq }, service.clone())
                .await?;
            attempts += 1;
        }
    }

    // ------------------------------------------------------------- session

    /// Opens the connection and does the two things every connection needs.
    ///
    /// Spec §10.3: *"the session object owns the key, and `connect()`
    /// performs authorise-then-set-Verify-Mode as one unit"*. Called again
    /// after a drop, and doing both again is the point — §14 items 5 and 15.
    pub async fn connect(&mut self) -> Result<(), SessionError> {
        let mut events = self.transport.subscribe();
        self.send(Tpci::Connect, ApplicationService::NoApplicationPdu)
            .await?;
        let deadline = tokio::time::Instant::now() + self.timing.connection_timeout;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(SessionError::NoAnswer {
                    waiting_for: "to positive L_Data.con for T_Connect",
                    each: self.timing.connection_timeout,
                    attempts: 1,
                });
            }
            match tokio::time::timeout(remaining, events.recv()).await {
                Ok(Ok(TunnelEvent::Telegram(frame)))
                    if frame.source == self.transport.assigned_address()
                        && frame.destination == Destination::Individual(self.target.address())
                        && frame.transport == Tpci::Connect
                        && frame.service == ApplicationService::NoApplicationPdu =>
                {
                    match frame.kind {
                        LDataMessageKind::Confirmation { error: false } => break,
                        LDataMessageKind::Confirmation { error: true } => {
                            return Err(SessionError::ConnectRejected {
                                target: self.target.address(),
                            });
                        }
                        LDataMessageKind::Request | LDataMessageKind::Indication => continue,
                    }
                }
                Ok(Ok(TunnelEvent::Telegram(_))) => continue,
                Ok(Ok(TunnelEvent::Closed)) => {
                    return Err(SessionError::ConnectionLost {
                        during: "T_Connect confirmation",
                    });
                }
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => {
                    return Err(SessionError::Lagged {
                        waiting_for: "positive L_Data.con for T_Connect",
                    });
                }
                Ok(Err(broadcast::error::RecvError::Closed)) => {
                    return Err(SessionError::ConnectionLost {
                        during: "T_Connect confirmation",
                    });
                }
                Err(_) => {
                    return Err(SessionError::NoAnswer {
                        waiting_for: "to positive L_Data.con for T_Connect",
                        each: self.timing.connection_timeout,
                        attempts: 1,
                    });
                }
            }
        }
        self.send_seq = 0;
        self.connection = Some(ConnectionState {
            authorisation: self.plan.initial_authorisation(),
            // Assumed clear, per RES §4.2.14.7.4: *"When opening a Transport
            // Layer connection to the Management Server, the Management
            // Client shall assume the Verify Mode Control to be cleared"*.
            verify_mode: VerifyMode::Unavailable,
            authorise_requests: 0,
        });
        self.authorise().await?;
        if self.authorisation.is_some() {
            self.assert_verify_mode().await?;
        }
        Ok(())
    }

    /// Re-opens a connection that dropped, counting the reconnect.
    async fn reconnect(&mut self) -> Result<(), SessionError> {
        self.connection = None;
        self.reconnects += 1;
        self.connect().await
    }

    /// Closes the connection.
    ///
    /// Best-effort and never an error: TL §3.8 expects no confirmation, and
    /// a device that has already disconnected has nothing left to tear down.
    pub async fn disconnect(&mut self) {
        let _ = self
            .send(Tpci::Disconnect, ApplicationService::NoApplicationPdu)
            .await;
        self.connection = None;
    }

    /// MP §3.5.1 `DMP_Authorize_RCo`, plus §10.4's extension when it was
    /// opted into.
    async fn authorise(&mut self) -> Result<(), SessionError> {
        let AuthorisationPlan::WithKey(key) = self.plan else {
            // `[D]` MP §3.5.1's own guard, `key != FFFFFFFFh`: with no key
            // the procedure is skipped entirely, and the device grants
            // whatever level FFFFFFFFh protects. Recorded as "free level,
            // unknown value" and not as "authorised" (§10.9 item 2).
            return Ok(());
        };
        if self.two_key_extension {
            return self.authorise_two_key(key.octets()).await;
        }
        let level = self.authorize_request(key.octets()).await?;
        self.record_level(level);
        Ok(())
    }

    /// MP §3.5.2 `DM_Authorize2_RCo`, transcribed from spec §10.4.
    ///
    /// The comparison is on access, not on the octet: `client_level >
    /// free_level` in the clause is numeric and therefore means *worse*,
    /// which is why this asks [`AccessLevel::is_more_powerful_than`] instead
    /// of comparing octets and hoping.
    async fn authorise_two_key(&mut self, key: [u8; 4]) -> Result<(), SessionError> {
        let free_level = self
            .authorize_request(FREE_ACCESS_KEY.to_be_bytes())
            .await?;
        self.record_level(free_level);
        // No short-cut for `free_level == MAXIMUM`, tempting though it is:
        // the clause performs both exchanges unconditionally, and an
        // optimisation that changes how many frames a documented procedure
        // puts on the wire is a deviation, not an optimisation.
        let client_level = self.authorize_request(key).await?;
        self.record_level(client_level);
        if free_level.is_more_powerful_than(client_level) {
            let reselected = self
                .authorize_request(FREE_ACCESS_KEY.to_be_bytes())
                .await?;
            self.record_level(reselected);
        }
        Ok(())
    }

    async fn authorize_request(&mut self, key: [u8; 4]) -> Result<AccessLevel, SessionError> {
        let level = self
            .exchange(
                ApplicationService::AuthorizeRequest { key },
                "A_Authorize_Response",
                |service| match service {
                    ApplicationService::AuthorizeResponse { level } => {
                        Some(AccessLevel::from_octet(*level))
                    }
                    _ => None,
                },
            )
            .await?;
        if let Some(state) = &mut self.connection {
            state.authorise_requests += 1;
        }
        Ok(level)
    }

    fn record_level(&mut self, level: AccessLevel) {
        if let Some(state) = &mut self.connection {
            state.authorisation = Authorisation::Granted { level };
        }
    }

    /// Whether the granted level looks like the minimum a wrong key earns.
    ///
    /// Spec §10.2: a wrong key is *worse than not trying*. This is a
    /// suspicion and not a diagnosis, because a correct key may map to the
    /// minimum level and the Standard gives no way to tell.
    pub fn authorisation_is_suspicious(&self) -> bool {
        self.connection
            .map(|state| state.authorisation.is_suspicious_for(self.level_count))
            .unwrap_or(false)
    }

    /// Sets `PID_DEVICE_CONTROL` bit 2 and reads back what the device did
    /// with it.
    ///
    /// Failing to *set* Verify Mode is not an error (spec §6.3, PROF
    /// footnote 8: *"If Verify Mode is not implemented, it shall always be
    /// off."*); failing to notice that it is off is, which is why the result
    /// is stored and every memory write branches on it.
    async fn assert_verify_mode(&mut self) -> Result<(), SessionError> {
        let scope = match self.session_scope() {
            Some(scope) => scope,
            None => return Ok(()),
        };
        self.authorise_write(scope)?;
        let before = self.read_device_control().await?;
        let wanted = with_verify_mode(before, true);
        // Read-modify-write: a blunt `04h` would clear three other bits of
        // RES Table 11 to change one.
        //
        // `Comparison::VerifyModeIsAllowedToStayOff` and not the usual
        // read-back check: a device that does not implement Verify Mode
        // answers with bit 2 clear and is *conforming* while doing it (PROF
        // footnote 8). Treating that as a failed write would refuse to talk
        // to a legal device. Every other bit must still come back as sent,
        // because nothing in the Standard permits those to drift.
        let echoed = self
            .property_write_octets(
                ObjectIndex::DEVICE,
                PID_DEVICE_CONTROL,
                1,
                1,
                vec![wanted],
                scope,
                Comparison::VerifyModeIsAllowedToStayOff,
            )
            .await?;
        let observed = match echoed.first() {
            Some(octet) => *octet,
            None => self.read_device_control().await?,
        };
        if observed != wanted && observed != with_verify_mode(wanted, false) {
            return Err(SessionError::PropertyReadBackMismatch {
                object_index: ObjectIndex::DEVICE,
                property_id: PID_DEVICE_CONTROL,
                written: vec![wanted],
                read: vec![observed],
            });
        }
        if let Some(state) = &mut self.connection {
            state.verify_mode = if verify_mode_active(observed) {
                VerifyMode::Active
            } else {
                VerifyMode::Unavailable
            };
        }
        Ok(())
    }

    async fn read_device_control(&mut self) -> Result<u8, SessionError> {
        let octets = self
            .read_property(ObjectIndex::DEVICE, PID_DEVICE_CONTROL, 1, 1)
            .await?;
        match octets.first() {
            Some(octet) => Ok(*octet),
            None => Err(SessionError::MalformedProperty {
                object_index: ObjectIndex::DEVICE,
                property_id: PID_DEVICE_CONTROL,
                expected: 1,
                got: 0,
            }),
        }
    }

    /// What the session currently believes about Verify Mode.
    pub fn verify_mode(&self) -> Option<VerifyMode> {
        self.connection.map(|state| state.verify_mode)
    }

    // --------------------------------------------------------------- reads

    /// `A_PropertyValue_Read`, with AL §3.4.4.2's `nr_of_elem = 0` refusal
    /// surfaced as an error rather than as empty data.
    pub async fn read_property(
        &mut self,
        object_index: ObjectIndex,
        property_id: u8,
        nr_of_elem: u8,
        start_index: u16,
    ) -> Result<Vec<u8>, SessionError> {
        let answer = self
            .exchange(
                ApplicationService::PropertyValueRead {
                    object_index: object_index.octet(),
                    property_id,
                    nr_of_elem,
                    start_index,
                },
                "A_PropertyValue_Response",
                |service| match service {
                    ApplicationService::PropertyValueResponse {
                        object_index: oi,
                        property_id: pid,
                        nr_of_elem,
                        data,
                        ..
                    } if *oi == object_index.octet() && *pid == property_id => {
                        Some((*nr_of_elem, data.clone()))
                    }
                    _ => None,
                },
            )
            .await?;
        let (elements, data) = answer;
        if elements == 0 {
            return Err(SessionError::PropertyRefused {
                object_index,
                property_id,
            });
        }
        Ok(data)
    }

    /// `PID_LOAD_STATE_CONTROL` read as a state (spec §5.1: read and write
    /// use different encodings of the same property).
    pub async fn read_load_state(
        &mut self,
        object_index: ObjectIndex,
    ) -> Result<LoadState, SessionError> {
        let octets = self
            .read_property(object_index, PID_LOAD_STATE_CONTROL, 1, 1)
            .await?;
        let [octet] = octets[..] else {
            return Err(SessionError::MalformedProperty {
                object_index,
                property_id: PID_LOAD_STATE_CONTROL,
                expected: 1,
                got: octets.len(),
            });
        };
        LoadState::from_octet(octet).map_err(SessionError::UnknownLoadState)
    }

    /// `PID_ERROR_CODE`, decoded through the shared DPT 20.011 codec.
    ///
    /// Read this *before* unloading anything: `[D]` RES §4.2.28 clears it
    /// when the state leaves `Error`, so an unload destroys the only
    /// evidence of what went wrong (spec §5.5, §9.1 step 3).
    pub async fn read_error_code(
        &mut self,
        object_index: ObjectIndex,
    ) -> Result<SystemErrorClass, SessionError> {
        let octets = self
            .read_property(object_index, PID_ERROR_CODE, 1, 1)
            .await?;
        read_error_code(&GroupValue::Bytes(octets)).map_err(SessionError::ErrorCode)
    }

    /// `PID_MANUFACTURER_ID`, the guard of spec §7.1 step 04.
    pub async fn read_manufacturer_id(&mut self) -> Result<u16, SessionError> {
        let octets = self
            .read_property(ObjectIndex::DEVICE, PID_MANUFACTURER_ID, 1, 1)
            .await?;
        match octets[..] {
            [high, low] => Ok(u16::from_be_bytes([high, low])),
            _ => Err(SessionError::MalformedProperty {
                object_index: ObjectIndex::DEVICE,
                property_id: PID_MANUFACTURER_ID,
                expected: 2,
                got: octets.len(),
            }),
        }
    }

    /// `PID_DOWNLOAD_COUNTER` of the Device Object — `[D]` CP §3.12.4/3.12.5,
    /// pp. 99-100, and RES §4.2.30.3, p. 42, both read it there rather than
    /// from the part's own object, so this always addresses
    /// [`ObjectIndex::DEVICE`] and never the caller's target part.
    ///
    /// `SessionError::PropertyRefused` here is not a malformed read: RES
    /// §4.2.30.1, p. 41, and Volume 6 Profiles Annex A (`[C18]`) both make
    /// the property optional, so a refusal is the device saying it has none
    /// — a fact `download.rs`'s `partial_download` acts on, not an error
    /// this method should paper over.
    pub async fn read_download_counter(&mut self) -> Result<u16, SessionError> {
        let octets = self
            .read_property(ObjectIndex::DEVICE, PID_DOWNLOAD_COUNTER, 1, 1)
            .await?;
        match octets[..] {
            [high, low] => Ok(u16::from_be_bytes([high, low])),
            _ => Err(SessionError::MalformedProperty {
                object_index: ObjectIndex::DEVICE,
                property_id: PID_DOWNLOAD_COUNTER,
                expected: 2,
                got: octets.len(),
            }),
        }
    }

    /// Device Descriptor Type 0, which is step 02 of every procedure in
    /// spec §7 and the only place the mask version comes from.
    ///
    /// The mask decides the allocation subtype (§7.3 design rule 1) and
    /// narrows RES Table 94 (§5.4), so a download that skipped this step
    /// would be guessing at both.
    pub async fn read_mask_version(&mut self) -> Result<MaskVersion, SessionError> {
        let data = self
            .exchange(
                ApplicationService::DeviceDescriptorRead { descriptor_type: 0 },
                "A_DeviceDescriptor_Response",
                |service| match service {
                    ApplicationService::DeviceDescriptorResponse {
                        descriptor_type: 0,
                        data,
                    } => Some(data.clone()),
                    _ => None,
                },
            )
            .await?;
        match data[..] {
            [high, low] => Ok(MaskVersion(u16::from_be_bytes([high, low]))),
            _ => Err(SessionError::MalformedDescriptor { got: data.len() }),
        }
    }

    /// `PID_TABLE_REFERENCE`, the base address spec §7.2 step 3 reads back.
    ///
    /// Zero is refused here rather than returned: it is
    /// `[D]` CP §3.5.2's failure value, and the one thing that must never
    /// happen to it is becoming a write offset.
    pub async fn read_table_reference(
        &mut self,
        object_index: ObjectIndex,
    ) -> Result<u32, SessionError> {
        let octets = self
            .read_property(object_index, PID_TABLE_REFERENCE, 1, 1)
            .await?;
        let base = match octets[..] {
            [a, b, c, d] => u32::from_be_bytes([a, b, c, d]),
            [a, b] => u32::from(u16::from_be_bytes([a, b])),
            _ => {
                return Err(SessionError::MalformedProperty {
                    object_index,
                    property_id: PID_TABLE_REFERENCE,
                    expected: 4,
                    got: octets.len(),
                })
            }
        };
        if base == 0 {
            return Err(SessionError::AllocationFailed { object_index });
        }
        Ok(base)
    }

    /// `PID_MCB_TABLE`, whose CRC spec §7.2 step 7 stores and §7.4 compares.
    ///
    /// Returned as raw octets, not [`knx_core::commissioning::mcb::MemoryControlBlock`]:
    /// the layout is RES §4.2.27, Table 12, p. 39, but a protocol session
    /// reads a property, it does not decide what the caller does with the
    /// bytes. `download.rs` parses them where the CRC comparison happens.
    pub async fn read_memory_control_block(
        &mut self,
        object_index: ObjectIndex,
    ) -> Result<Vec<u8>, SessionError> {
        self.read_property(object_index, PID_MCB_TABLE, 1, 1).await
    }

    /// `PID_MAX_APDU_LENGTH` from the Device Object, as spec §6.4's
    /// discovery rule requires.
    ///
    /// `router_object` is consulted only if the Device Object has no such
    /// property, and a value found there means the opposite of what a
    /// careless read would conclude: `[D]` RES §4.3.7.2.2 — management stays
    /// on `L_Data_Standard` frames.
    pub async fn read_apdu_length(
        &mut self,
        router_object: Option<ObjectIndex>,
    ) -> Result<ApduLengthSource, SessionError> {
        match self.read_apdu_length_of(ObjectIndex::DEVICE).await {
            Ok(Some(value)) => return Ok(ApduLengthSource::DeviceObject(value)),
            Ok(None) => {}
            Err(err) => return Err(err),
        }
        if let Some(router) = router_object {
            if let Some(value) = self.read_apdu_length_of(router).await? {
                return Ok(ApduLengthSource::RouterObjectOnly(value));
            }
        }
        Ok(ApduLengthSource::Absent)
    }

    async fn read_apdu_length_of(
        &mut self,
        object_index: ObjectIndex,
    ) -> Result<Option<u16>, SessionError> {
        match self
            .read_property(object_index, PID_MAX_APDU_LENGTH, 1, 1)
            .await
        {
            Ok(octets) => match octets[..] {
                [high, low] => Ok(Some(u16::from_be_bytes([high, low]))),
                [low] => Ok(Some(u16::from(low))),
                _ => Err(SessionError::MalformedProperty {
                    object_index,
                    property_id: PID_MAX_APDU_LENGTH,
                    expected: 2,
                    got: octets.len(),
                }),
            },
            // An absent property answers `nr_of_elem = 0` (AL §3.4.4.1),
            // which is exactly spec §6.4's "not present in the Device
            // Object" case and not a failure.
            Err(SessionError::PropertyRefused { .. }) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// The negotiated write length for this device, per spec §6.4.
    pub async fn write_limit(
        &mut self,
        router_object: Option<ObjectIndex>,
    ) -> Result<WriteLimit, SessionError> {
        Ok(write_limit(self.read_apdu_length(router_object).await?))
    }

    /// `A_Memory_Read` or `A_UserMemory_Read`, chosen by address, with the
    /// failure answer surfaced as an error.
    ///
    /// `[D]` AL §3.5.3: on a failed read *"the parameter number of the
    /// A_Memory_Response-PDU shall be zero and shall contain no data"* —
    /// which this reports rather than returning an empty vector that a
    /// caller might compare successfully against nothing.
    pub async fn read_memory(&mut self, address: u32, number: u8) -> Result<Vec<u8>, SessionError> {
        // The service is chosen by `base + length`, exactly as CP §3.5.2
        // chooses it for a write. Choosing it from the address alone is the
        // trap `knx_core::commissioning::memory::service_for` documents: a
        // region that `service_for` sent as `A_UserMemory_Write` would then be
        // read back out of a different address space.
        self.read_memory_as(service_for(address, u32::from(number)), address, number)
            .await
    }

    /// The same read with the service named by the caller, for a read-back
    /// that must use the service its own write used.
    async fn read_memory_as(
        &mut self,
        service: MemoryService,
        address: u32,
        number: u8,
    ) -> Result<Vec<u8>, SessionError> {
        let request = match service {
            // `service_for` names this service only below `FFFFh`, and a
            // chunk's service is `service_for`'s answer for the whole region.
            MemoryService::Memory => ApplicationService::MemoryRead {
                number,
                address: address as u16,
            },
            MemoryService::UserMemory => ApplicationService::UserMemoryRead { number, address },
        };
        let data = self
            .exchange(request, "A_Memory_Response", |answer| match answer {
                ApplicationService::MemoryResponse { address: at, data }
                    if u32::from(*at) == address =>
                {
                    Some(data.clone())
                }
                ApplicationService::UserMemoryResponse { address: at, data } if *at == address => {
                    Some(data.clone())
                }
                _ => None,
            })
            .await?;
        if data.is_empty() {
            return Err(SessionError::MemoryRefused { address });
        }
        Ok(data)
    }

    // -------------------------------------------------------------- writes

    /// `A_PropertyValue_Write`, with the device's own read-back compared
    /// against what was sent.
    ///
    /// `[D]` AL §3.4.4.2: *"The remote application process shall respond to
    /// the A_PropertyValue_Write.ind primitive with an
    /// A_PropertyValue_Read.res primitive containing the requested number of
    /// elements of the Property value … The value of the Property of the
    /// associated Interface Object shall be explicitly read back after
    /// writing to it."* So a property write is answered, and the answer is a
    /// read-back — which is why this needs no Verify Mode and no second
    /// read.
    ///
    /// The comparison is skipped for `PID_LOAD_STATE_CONTROL` alone, and
    /// that is not an exception to the project's read-back rule: the
    /// property is `PDT_CONTROL`, so a read of it answers the resulting
    /// *state* and never the ten-octet event that was written (design spec
    /// §5.1). Comparing them would fail every single time. The event's
    /// verification is [`Self::write_load_event`]'s state check instead.
    ///
    /// `scope` is what the **caller** is doing, and it is checked against
    /// the session's authorisation. Reading the scope out of that
    /// authorisation instead — which this used to do — made the check pass
    /// by construction and left `WriteScope` meaning nothing for property
    /// writes.
    pub async fn write_property(
        &mut self,
        object_index: ObjectIndex,
        property_id: u8,
        data: Vec<u8>,
        scope: WriteScope,
    ) -> Result<Vec<u8>, SessionError> {
        let comparison = if property_id == PID_LOAD_STATE_CONTROL {
            Comparison::ResultingStateInstead
        } else {
            Comparison::ExactOctets
        };
        self.property_write_octets(object_index, property_id, 1, 1, data, scope, comparison)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn property_write_octets(
        &mut self,
        object_index: ObjectIndex,
        property_id: u8,
        nr_of_elem: u8,
        start_index: u16,
        data: Vec<u8>,
        scope: WriteScope,
        comparison: Comparison,
    ) -> Result<Vec<u8>, SessionError> {
        self.authorise_write(scope)?;
        let written = data.clone();
        let answer = self
            .exchange(
                ApplicationService::PropertyValueWrite {
                    object_index: object_index.octet(),
                    property_id,
                    nr_of_elem,
                    start_index,
                    data,
                },
                "A_PropertyValue_Response to a write",
                |service| match service {
                    ApplicationService::PropertyValueResponse {
                        object_index: oi,
                        property_id: pid,
                        nr_of_elem,
                        data,
                        ..
                    } if *oi == object_index.octet() && *pid == property_id => {
                        Some((*nr_of_elem, data.clone()))
                    }
                    _ => None,
                },
            )
            .await?;
        let (elements, read) = answer;
        if elements == 0 {
            return Err(SessionError::PropertyRefused {
                object_index,
                property_id,
            });
        }
        if comparison == Comparison::ExactOctets && read != written {
            return Err(SessionError::PropertyReadBackMismatch {
                object_index,
                property_id,
                written,
                read,
            });
        }
        Ok(read)
    }

    /// Writes one load event and checks what the Load State Machine did
    /// with it.
    ///
    /// The event is MP §3.31.3's ten octets at `start_index = 01h`,
    /// `nr_of_elem = 01h`. The check afterwards is the §5.5 wait loop, and
    /// its three failure shapes are kept apart on purpose: an unchanged
    /// state (§9.2 row 3, four causes, one of them not an error), a state
    /// RES Table 94 does not permit, and a transition that never settled.
    pub async fn write_load_event(
        &mut self,
        object_index: ObjectIndex,
        event: LoadEvent,
    ) -> Result<LoadState, SessionError> {
        self.write_load_control(object_index, event, event_payload(event))
            .await
    }

    /// Writes an `Additional Load Controls` payload — an allocation, in
    /// practice — and checks the Load State Machine the same way an event
    /// write is checked.
    ///
    /// The payload is built in `knx-core` from the subtype the device's mask
    /// profiles (spec §7.3), which is why this takes a whole
    /// [`LoadControlPayload`] rather than a subtype: choosing the subtype is
    /// a domain decision with a citation attached, and no part of it belongs
    /// in a transport-layer session.
    ///
    /// `[D]` CP §3.5.1.2 Table 4: an allocation is *"ignored"* outside
    /// `Loading`, with no error, so the state check here is what stands
    /// between a caller and a silently absent allocation.
    pub async fn write_allocation(
        &mut self,
        object_index: ObjectIndex,
        payload: LoadControlPayload,
    ) -> Result<LoadState, SessionError> {
        self.write_load_control(object_index, LoadEvent::AdditionalLoadControls, payload)
            .await
    }

    async fn write_load_control(
        &mut self,
        object_index: ObjectIndex,
        event: LoadEvent,
        payload: LoadControlPayload,
    ) -> Result<LoadState, SessionError> {
        let scope = match event {
            // An `Unload` is its own scope when it is the point of the
            // operation — the standalone procedure of §7.5 — and part of the
            // download when the download is the point: `[D]` CP §3.5.2 step
            // 05 unloads every part *inside* the complete download, so an
            // operator who authorised a download to this device authorised
            // that. No other scope reaches an unload, and a download
            // authorisation still reaches nothing but a download.
            LoadEvent::Unload if self.session_scope() == Some(WriteScope::Download) => {
                WriteScope::Download
            }
            LoadEvent::Unload => WriteScope::Unload,
            _ => WriteScope::Download,
        };
        self.authorise_write(scope)?;
        let before = self.read_load_state(object_index).await?;
        let payload = payload.octets().to_vec();
        self.property_write_octets(
            object_index,
            PID_LOAD_STATE_CONTROL,
            LOAD_CONTROL_NR_OF_ELEM,
            LOAD_CONTROL_START_INDEX,
            payload,
            scope,
            Comparison::ResultingStateInstead,
        )
        .await?;
        let outcomes = permitted_outcomes(before, Stimulus::Event(event), self.mask);
        let observed = match self
            .wait_for_load_state(object_index, event, before, &outcomes)
            .await
        {
            Ok(observed) => observed,
            // A state that never moved at all is not the same observation as
            // a transition that was still running when patience ran out, and
            // spec §9.2 row 3 lists four causes for it — one of which is not
            // an error. Reported as what was seen.
            Err(SessionError::TransitionTimedOut {
                last_state: Some(state),
                ..
            }) if state == before => {
                return Err(SessionError::StateUnchanged {
                    object_index,
                    event,
                    state: before,
                })
            }
            Err(err) => return Err(err),
        };
        if !outcomes.accepts(observed) {
            return Err(SessionError::IllegalTransition {
                object_index,
                event,
                from: before,
                observed,
            });
        }
        Ok(observed)
    }

    /// Spec §5.5's wait loop, implemented once.
    ///
    /// Polls at most every [`SessionTiming::poll_interval`] for at most
    /// [`SessionTiming::max_transition`], re-establishes a dropped
    /// connection and keeps polling — which re-authorises and re-asserts
    /// Verify Mode, because [`Self::connect`] does both — accepts
    /// `LoadCompleting` as still working, and accepts a device that does not
    /// answer *while* in a state RES Table 94 permits silence in.
    ///
    /// "Does not answer" is two different devices, and since C19 they are
    /// handled as two. One acknowledges the read and withholds the answer:
    /// its connection is alive, RES §4.23.2.4.1's *"an established
    /// TL-connection is kept alive"*, and the next poll goes out on it. The
    /// other acknowledges nothing: its connection was released by TL's
    /// action A6 after [`MAX_TRANSMISSIONS`] transmissions, RES's
    /// *"otherwise, the TL-connection will close after TL-timeout"*, and the
    /// next poll must open a new one first. Polling a released connection —
    /// which is what this loop used to do — asks a peer that has forgotten
    /// the connection exists, and what comes back is undefined rather than
    /// merely absent.
    async fn wait_for_load_state(
        &mut self,
        object_index: ObjectIndex,
        event: LoadEvent,
        from: LoadState,
        outcomes: &PermittedOutcomes,
    ) -> Result<LoadState, SessionError> {
        let started = tokio::time::Instant::now();
        let mut last_state: Option<LoadState> = None;
        // C5, RES §4.23.2.4.1: the deadline below buys exactly one more
        // attempt, not a second polling loop — this flag is what makes
        // "once more" mean once.
        let mut made_the_one_more_attempt = false;
        loop {
            let connected = match self.connection {
                Some(_) => true,
                None => match self.reconnect().await {
                    Ok(()) => true,
                    // `[D]` RES §4.23.2.4.1: *"the MaC shall try to
                    // re-establish the connection **periodically** during
                    // the maximum transition time"* — periodically, so one
                    // failed attempt is a reason to wait and try again, not
                    // a reason to stop. RES §4.23.2.4.1's NOTE 86 names the
                    // device this covers: *"A device may be offline during
                    // state LoadCompleting."* An offline device does not
                    // answer T_Connect either, and before C19 that never
                    // came up here, because an exhausted read left a stale
                    // connection in place and no re-establishment was ever
                    // attempted. Bounded by the deadline below, which turns
                    // a device that never comes back into
                    // `TransitionTimedOut` carrying the last state read —
                    // the same report it produced before.
                    Err(err)
                        if reestablishment_may_be_retried(&err)
                            && silence_is_permitted(last_state) =>
                    {
                        // Cleared, and not merely left alone. [`Self::connect`]
                        // populates `self.connection` before it authorises and
                        // before it asserts Verify Mode, so a failure in either
                        // returns `Err` with a half-established connection still
                        // in the field — and a device that T_ACKs and withholds
                        // the answer, which is exactly the device NOTE 86
                        // describes, fails in precisely that way. Left in place,
                        // the next iteration would see `Some(_)`, skip the retry
                        // this arm just promised, and poll a connection that
                        // never finished coming up.
                        self.connection = None;
                        false
                    }
                    Err(err) => return Err(err),
                },
            };
            // Only when there is something to read on. A tolerated failed
            // re-establishment skips straight to the deadline check and the
            // sleep, rather than asking a `None` connection for a state and
            // collecting a `NotConnected` this loop would then have to
            // pretend to understand.
            let read = match connected {
                true => self.read_load_state(object_index).await.map(Some),
                false => Ok(None),
            };
            match read {
                Ok(None) => {}
                Ok(Some(state)) => {
                    last_state = Some(state);
                    if outcomes.is_settled(state) {
                        return Ok(state);
                    }
                    if !outcomes.accepts(state) && state != from {
                        // Waiting does not turn a state Table 94 forbids
                        // into one it permits, and reporting it as a
                        // time-out thirty seconds later would hide what was
                        // actually read. The state the device started in is
                        // excluded: a machine that has not moved *yet* is
                        // not a machine that moved somewhere illegal, and
                        // the deadline decides that one.
                        return Err(SessionError::IllegalTransition {
                            object_index,
                            event,
                            from,
                            observed: state,
                        });
                    }
                }
                Err(SessionError::ConnectionLost { .. }) => {
                    // RES §4.23.2.4.1 asks for exactly this: re-establish
                    // the connection during the maximum transition time and
                    // carry on polling.
                    self.connection = None;
                }
                Err(SessionError::NoAnswer { .. }) if silence_is_permitted(last_state) => {
                    // `[D]` RES Table 94's footnote: a device in
                    // LoadCompleting may legitimately not answer. Silence
                    // here is not failure (spec §5.5, §9.2 row 2).
                    //
                    // Since C19 this arm means one thing only: the device
                    // acknowledged the read at the Transport Layer and
                    // withheld the answer. `[D]` RES §4.23.2.4.1: *"If the
                    // MaS responds during state LoadCompleting, an
                    // established TL-connection is kept alive"* — the T_ACK
                    // is that response, so the connection stays and the next
                    // poll goes out on it with no reconnect.
                }
                Err(SessionError::ConnectionReleased { .. })
                    if silence_is_permitted(last_state) =>
                {
                    // The other half of the same RES sentence: *"otherwise,
                    // the TL-connection will close after TL-timeout"*
                    // (§4.23.2.4.1, p. 297). Nothing answered at all, so
                    // `exchange_inner` ran TL's action A6 and the connection
                    // is CLOSED. Tolerated exactly as the silence above is —
                    // NOTE 86 expects an offline device here — but the next
                    // poll may not use the connection that no longer exists,
                    // so the loop re-establishes it first. Set explicitly
                    // rather than relied upon, so this arm does not depend on
                    // what the layer below it did with the field.
                    self.connection = None;
                }
                Err(err) => return Err(err),
            }
            if started.elapsed() >= self.timing.max_transition {
                if made_the_one_more_attempt {
                    return Err(SessionError::TransitionTimedOut {
                        object_index,
                        event,
                        last_state,
                        waited: started.elapsed(),
                    });
                }
                // `[D]` RES §4.23.2.4.1's leading condition: *"If a before
                // established TL-connection breaks down, the MaC shall try
                // to re-establish the connection periodically during the
                // maximum transition time and once more when the maximum
                // transition time has passed."* The loop above already is
                // the periodic part.
                //
                // (**[A]**, this project's and not the Standard's): the
                // clause is conditioned on a connection that broke down;
                // this code grants the one extra attempt unconditionally,
                // including to a connection that never dropped and a
                // device that is simply slow. From here, a dropped
                // connection and a slow-but-still-connected one are
                // indistinguishable, and the generous reading only costs
                // one extra poll on a path that is already failing.
                //
                // Setting the flag without returning lets exactly one
                // further iteration — reconnect included — run past the
                // deadline before the next crossing gives up.
                made_the_one_more_attempt = true;
            }
            tokio::time::sleep(self.timing.poll_interval).await;
        }
    }

    /// Writes a whole region, in chunks, verifying every single one.
    ///
    /// There is no unverified variant of this function, and that is the
    /// enforcement of spec §6.2's project rule (**[A]**, this project's and
    /// not the Standard's): *"no write path without a client-side
    /// verification read"*. Both of §6.2's branches live here —
    /// [`VerifyMode::Active`] compares the device's own
    /// `A_Memory_Write.res`, [`VerifyMode::Unavailable`] issues MP §3.16's
    /// explicit `A_Memory_Read` after the programming delay.
    pub async fn write_memory_region(
        &mut self,
        base: u32,
        data: &[u8],
        limit: WriteLimit,
    ) -> Result<usize, SessionError> {
        self.write_memory_region_as(base, data, limit, WriteScope::Download)
            .await
    }

    async fn write_memory_region_as(
        &mut self,
        base: u32,
        data: &[u8],
        limit: WriteLimit,
        scope: WriteScope,
    ) -> Result<usize, SessionError> {
        self.authorise_write(scope)?;
        let plan = chunks(base, data, limit)?;
        for chunk in &plan {
            let slice = &data[chunk.offset..chunk.offset + usize::from(chunk.length)];
            self.write_one_chunk(chunk.address, slice, chunk.service, scope)
                .await?;
        }
        Ok(plan.len())
    }

    /// Reads `curr_prog_mode`, and writes it back inverted only if the mode
    /// is not already the one asked for.
    ///
    /// The derivation is [`prog_mode_write`]'s and stays there: `old XOR
    /// 10000001b`, so bits 1–6 survive and the parity is inverted with the
    /// mode rather than computed from a convention this project has not
    /// established. Returning [`ProgModeWrite::AlreadyInRequestedMode`]
    /// means no frame was sent at all, which is
    /// `[D]` RES §4.26.3.4.2/§4.26.3.4.3's guard and not an optimisation.
    ///
    /// Simulator-only in this phase, like every other write, and for one
    /// extra reason: design spec §4.4 records that on System B the meaning of
    /// this octet is **not established**.
    ///
    /// The addendum's demand that phase 2 provide *"no path that fires it"* is
    /// honoured by the two gates below — the caller must hold
    /// [`WriteScope::ProgrammingModeToggle`], and `check_write_target()` then
    /// refuses any transport that is not the simulator — rather than by the
    /// method's absence, and no caller in this repository reaches it with a
    /// hardware transport.
    pub async fn set_programming_mode(
        &mut self,
        enable: bool,
    ) -> Result<ProgModeWrite, SessionError> {
        self.authorise_write(WriteScope::ProgrammingModeToggle)?;
        let address = u32::from(CURR_PROG_MODE_ADDRESS);
        let octets = self.read_memory(address, 1).await?;
        let old = *octets
            .first()
            .ok_or(SessionError::MemoryRefused { address })?;
        let decision = prog_mode_write(old, enable);
        if let ProgModeWrite::Write(octet) = decision {
            let limit = write_limit(ApduLengthSource::Absent);
            self.write_memory_region_as(
                address,
                &[octet],
                limit,
                WriteScope::ProgrammingModeToggle,
            )
            .await?;
        }
        Ok(decision)
    }

    /// `A_Restart`, Basic Restart (spec §8; MP §3.7.1.1.1): unconfirmed at
    /// the Application Layer, so what this call proves is only that the
    /// Transport Layer accepted the request — MP §3.7.1.1.3: *"The
    /// Application Layer of the Management Server shall not confirm the
    /// A_Restart-service if a Basic Restart is called."* MP §2.3 step 4,
    /// p. 15 is exactly this call, immediately followed by aborting the
    /// client-side connection, which is what happens here via
    /// [`Self::disconnect_after_restart`].
    ///
    /// A caller that must know whether the device actually restarted
    /// reconnects itself, no earlier than
    /// [`SessionTiming::restart_basic_t1`] and not giving up before
    /// [`SessionTiming::restart_responsive_again`] (MP §3.7.1.1.2, p. 79)
    /// — this method does not reconnect, because after a restart the
    /// device is met with a fresh connection, which is a new session's
    /// job, not this one's.
    pub async fn restart_basic(&mut self) -> Result<(), SessionError> {
        self.authorise_write(WriteScope::Restart)?;
        self.send_acknowledged(
            ApplicationService::Restart {
                response: false,
                restart_type: 0,
                data: Vec::new(),
            },
            "T_ACK for A_Restart (Basic Restart)",
        )
        .await?;
        self.disconnect_after_restart().await;
        Ok(())
    }

    /// `A_Restart`, Master Reset (spec §8; MP §3.7.1.2.1): confirmed, so
    /// unlike [`Self::restart_basic`] this returns what the device
    /// actually reported.
    ///
    /// `erase_code` and `channel_number` are passed through uninterpreted,
    /// per cemi.rs's own refusal to guess which octets belong to which
    /// restart type — Table 4 (MP §3.7.1.2.3.1, p. 82) is the caller's
    /// reference, and [`ERASE_CODE_CONFIRMED_RESTART`] is the one value
    /// this module names, because it is the Erase Code that makes a
    /// Master Reset behave like a confirmed Basic Restart and nothing
    /// else.
    ///
    /// MP §3.7.1.2.2, p. 81: *"The Management Server shall execute the
    /// Master Reset only after the A_Restart_Response-PDU has been sent on
    /// the bus"* — so the response this returns is guaranteed to have gone
    /// out before the device started resetting anything, whatever
    /// `error_code` says. [`MasterResetResponse::recovery_wait`] is the
    /// timing this response feeds into next.
    pub async fn restart_master_reset(
        &mut self,
        erase_code: u8,
        channel_number: u8,
    ) -> Result<MasterResetResponse, SessionError> {
        self.authorise_write(WriteScope::Restart)?;
        let data = self
            .exchange(
                ApplicationService::Restart {
                    response: false,
                    restart_type: 1,
                    data: vec![erase_code, channel_number],
                },
                "A_Restart_Response (Master Reset)",
                |service| match service {
                    ApplicationService::Restart {
                        response: true,
                        restart_type: 1,
                        data,
                    } => Some(data.clone()),
                    _ => None,
                },
            )
            .await?;
        let response = decode_master_reset_response(&data)?;
        self.disconnect_after_restart().await;
        Ok(response)
    }

    /// MP §3.7.3 exception (5), p. 90: after any `A_Restart`, this session
    /// sends its own `T_Disconnect` regardless of whether the device sends
    /// one — *"the Management Server (device) may or may not break down
    /// the Transport Layer connection… nevertheless an explicit
    /// DM_Disconnect procedure shall follow"* — and then does not return
    /// until [`SessionTiming::post_restart_disconnect_wait`] has fully
    /// elapsed, *"regardless of whether or not a T_Disconnect-PDU is
    /// received"*: a device that disconnects early does not shorten this
    /// wait, because the two sides' T_Disconnects can cross on the wire
    /// with no way to tell which is which — *"the configuration shall not
    /// be continued while this time-out has not elapsed"*.
    ///
    /// [`Self::disconnect`] is this method's ordinary, patience-free
    /// cousin and stays that way for every other caller: this is the one
    /// path MP §3.7.3 puts a mandatory wait on.
    async fn disconnect_after_restart(&mut self) {
        self.disconnect().await;
        tokio::time::sleep(self.timing.post_restart_disconnect_wait).await;
    }

    async fn write_one_chunk(
        &mut self,
        address: u32,
        data: &[u8],
        service: MemoryService,
        scope: WriteScope,
    ) -> Result<(), SessionError> {
        self.authorise_write(scope)?;
        let verify = self
            .connection
            .map(|state| state.verify_mode)
            .ok_or(SessionError::NotConnected)?;
        let request = match service {
            MemoryService::Memory => ApplicationService::MemoryWrite {
                address: address as u16,
                data: data.to_vec(),
            },
            MemoryService::UserMemory => ApplicationService::UserMemoryWrite {
                address,
                data: data.to_vec(),
            },
        };
        match verify {
            VerifyMode::Active => {
                let read = self
                    .exchange(request, "A_Memory_Write.res", |answer| match answer {
                        ApplicationService::MemoryResponse { address: at, data }
                            if u32::from(*at) == address =>
                        {
                            Some(data.clone())
                        }
                        ApplicationService::UserMemoryResponse { address: at, data }
                            if *at == address =>
                        {
                            Some(data.clone())
                        }
                        _ => None,
                    })
                    .await?;
                if read.is_empty() {
                    // `[D]` AL §3.5.4: *"in case of a failed write operation
                    // the field number of the A_Memory_Response-PDU shall be
                    // zero and there shall be no field data"*.
                    return Err(SessionError::MemoryRefused { address });
                }
                if read != data {
                    return Err(SessionError::ReadBackMismatch {
                        address,
                        written: data.to_vec(),
                        read,
                    });
                }
            }
            VerifyMode::Unavailable => {
                // `[D]` AL §3.5.4: with Verify Mode inactive the device
                // *"shall not respond"*, so the T_ACK is the only
                // acknowledgement there will be — and therefore the one that
                // has to be waited for, with TL clause 4's time-out and
                // `max_rep_count = 3` applying to it like any other request.
                self.send_acknowledged(request, "a T_ACK for the memory write")
                    .await?;
                tokio::time::sleep(self.timing.programming_delay).await;
                let number = u8::try_from(data.len()).expect(
                    "a chunk is at most SERVICE_MAX_OCTETS = 63 octets \
                     (AL §3.5.3/§3.5.4), so its length fits one octet",
                );
                let read = self.read_memory_as(service, address, number).await?;
                if read != data {
                    return Err(SessionError::ReadBackMismatch {
                        address,
                        written: data.to_vec(),
                        read,
                    });
                }
            }
        }
        Ok(())
    }
}

/// Whether the last state the device reported is one RES Table 94 permits it
/// to fall silent in.
///
/// `None` — nothing read yet on this wait — is not permission: a device that
/// has never answered has not earned the benefit of the doubt.
fn silence_is_permitted(last_state: Option<LoadState>) -> bool {
    last_state.map(LoadState::may_be_silent).unwrap_or(false)
}

/// Whether a failed re-establishment is one RES §4.23.2.4.1's
/// *"periodically"* asks to try again, rather than a device fault to report.
///
/// Every variant here is the connection failing to come up; nothing here is
/// the device answering something wrong. A refused property, a mismatched
/// read-back or a Verify Mode the device would not take are faults the
/// caller has to see, and they are absent from this list on purpose.
///
/// The list is not, however, every way a re-establishment can fail.
/// [`SessionError::Lagged`] is a local broadcast-buffer overrun — this
/// client fell behind its own event stream — and is deliberately excluded:
/// it is not evidence that the device was silent, so retrying it would
/// paper over a client-side fault with a device-side excuse. It was fatal
/// here before C19 and it stays fatal.
fn reestablishment_may_be_retried(err: &SessionError) -> bool {
    matches!(
        err,
        SessionError::NoAnswer { .. }
            | SessionError::ConnectionReleased { .. }
            | SessionError::ConnectRejected { .. }
            | SessionError::ConnectionLost { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::simulator::{Seen, SimulatedDevice, SimulatorConfig};
    use super::*;
    use crate::cemi::LDataFrame;
    use knx_core::commissioning::authorisation::AccessKey;
    use knx_core::commissioning::programming_mode::CURR_PROG_MODE_ADDRESS;
    use std::ops::Range;

    /// The cited timings with the waiting taken out. A test of the shape of
    /// the §5.5 loop has no business taking thirty seconds to find out that
    /// the loop has the right shape.
    fn fast() -> SessionTiming {
        SessionTiming {
            connection_timeout: Duration::from_millis(50),
            response_timeout: Duration::from_millis(50),
            poll_interval: Duration::from_millis(1),
            max_transition: Duration::from_millis(40),
            programming_delay: Duration::from_millis(0),
            restart_basic_t1: Duration::from_millis(1),
            restart_responsive_again: Duration::from_millis(5),
            post_restart_disconnect_wait: Duration::from_millis(6),
        }
    }

    fn addr(area: u8, line: u8, device: u8) -> IndividualAddress {
        IndividualAddress::new(area, line, device).expect("a valid individual address")
    }

    fn simulator_authorisation(device: &SimulatedDevice, scope: WriteScope) -> WriteAuthorisation {
        WriteAuthorisation::for_simulator(device.address(), scope)
            .expect("the simulated device is not an excluded address")
    }

    fn read_only<'t>(device: &'t SimulatedDevice) -> ManagementSession<'t, SimulatedDevice> {
        ManagementSession::read_only(device, device.address(), AuthorisationPlan::Skip, fast())
            .expect("the simulated device is contactable")
    }

    fn writer<'t>(
        device: &'t SimulatedDevice,
        scope: WriteScope,
    ) -> ManagementSession<'t, SimulatedDevice> {
        ManagementSession::authorised(
            device,
            AuthorisationPlan::Skip,
            fast(),
            simulator_authorisation(device, scope),
        )
        .expect("a simulator authorisation against a simulator transport is accepted")
    }

    fn device_control_writes(device: &SimulatedDevice) -> usize {
        device
            .seen()
            .iter()
            .filter(|entry| {
                matches!(
                    entry,
                    Seen::PropertyWrite {
                        property_id: PID_DEVICE_CONTROL,
                        ..
                    }
                )
            })
            .count()
    }

    /// A transport that has not said it is a simulator, which is what every
    /// real one looks like: [`ManagementTransport::target_kind`] defaults to
    /// [`TargetKind::Hardware`] and this type does not override it.
    struct HardwareLikeTransport(SimulatedDevice);

    impl ManagementTransport for HardwareLikeTransport {
        fn assigned_address(&self) -> IndividualAddress {
            self.0.assigned_address()
        }

        fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
            self.0.subscribe()
        }

        async fn send_frame(
            &self,
            destination: Destination,
            transport: Tpci,
            service: ApplicationService,
        ) -> Result<(), BusError> {
            self.0.send_frame(destination, transport, service).await
        }
    }

    struct ConnectConfirmationTransport {
        assigned: IndividualAddress,
        events: broadcast::Sender<TunnelEvent>,
    }

    impl ConnectConfirmationTransport {
        fn new(assigned: IndividualAddress) -> Self {
            let (events, _) = broadcast::channel(16);
            Self { assigned, events }
        }

        fn confirm_connect_with_error(&self, target: IndividualAddress, error: bool) {
            let _ = self.events.send(TunnelEvent::Telegram(LDataFrame {
                kind: LDataMessageKind::Confirmation { error },
                source: self.assigned,
                destination: Destination::Individual(target),
                transport: Tpci::Connect,
                service: ApplicationService::NoApplicationPdu,
            }));
        }

        fn confirm_connect(&self, target: IndividualAddress) {
            self.confirm_connect_with_error(target, false);
        }
    }

    impl ManagementTransport for ConnectConfirmationTransport {
        fn assigned_address(&self) -> IndividualAddress {
            self.assigned
        }

        fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
            self.events.subscribe()
        }

        async fn send_frame(
            &self,
            _destination: Destination,
            _transport: Tpci,
            _service: ApplicationService,
        ) -> Result<(), BusError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn connect_waits_for_the_matching_current_target_confirmation() {
        let assigned = addr(1, 1, 249);
        let stale_target = addr(1, 1, 1);
        let current_target = addr(1, 1, 2);
        let transport = ConnectConfirmationTransport::new(assigned);
        let mut session = ManagementSession::read_only(
            &transport,
            current_target,
            AuthorisationPlan::Skip,
            fast(),
        )
        .expect("build read-only session");
        let mut connecting = Box::pin(session.connect());

        assert!(
            tokio::time::timeout(Duration::from_millis(10), &mut connecting)
                .await
                .is_err(),
            "gateway acceptance alone must not establish the device connection"
        );

        transport.confirm_connect(stale_target);
        assert!(
            tokio::time::timeout(Duration::from_millis(10), &mut connecting)
                .await
                .is_err(),
            "a late confirmation for the previous target must be ignored"
        );

        transport.confirm_connect(current_target);
        tokio::time::timeout(Duration::from_millis(10), connecting)
            .await
            .expect("matching confirmation should complete promptly")
            .expect("matching positive confirmation establishes the session");
    }

    #[tokio::test]
    async fn connect_uses_the_transport_layer_connection_timeout() {
        let transport = ConnectConfirmationTransport::new(addr(1, 1, 249));
        let timing = fast();
        let mut session = ManagementSession::read_only(
            &transport,
            addr(1, 1, 2),
            AuthorisationPlan::Skip,
            timing,
        )
        .expect("build read-only session");

        let error = session
            .connect()
            .await
            .expect_err("no matching L_Data.con must not establish a session");
        assert!(matches!(
            error,
            SessionError::NoAnswer {
                waiting_for: "to positive L_Data.con for T_Connect",
                each,
                attempts: 1,
            } if each == timing.connection_timeout
        ));
        assert!(session.connection().is_none());
    }

    #[tokio::test]
    async fn connect_stops_immediately_on_a_matching_negative_confirmation() {
        let target = addr(1, 1, 2);
        let transport = ConnectConfirmationTransport::new(addr(1, 1, 249));
        let mut session =
            ManagementSession::read_only(&transport, target, AuthorisationPlan::Skip, fast())
                .expect("build read-only session");
        let mut connecting = Box::pin(session.connect());
        assert!(
            tokio::time::timeout(Duration::from_millis(1), &mut connecting)
                .await
                .is_err(),
            "connect must be waiting before the confirmation arrives"
        );

        transport.confirm_connect_with_error(target, true);
        let result = tokio::time::timeout(Duration::from_millis(10), connecting)
            .await
            .expect("a negative confirmation is a final result, not a timeout");
        assert!(matches!(
            result,
            Err(SessionError::ConnectRejected { target: rejected }) if rejected == target
        ));
    }

    #[test]
    fn default_connection_timeout_is_the_transport_layer_six_seconds() {
        assert_eq!(
            SessionTiming::default().connection_timeout,
            CONNECTION_TIMEOUT
        );
        assert_eq!(CONNECTION_TIMEOUT, Duration::from_secs(6));
    }

    /// MP §3.7.1.1.2, p. 79 (t1, t2) and MP §3.7.3 exception (5), p. 90 (the
    /// post-restart disconnect wait): the three quantities the brief
    /// forbids rounding or collapsing, checked as three quantities and not
    /// as one "restart is roughly five-ish seconds" figure.
    #[test]
    fn default_restart_timings_match_mp_section_3_7_verbatim() {
        let timing = SessionTiming::default();
        assert_eq!(timing.restart_basic_t1, Duration::from_secs(1));
        assert_eq!(timing.restart_responsive_again, Duration::from_secs(5));
        assert_eq!(timing.post_restart_disconnect_wait, Duration::from_secs(6));
    }

    // ------------------------------------------------------- the refusals

    /// §2.3, and the brief's own demand: the write half of every entry point is
    /// unreachable without an authorisation value, and the proof is that the
    /// device saw nothing.
    #[tokio::test]
    async fn no_write_entry_point_is_reachable_without_an_authorisation() {
        let device = SimulatedDevice::new();
        let mut session = read_only(&device);
        session.connect().await.expect("connecting is a read");

        let limit = write_limit(ApduLengthSource::Absent);
        let attempts: Vec<SessionError> = vec![
            session
                .write_property(
                    ObjectIndex::DEVICE,
                    PID_DEVICE_CONTROL,
                    vec![0x04],
                    WriteScope::Download,
                )
                .await
                .expect_err("a property write needs an authorisation"),
            session
                .write_load_event(ObjectIndex::APPLICATION_PROGRAM, LoadEvent::StartLoading)
                .await
                .expect_err("a load event needs an authorisation"),
            session
                .write_memory_region(0x4000, &[0x01, 0x02], limit)
                .await
                .expect_err("a memory write needs an authorisation"),
            session
                .set_programming_mode(true)
                .await
                .expect_err("a programming-mode toggle needs an authorisation"),
            session
                .restart_basic()
                .await
                .expect_err("a Basic Restart needs an authorisation"),
            session
                .restart_master_reset(ERASE_CODE_CONFIRMED_RESTART, 0)
                .await
                .expect_err("a Master Reset needs an authorisation"),
        ];

        for error in &attempts {
            assert!(
                matches!(error, SessionError::NoAuthorisation { .. }),
                "every write must be refused for want of an authorisation, got {error}"
            );
        }
        assert!(
            !device.memory_was_written(),
            "no octet may reach the device's memory: {:?}",
            device.seen()
        );
        assert_eq!(
            device_control_writes(&device),
            0,
            "a read-only session must not even set Verify Mode, which is a write"
        );
    }

    /// §15 and §13 R11: a transport that has not declared itself a
    /// simulator cannot be written to in this phase, whatever the
    /// authorisation says.
    #[tokio::test]
    async fn a_transport_that_is_not_a_simulator_is_refused_at_construction() {
        let device = SimulatedDevice::new();
        let target = device.address();
        let hardware = HardwareLikeTransport(device);
        let authorisation = WriteAuthorisation::for_simulator(target, WriteScope::Download)
            .expect("the simulated address is contactable");

        let refusal = ManagementSession::authorised(
            &hardware,
            AuthorisationPlan::Skip,
            fast(),
            authorisation,
        )
        .expect_err("a hardware-shaped transport must not accept a write session");

        match refusal {
            SessionError::NotASimulator {
                transport,
                authorised,
                ..
            } => {
                assert_eq!(transport, TargetKind::Hardware);
                assert_eq!(authorised, TargetKind::Simulator);
            }
            other => panic!("expected a simulator refusal, got {other}"),
        }
        assert!(
            !hardware.0.memory_was_written(),
            "nothing may have been sent before the refusal"
        );
    }

    /// The other half of the same gate: a hardware authorisation, complete
    /// with the operator's confirmation phrase, still does not run in phase
    /// 2 — not even against the simulator.
    #[tokio::test]
    async fn a_hardware_authorisation_does_not_run_in_this_phase() {
        let device = SimulatedDevice::new();
        let target = device.address();
        let phrase = knx_core::commissioning::mutation::required_confirmation_phrase(
            target,
            WriteScope::Download,
        );
        let authorisation = WriteAuthorisation::for_hardware(target, WriteScope::Download, &phrase)
            .expect("the phrase is the required one");

        let refusal =
            ManagementSession::authorised(&device, AuthorisationPlan::Skip, fast(), authorisation)
                .expect_err("a hardware authorisation must not be usable in phase 2");
        assert!(matches!(refusal, SessionError::NotASimulator { .. }));
    }

    /// §2.1, §14 item 11: the alarm panel is refused by the guard before a
    /// session exists, and the refusal is reported rather than silent.
    #[tokio::test]
    async fn the_excluded_address_is_refused_before_a_session_exists() {
        let device = SimulatedDevice::new();
        let refusal =
            ManagementSession::read_only(&device, addr(1, 1, 220), AuthorisationPlan::Skip, fast())
                .expect_err("the excluded address must not be contactable");
        match refusal {
            SessionError::Excluded(excluded) => {
                assert!(
                    excluded.to_string().contains("1.1.220"),
                    "the refusal must name the address it refused: {excluded}"
                );
            }
            other => panic!("expected an exclusion refusal, got {other}"),
        }
        assert!(device.seen().is_empty(), "not one frame may have been sent");
    }

    /// A session must not manage the address the gateway gave this
    /// connection.
    #[tokio::test]
    async fn a_session_refuses_to_address_itself() {
        let device = SimulatedDevice::new();
        let own = device.assigned_address();
        let refusal = ManagementSession::read_only(&device, own, AuthorisationPlan::Skip, fast())
            .expect_err("a session must not target its own address");
        assert!(matches!(refusal, SessionError::SelfAddressed(_)));
    }

    // -------------------------------------------------- reads and answers

    #[tokio::test]
    async fn a_read_only_session_reads_state_without_writing_anything() {
        let device = SimulatedDevice::new();
        device.preset_load_state(ObjectIndex::APPLICATION_PROGRAM, LoadState::Loaded);
        let mut session = read_only(&device);
        session.connect().await.expect("connect");

        assert_eq!(
            session
                .read_load_state(ObjectIndex::APPLICATION_PROGRAM)
                .await
                .expect("the load state is readable"),
            LoadState::Loaded
        );
        assert_eq!(
            session
                .read_manufacturer_id()
                .await
                .expect("the manufacturer id is readable"),
            0x0002
        );
        assert_eq!(session.verify_mode(), Some(VerifyMode::Unavailable));
        assert_eq!(device_control_writes(&device), 0);
    }

    /// §6.4 / §14 item 13, the transport half: where the length is read
    /// from decides what it means.
    #[tokio::test]
    async fn apdu_length_discovery_prefers_the_device_object_and_ignores_the_router() {
        let device = SimulatedDevice::new();
        let mut session = read_only(&device);
        session.connect().await.expect("connect");
        assert_eq!(
            session.read_apdu_length(None).await.expect("read"),
            ApduLengthSource::DeviceObject(15)
        );

        let router_only = SimulatedDevice::with_config(SimulatorConfig {
            max_apdu_length: None,
            router_max_apdu_length: Some(254),
            ..SimulatorConfig::default()
        });
        let mut session = read_only(&router_only);
        session.connect().await.expect("connect");
        let source = session
            .read_apdu_length(Some(ObjectIndex::new(6)))
            .await
            .expect("read");
        assert_eq!(source, ApduLengthSource::RouterObjectOnly(254));
        assert_eq!(
            write_limit(source).max_octets(),
            12,
            "a length found only in the Router Object does not govern management"
        );

        let absent = SimulatedDevice::with_config(SimulatorConfig {
            max_apdu_length: None,
            ..SimulatorConfig::default()
        });
        let mut session = read_only(&absent);
        session.connect().await.expect("connect");
        assert_eq!(
            session.read_apdu_length(None).await.expect("read"),
            ApduLengthSource::Absent
        );
    }

    /// §9.2 row 1: silence is a time-out and nothing more. The message must
    /// not name a cause nobody observed.
    #[tokio::test]
    async fn a_silent_device_is_reported_as_silence_and_not_as_a_diagnosis() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            silent: true,
            ..SimulatorConfig::default()
        });
        let mut session = read_only(&device);
        session.connect().await.expect("T_Connect needs no answer");
        let error = session
            .read_load_state(ObjectIndex::APPLICATION_PROGRAM)
            .await
            .expect_err("a silent device cannot answer");
        match &error {
            // C19 moved this outcome from `NoAnswer` to
            // `ConnectionReleased`: nothing acknowledged, so TL's action A6
            // applies. The subject of this test is unchanged — the message
            // must still refuse to name a cause it did not observe.
            SessionError::ConnectionReleased { .. } => {
                // The transmission count has its own test, against the
                // literal 4, below — comparing it here against
                // `MAX_TRANSMISSIONS` would just check the production code
                // against itself. This test's subject is the message.
                assert!(error.to_string().contains("does not distinguish"));
            }
            other => panic!("expected a released connection, got {other}"),
        }
    }

    /// C4, TL §3, p. 15: 3 repetitions of the original send is 4
    /// transmissions, and the fourth must actually go out — not stop at
    /// the third the way `attempts >= MAX_REP_COUNT` used to.
    #[tokio::test]
    async fn a_connected_exchange_makes_exactly_four_transmissions_before_giving_up() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            // Every transmission is dropped, so the count in the resulting
            // `NoAnswer` is exactly how many the client sent — not how many
            // more it would have sent had the device answered.
            silent: true,
            ..SimulatorConfig::default()
        });
        let mut session = read_only(&device);
        session.connect().await.expect("T_Connect needs no answer");
        let error = session
            .read_load_state(ObjectIndex::APPLICATION_PROGRAM)
            .await
            .expect_err("a silent device cannot answer");
        match error {
            SessionError::ConnectionReleased { attempts, .. } => {
                assert_eq!(attempts, 4, "the original send plus 3 repetitions");
            }
            other => panic!("expected a released connection, got {other}"),
        }
    }

    /// C4: a device that stays silent for the original send and the first
    /// two repetitions, then answers on what TL §3 calls the third and
    /// final repetition — the fourth transmission overall — must still be
    /// heard. `MAX_REP_COUNT` transmissions never leaving the wire would
    /// make this one time out instead.
    #[tokio::test]
    async fn an_answer_on_the_fourth_transmission_is_accepted() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            silent_for_first_numbered_data_frames: 3,
            ..SimulatorConfig::default()
        });
        let mut session = read_only(&device);
        session.connect().await.expect("T_Connect needs no answer");
        let state = session
            .read_load_state(ObjectIndex::APPLICATION_PROGRAM)
            .await
            .expect("the fourth transmission must be answered");
        assert_eq!(
            state,
            LoadState::Unloaded,
            "the device answers normally once it stops dropping frames"
        );
    }

    /// C19, TL §3.9, p. 15: *"If it fails, the local Transport Layer shall
    /// pass a T_Disconnect.ind primitive to the local user indicating that
    /// the connection is released (state = CLOSED)."* The state machine
    /// spells out the mechanism — `E18` (§5.2, p. 19) in `OPEN_WAIT` is
    /// `CLOSED` under action `A6` (§5.4.1's table, p. 22), and `A6` (§5.3,
    /// p. 19) sends `T_DISCONNECT_REQ_PDU` first.
    ///
    /// Three things are asserted, because the fix has three halves and a
    /// test that checked only the error variant would pass against code
    /// that renamed the error and kept the stale connection.
    #[tokio::test]
    async fn exhausting_the_repetitions_releases_the_connection() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            silent: true,
            ..SimulatorConfig::default()
        });
        let mut session = read_only(&device);
        session.connect().await.expect("T_Connect needs no answer");
        assert!(
            session.connection().is_some(),
            "the connection has to exist before this test can watch it go"
        );

        let error = session
            .read_load_state(ObjectIndex::APPLICATION_PROGRAM)
            .await
            .expect_err("a silent device cannot answer");

        match error {
            SessionError::ConnectionReleased { attempts, .. } => {
                assert_eq!(attempts, 4, "the original send plus 3 repetitions");
            }
            other => panic!("expected a released connection, got {other}"),
        }
        assert!(
            session.connection().is_none(),
            "TL §3.9: state = CLOSED. A session that kept its connection here \
             would send its next request to a peer that has forgotten the \
             connection exists"
        );
        assert_eq!(
            device
                .seen()
                .iter()
                .filter(|entry| matches!(entry, Seen::Disconnect))
                .count(),
            1,
            "action A6's first line is the T_DISCONNECT_REQ_PDU, and a local \
             state change on its own is not A6"
        );
        assert_eq!(
            session.reconnects(),
            0,
            "releasing is not reconnecting: nothing here asked for a new \
             connection"
        );
    }

    /// C19: the two silences are not the same silence, and the wait loop
    /// has to treat them differently.
    ///
    /// A device that T_ACKs the poll and withholds the answer is RES
    /// §4.23.2.4.1's *"If the MaS responds during state LoadCompleting, an
    /// established TL-connection is kept alive"* — one transmission, no
    /// release, no reconnect. A device that acknowledges nothing is the
    /// same clause's *"otherwise, the TL-connection will close after
    /// TL-timeout"*, with NOTE 86 naming the cause: *"A device may be
    /// offline during state LoadCompleting."* — four transmissions, action
    /// A6, and a connection the loop must re-establish before it polls
    /// again.
    ///
    /// Both halves run the same wait against the same states and differ
    /// only in whether the T_ACK comes back, so the assertions below are a
    /// difference and not two unrelated measurements. Neither asserts on
    /// elapsed time: the counts are what the Standard constrains.
    #[tokio::test]
    async fn the_wait_loop_tells_a_released_connection_from_a_silent_device() {
        async fn wait_out(config: SimulatorConfig) -> (SimulatedDevice, u32, bool, usize, usize) {
            let device = SimulatedDevice::with_config(config);
            // Settling by answered reads rather than by elapsed time: the
            // first read the device answers reports `LoadCompleting`, which
            // is what earns the wait its Table 94 permission to tolerate
            // silence, and the next one it answers reports `Loaded`. A read
            // that goes quiet spends nothing, so no assertion below depends
            // on how long a quiet poll took.
            device.preset_load_state(ObjectIndex::APPLICATION_PROGRAM, LoadState::Loaded);
            device.preset_load_completing_polls(ObjectIndex::APPLICATION_PROGRAM, 1);
            let mut session = ManagementSession::read_only(
                &device,
                device.address(),
                AuthorisationPlan::Skip,
                SessionTiming {
                    // Long enough that the deadline plays no part: this test
                    // is about what the loop does with a silence, not about
                    // when it gives up on one.
                    max_transition: Duration::from_secs(30),
                    ..fast()
                },
            )
            .expect("the simulated device is contactable");
            session.connect().await.expect("connect");
            let outcomes = permitted_outcomes(
                LoadState::Loading,
                Stimulus::Event(LoadEvent::LoadCompleted),
                None,
            );
            let observed = tokio::time::timeout(
                Duration::from_secs(5),
                session.wait_for_load_state(
                    ObjectIndex::APPLICATION_PROGRAM,
                    LoadEvent::LoadCompleted,
                    LoadState::Loading,
                    &outcomes,
                ),
            )
            .await
            .expect("the wait must terminate, not spin")
            .expect("the device settles once it stops going quiet");
            assert_eq!(observed, LoadState::Loaded);
            let connects = device
                .seen()
                .iter()
                .filter(|entry| matches!(entry, Seen::Connect))
                .count();
            let disconnects = device
                .seen()
                .iter()
                .filter(|entry| matches!(entry, Seen::Disconnect))
                .count();
            let reconnects = session.reconnects();
            let still_connected = session.connection().is_some();
            (device, reconnects, still_connected, connects, disconnects)
        }

        // Busy, not gone: one transmission, a T_ACK, no answer.
        let (busy, reconnects, still_connected, connects, disconnects) =
            wait_out(SimulatorConfig {
                unanswered_load_state_reads: Some(FIRST_QUIET_READ..FIRST_QUIET_READ + 1),
                ..SimulatorConfig::default()
            })
            .await;
        assert_eq!(
            busy.load_state_reads(),
            3,
            "read #1 answers, read #2 goes quiet, read #3 answers the \
             settled state — the loop keeps polling through a silence it is \
             allowed to tolerate"
        );
        assert!(
            still_connected,
            "the T_ACK is the MaS responding, and RES §4.23.2.4.1 keeps that \
             connection alive"
        );
        assert_eq!(
            reconnects, 0,
            "nothing was released, so nothing was reopened"
        );
        assert_eq!(connects, 1, "the original T_Connect and no other");
        assert_eq!(
            disconnects, 0,
            "a connection that was never released is never torn down"
        );

        // Offline: four transmissions, no T_ACK, action A6.
        let (offline, reconnects, still_connected, connects, disconnects) =
            wait_out(SimulatorConfig {
                unacknowledged_load_state_reads: Some(
                    FIRST_QUIET_READ..FIRST_QUIET_READ + u32::from(MAX_TRANSMISSIONS),
                ),
                ..SimulatorConfig::default()
            })
            .await;
        assert_eq!(
            offline.load_state_reads(),
            1 + u32::from(MAX_TRANSMISSIONS) + 1,
            "read #1 answers, reads #2–#5 are the full ladder of an \
             unacknowledged request, read #6 is the poll after the \
             re-established connection"
        );
        assert!(
            still_connected,
            "the loop re-established the connection it had to release, and \
             returned holding a live one"
        );
        assert_eq!(
            reconnects, 1,
            "RES §4.23.2.4.1: re-establish and carry on polling — exactly \
             once, because the device answers after that"
        );
        assert_eq!(
            connects, 2,
            "the original T_Connect and the one that replaced the released \
             connection"
        );
        assert_eq!(
            disconnects, 1,
            "action A6 sends T_DISCONNECT_REQ_PDU before the state goes \
             CLOSED"
        );
    }

    /// C19's discriminator, pinned on its own: an acknowledged request is
    /// never repeated.
    ///
    /// This is the whole difference between
    /// [`SessionError::NoAnswer`] and [`SessionError::ConnectionReleased`].
    /// `[D]` TL's action `A8` (§5.3, p. 20) stops the acknowledge timer when
    /// the `T_ACK` arrives, so `ACKNOWLEDGE_TIME_OUT_ind` — TL §5.2's `E18`,
    /// p. 19 — can never fire for this request, the connection never reaches
    /// `A6`, and repeating the frame would put a second request on the wire
    /// rather than a retransmission of the first.
    ///
    /// The wait-loop test below cannot see this: there, a repetition and the
    /// next poll are both simply the next read, and the counts come out the
    /// same either way. Here the window swallows every answer, so a
    /// repetition has nowhere to hide.
    #[tokio::test]
    async fn an_acknowledged_request_is_never_repeated() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            // Every read is acknowledged and none is answered.
            unanswered_load_state_reads: Some(1..u32::MAX),
            ..SimulatorConfig::default()
        });
        let mut session = read_only(&device);
        session.connect().await.expect("connect");
        let err = session
            .read_load_state(ObjectIndex::APPLICATION_PROGRAM)
            .await
            .expect_err("the device acknowledges and never answers");
        match err {
            SessionError::NoAnswer { attempts, .. } => assert_eq!(
                attempts, 1,
                "TL action A8 stopped the acknowledge timer: there is \
                 nothing here to repeat"
            ),
            other => panic!(
                "an acknowledged request keeps its connection and reports \
                 NoAnswer, got {other}"
            ),
        }
        assert_eq!(
            device.load_state_reads(),
            1,
            "one transmission, not a ladder of four: repeating an \
             acknowledged request would send a second request"
        );
        assert!(
            session.connection().is_some(),
            "RES §4.23.2.4.1: the T_ACK is the MaS responding, and that \
             connection is kept alive"
        );
    }

    /// C19 fix round: a re-establishment that fails is not a device fault.
    ///
    /// `[D]` RES §4.23.2.4.1: *"If a before established TL-connection breaks
    /// down, the MaC shall try to re-establish the connection
    /// **periodically** during the maximum transition time"*, and NOTE 86
    /// says who is at the other end: *"A device may be offline during state
    /// LoadCompleting. A running TL-connection may be lost..."* An offline
    /// device does not answer `T_Connect` either, so treating one refused
    /// re-establishment as fatal would end the wait at the first attempt
    /// instead of at the deadline.
    ///
    /// Before C19 this path was unreachable: an exhausted read left its
    /// stale connection in the field and nothing was ever re-established.
    /// The device here goes away for good, so the only correct ending is
    /// the deadline's, carrying the last state that was actually read.
    #[tokio::test]
    async fn a_failed_re_establishment_is_retried_until_the_deadline() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            // Read #2 breaks the connection down, and every T_Connect after
            // the session's first one goes unanswered: the device is gone.
            drop_connection_on_load_state_read: Some(2),
            unanswered_connects: Some(2..u32::MAX),
            ..SimulatorConfig::default()
        });
        device.preset_load_state(ObjectIndex::APPLICATION_PROGRAM, LoadState::Loaded);
        device.preset_load_completing_polls(ObjectIndex::APPLICATION_PROGRAM, 1);
        let mut session = ManagementSession::read_only(
            &device,
            device.address(),
            AuthorisationPlan::Skip,
            SessionTiming {
                // Room for several failed re-establishments at 50 ms each,
                // so that "periodically" has something to be periodic over.
                max_transition: Duration::from_millis(300),
                ..fast()
            },
        )
        .expect("the simulated device is contactable");
        session.connect().await.expect("connect");
        let outcomes = permitted_outcomes(
            LoadState::Loading,
            Stimulus::Event(LoadEvent::LoadCompleted),
            None,
        );
        let err = tokio::time::timeout(
            Duration::from_secs(10),
            session.wait_for_load_state(
                ObjectIndex::APPLICATION_PROGRAM,
                LoadEvent::LoadCompleted,
                LoadState::Loading,
                &outcomes,
            ),
        )
        .await
        .expect("the wait must terminate, not spin")
        .expect_err("the device never comes back");
        match err {
            SessionError::TransitionTimedOut { last_state, .. } => assert_eq!(
                last_state,
                Some(LoadState::LoadCompleting),
                "the deadline reports the last state that was read, not the \
                 failure of the attempt that came after it"
            ),
            other => panic!(
                "RES §4.23.2.4.1 asks for periodic re-establishment, so a \
                 refused attempt is not the answer to this wait: got {other}"
            ),
        }
        assert!(
            session.reconnects() >= 3,
            "periodically means more than once: {} attempts were made",
            session.reconnects()
        );
        assert!(
            session.connection().is_none(),
            "a re-establishment that failed leaves no connection behind, \
             half-established or otherwise"
        );
    }

    /// The reads of the first poll that goes quiet, numbered from one and
    /// counting every transmission. Read #1 has to answer, because a wait
    /// that has read nothing has no Table 94 permission to tolerate
    /// anything; the quiet poll is the next one.
    const FIRST_QUIET_READ: u32 = 2;

    /// The whole ladder of one unacknowledged poll: four transmissions, no
    /// `T_ACK` for any of them, TL action `A6` at the end of it.
    fn one_unacknowledged_poll() -> Range<u32> {
        FIRST_QUIET_READ..FIRST_QUIET_READ + u32::from(MAX_TRANSMISSIONS)
    }

    /// The states that make a wait sit in `LoadCompleting` for exactly one
    /// answered read and report `Loaded` on the next one it answers.
    fn completing_once(device: &SimulatedDevice) {
        device.preset_load_state(ObjectIndex::APPLICATION_PROGRAM, LoadState::Loaded);
        device.preset_load_completing_polls(ObjectIndex::APPLICATION_PROGRAM, 1);
    }

    /// A write-capable session whose `connect()` therefore performs the
    /// Verify Mode step, with a deadline far enough away to play no part.
    fn patient_writer<'t>(device: &'t SimulatedDevice) -> ManagementSession<'t, SimulatedDevice> {
        ManagementSession::authorised(
            device,
            AuthorisationPlan::Skip,
            SessionTiming {
                max_transition: Duration::from_secs(30),
                ..fast()
            },
            simulator_authorisation(device, WriteScope::Download),
        )
        .expect("a simulator authorisation against a simulator transport is accepted")
    }

    /// Runs the `LoadCompleting` wait these three tests share, under an
    /// outer time-out so that a loop which stops making progress fails as a
    /// test rather than as a hung suite.
    async fn wait_out_load_completing(
        session: &mut ManagementSession<'_, SimulatedDevice>,
    ) -> Result<LoadState, SessionError> {
        let outcomes = permitted_outcomes(
            LoadState::Loading,
            Stimulus::Event(LoadEvent::LoadCompleted),
            None,
        );
        tokio::time::timeout(
            Duration::from_secs(5),
            session.wait_for_load_state(
                ObjectIndex::APPLICATION_PROGRAM,
                LoadEvent::LoadCompleted,
                LoadState::Loading,
                &outcomes,
            ),
        )
        .await
        .expect("the wait must terminate, not spin")
    }

    fn connects(device: &SimulatedDevice) -> usize {
        device
            .seen()
            .iter()
            .filter(|entry| matches!(entry, Seen::Connect))
            .count()
    }

    /// C19 fix round 2: a half-established connection is not a connection,
    /// and the next poll may not inherit one.
    ///
    /// [`ManagementSession::connect`] puts `self.connection` in place as
    /// soon as the `T_Connect` confirms, and only then authorises and
    /// asserts Verify Mode — spec §10.3 makes those one unit with the
    /// connection, not an optional epilogue to it. A device that
    /// acknowledges the Verify Mode read and withholds the answer is RES
    /// §4.23.2.4.1's *"If the MaS responds during state LoadCompleting, an
    /// established TL-connection is kept alive"* — busy, not gone — and it
    /// makes `connect()` return [`SessionError::NoAnswer`] with that
    /// half-built connection still in the field.
    ///
    /// Left there, the next iteration takes the `Some(_)` branch, skips the
    /// re-establishment this arm just promised, and polls on a connection
    /// whose Verify Mode step never ran. So the assertions below are on what
    /// that costs and not on the field: the device's own Verify Mode bit,
    /// the `PID_DEVICE_CONTROL` write that sets it, and the session's record
    /// of it — which is what spec §6.2 branches every memory write on.
    #[tokio::test]
    async fn a_half_established_connection_is_not_inherited_by_the_next_poll() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            // Poll #2 is acknowledged by nobody, so action A6 releases the
            // connection and the loop has to build another.
            unacknowledged_load_state_reads: Some(one_unacknowledged_poll()),
            // …and that one gets as far as `Some(..)` and no further: its
            // Verify Mode read is acknowledged and never answered.
            unanswered_device_control_reads: Some(2..3),
            ..SimulatorConfig::default()
        });
        completing_once(&device);
        let mut session = patient_writer(&device);
        session.connect().await.expect("connect");

        let observed = wait_out_load_completing(&mut session)
            .await
            .expect("the device answers once a whole connection is up again");

        assert_eq!(observed, LoadState::Loaded);
        assert!(
            device.verify_mode(),
            "the device says Verify Mode's bit 2 is clear while the session \
             says it is connected. Either the wait polled on holding a \
             connection whose Verify Mode step never ran, or the write \
             itself is broken — the Verify Mode tests below tell them apart"
        );
        assert_eq!(
            session.verify_mode(),
            Some(VerifyMode::Active),
            "spec §6.2 picks a write procedure from this value, so a \
             connection that inherited it from a connect that never finished \
             picks the wrong one"
        );
        assert_eq!(
            device_control_writes(&device),
            2,
            "one per connection that finished coming up: the first one and \
             the one that replaced the half-established attempt"
        );
        assert_eq!(
            device.device_control_reads(),
            3,
            "three attempts at the Verify Mode step — the original, the one \
             that went unanswered, and the retry it earned"
        );
        assert_eq!(
            connects(&device),
            3,
            "the original T_Connect, the half-established one, and the \
             re-establishment the tolerated failure promised"
        );
        assert_eq!(
            session.reconnects(),
            2,
            "a failed re-establishment still has to be followed by another \
             one, which is what `periodically` means"
        );
    }

    /// C19 fix round 2: a re-establishment the Data Link Layer refuses is
    /// still a re-establishment that may be tried again.
    ///
    /// `[D]` TL §3.7, p. 13: `T_Connect` goes out with `ack_request` set, so
    /// the `L_Data.con` reports whether the addressed device acknowledged it
    /// at layer 2, and a negative one is [`SessionError::ConnectRejected`].
    /// RES §4.23.2.4.1's NOTE 86 — *"A device may be offline during state
    /// LoadCompleting"* — is exactly the device that fails to acknowledge,
    /// so a refusal here is the expected answer from a device that is coming
    /// back, not a fault to report.
    #[tokio::test]
    async fn a_refused_re_establishment_is_tried_again() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            unacknowledged_load_state_reads: Some(one_unacknowledged_poll()),
            // The first re-establishment is refused at the Data Link Layer;
            // the one after it is not.
            rejected_connects: Some(2..3),
            ..SimulatorConfig::default()
        });
        completing_once(&device);
        let mut session = ManagementSession::read_only(
            &device,
            device.address(),
            AuthorisationPlan::Skip,
            SessionTiming {
                max_transition: Duration::from_secs(30),
                ..fast()
            },
        )
        .expect("the simulated device is contactable");
        session.connect().await.expect("connect");

        let observed = wait_out_load_completing(&mut session)
            .await
            .expect("RES §4.23.2.4.1: a refused attempt is not the answer to this wait");

        assert_eq!(observed, LoadState::Loaded);
        assert_eq!(
            connects(&device),
            2,
            "the refused T_Connect never reached the device, so only the \
             original and the attempt after the refusal are recorded"
        );
        assert_eq!(
            session.reconnects(),
            2,
            "the refusal was an attempt too, and the wait made another"
        );
        assert!(
            session.connection().is_some(),
            "the wait returned holding the connection its last attempt built"
        );
    }

    /// C19 fix round 2: a re-establishment the device tears down mid-way is
    /// likewise tried again.
    ///
    /// The device here answers the `T_Connect` and then sends
    /// `T_DISCONNECT_REQ_PDU` while the client is reading
    /// `PID_DEVICE_CONTROL` for the Verify Mode step, which is
    /// [`SessionError::ConnectionLost`] raised from inside `connect()`
    /// rather than from a poll. NOTE 86's second sentence has this one by
    /// name: *"A running TL-connection may be lost..."*
    #[tokio::test]
    async fn a_re_establishment_the_device_drops_is_tried_again() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            unacknowledged_load_state_reads: Some(one_unacknowledged_poll()),
            // Read #1 belongs to the session's first connect; read #2 is the
            // re-establishment's, and the device drops the connection under
            // it.
            drop_connection_on_device_control_read: Some(2),
            ..SimulatorConfig::default()
        });
        completing_once(&device);
        let mut session = patient_writer(&device);
        session.connect().await.expect("connect");

        let observed = wait_out_load_completing(&mut session)
            .await
            .expect("RES §4.23.2.4.1: a connection lost while being built is rebuilt");

        assert_eq!(observed, LoadState::Loaded);
        assert!(
            device.verify_mode(),
            "the wait did not settle for the connection it lost: the one it \
             returned with completed its Verify Mode step"
        );
        assert_eq!(
            connects(&device),
            3,
            "the original T_Connect, the one the device dropped, and the one \
             that replaced it"
        );
        assert_eq!(
            session.reconnects(),
            2,
            "the lost attempt counted, and the wait made another"
        );
    }

    // ----------------------------------------------------- authorisation

    /// §14 item 14, the scoping ruling: the default path is MP §3.5.1 and
    /// issues exactly one request per connection.
    #[tokio::test]
    async fn the_default_authorisation_path_issues_exactly_one_request() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            key: Some(0x1234_5678),
            key_level: 0,
            ..SimulatorConfig::default()
        });
        let key = AccessKey::new(0x1234_5678).expect("not the free-access sentinel");
        let mut session = ManagementSession::read_only(
            &device,
            device.address(),
            AuthorisationPlan::WithKey(key),
            fast(),
        )
        .expect("contactable");
        session.connect().await.expect("connect");

        assert_eq!(device.authorize_requests(), 1);
        assert_eq!(
            session.connection().expect("connected").authorisation,
            Authorisation::Granted {
                level: AccessLevel::MAXIMUM
            }
        );
        assert!(!session.authorisation_is_suspicious());
    }

    /// §10.9 item 2: with no key the procedure is skipped, and what is
    /// recorded is "free level, unknown value" — not "authorised".
    #[tokio::test]
    async fn no_key_means_no_request_and_no_claim_of_authorisation() {
        let device = SimulatedDevice::new();
        let mut session = read_only(&device);
        session.connect().await.expect("connect");
        assert_eq!(device.authorize_requests(), 0);
        assert_eq!(
            session.connection().expect("connected").authorisation,
            Authorisation::FreeLevelUnknown
        );
    }

    /// §10.2: a wrong key is worse than not trying, and the client must
    /// notice rather than proceed.
    #[tokio::test]
    async fn a_wrong_key_lands_at_the_minimum_level_and_is_noticed() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            key: Some(0x1234_5678),
            key_level: 0,
            ..SimulatorConfig::default()
        });
        let wrong = AccessKey::new(0xDEAD_BEEF).expect("not the sentinel");
        let mut session = ManagementSession::read_only(
            &device,
            device.address(),
            AuthorisationPlan::WithKey(wrong),
            fast(),
        )
        .expect("contactable")
        .with_level_count(LevelCount::Sixteen);
        session.connect().await.expect("connect");

        assert_eq!(
            session.connection().expect("connected").authorisation,
            Authorisation::Granted {
                level: AccessLevel::MINIMUM_OF_SIXTEEN
            }
        );
        assert!(
            session.authorisation_is_suspicious(),
            "the minimum level after supplying a key is at least suspicious"
        );
    }

    /// §10.4 / §14 item 14: the opt-in extension, in both directions. Three
    /// exchanges when the free level was the better one, two when the key's
    /// level was.
    #[tokio::test]
    async fn the_two_key_extension_reselects_the_free_key_only_when_it_was_better() {
        let key = AccessKey::new(0x0000_00AA).expect("not the sentinel");

        let free_is_better = SimulatedDevice::with_config(SimulatorConfig {
            free_access_level: 0,
            key: Some(0x0000_00AA),
            key_level: 3,
            ..SimulatorConfig::default()
        });
        let mut session = ManagementSession::read_only(
            &free_is_better,
            free_is_better.address(),
            AuthorisationPlan::WithKey(key),
            fast(),
        )
        .expect("contactable")
        .with_two_key_extension();
        session.connect().await.expect("connect");
        assert_eq!(
            free_is_better.authorize_requests(),
            3,
            "free, then the key, then the free key again"
        );
        assert_eq!(
            session.connection().expect("connected").authorisation,
            Authorisation::Granted {
                level: AccessLevel::MAXIMUM
            }
        );

        let key_is_better = SimulatedDevice::with_config(SimulatorConfig {
            free_access_level: 3,
            key: Some(0x0000_00AA),
            key_level: 0,
            ..SimulatorConfig::default()
        });
        let mut session = ManagementSession::read_only(
            &key_is_better,
            key_is_better.address(),
            AuthorisationPlan::WithKey(key),
            fast(),
        )
        .expect("contactable")
        .with_two_key_extension();
        session.connect().await.expect("connect");
        assert_eq!(
            key_is_better.authorize_requests(),
            2,
            "no reselection when the key bought something"
        );
    }

    /// AL §3.4.4.2's error handling: an access level too low to write
    /// answers `nr_of_elem = 0`, and the session says which property.
    #[tokio::test]
    async fn an_insufficient_access_level_is_a_property_refusal() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            free_access_level: 3,
            write_requires_level: 0,
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        let error = session
            .connect()
            .await
            .expect_err("Verify Mode cannot be set at this level");
        match error {
            SessionError::PropertyRefused {
                object_index,
                property_id,
            } => {
                assert_eq!(object_index, ObjectIndex::DEVICE);
                assert_eq!(property_id, PID_DEVICE_CONTROL);
            }
            other => panic!("expected a property refusal, got {other}"),
        }
    }

    // -------------------------------------------------------- the writes

    /// §6.2 branch one and §14 item 4: with Verify Mode active the device's
    /// own `A_Memory_Write.res` is the verification, so no extra read is
    /// sent — and the comparison is still made.
    #[tokio::test]
    async fn verify_mode_active_verifies_against_the_devices_own_response() {
        let device = SimulatedDevice::new();
        let mut session = writer(&device, WriteScope::Download);
        session.connect().await.expect("connect");
        assert_eq!(session.verify_mode(), Some(VerifyMode::Active));
        assert!(device.verify_mode(), "the device took bit 2");

        let data: Vec<u8> = (0..20).collect();
        let limit = write_limit(ApduLengthSource::DeviceObject(15));
        assert_eq!(limit.max_octets(), 12);
        let chunks = session
            .write_memory_region(0x4000, &data, limit)
            .await
            .expect("the region is writable");
        assert_eq!(chunks, 2, "20 octets at 12 per write");

        let stored: Vec<Option<u8>> = device.memory(0x4000, 20);
        assert_eq!(
            stored,
            data.iter().copied().map(Some).collect::<Vec<_>>(),
            "every octet must have arrived"
        );
        assert!(
            !device
                .seen()
                .iter()
                .any(|entry| matches!(entry, Seen::MemoryRead { .. })),
            "with Verify Mode active the write's own answer is the read-back"
        );
    }

    /// §6.2 branch two: a device that conformantly does not implement
    /// Verify Mode gets an explicit `A_Memory_Read` per chunk. That is this
    /// project's rule, not the Standard's, which is why it is a test.
    #[tokio::test]
    async fn verify_mode_unavailable_forces_an_explicit_read_back() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            verify_mode_supported: false,
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        session.connect().await.expect("connect");
        assert_eq!(session.verify_mode(), Some(VerifyMode::Unavailable));
        assert!(!device.verify_mode());

        let limit = write_limit(ApduLengthSource::Absent);
        session
            .write_memory_region(0x4000, &[0xAA, 0xBB, 0xCC], limit)
            .await
            .expect("the region is writable");

        let tail: Vec<Seen> = device
            .seen()
            .into_iter()
            .filter(|entry| matches!(entry, Seen::MemoryWrite { .. } | Seen::MemoryRead { .. }))
            .collect();
        assert_eq!(
            tail,
            vec![
                Seen::MemoryWrite {
                    address: 0x4000,
                    data: vec![0xAA, 0xBB, 0xCC],
                    service: MemoryService::Memory,
                },
                Seen::MemoryRead {
                    address: 0x4000,
                    number: 3,
                    service: MemoryService::Memory,
                },
            ],
            "a write with no Verify Mode must be followed by a read of the same octets"
        );
    }

    /// §2.3: a property write is checked against the scope the **caller**
    /// names, so an authorisation for one operation does not quietly cover
    /// another, and the refusal says which scope was attempted.
    #[tokio::test]
    async fn a_property_write_is_refused_when_the_scope_is_not_the_one_authorised() {
        let device = SimulatedDevice::new();
        let mut session = writer(&device, WriteScope::Download);
        session.connect().await.expect("connect");
        // `connect()` has already written Verify Mode, so the interesting
        // question is whether anything arrives *after* this point.
        let before = device.seen().len();

        let error = session
            .write_property(
                ObjectIndex::DEVICE,
                PID_DEVICE_CONTROL,
                vec![0x00],
                WriteScope::Restart,
            )
            .await
            .expect_err("a download authorisation does not cover a restart");
        assert!(
            error.to_string().contains("restart"),
            "the refusal must name the scope attempted, not the one held: {error}"
        );
        assert_eq!(
            device.seen().len(),
            before,
            "the device must have seen nothing at all"
        );
    }

    /// CP §3.5.2 picks the service on `base + length`, and the read-back has
    /// to pick the same one. A region that straddles `FFFFh` goes out as
    /// `A_UserMemory_Write` throughout — so reading it back with
    /// `A_Memory_Read`, which is what choosing the service from the address
    /// alone did, would read a different address space and compare octets
    /// nobody wrote.
    #[tokio::test]
    async fn a_region_that_straddles_ffffh_is_read_back_through_the_same_service() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            verify_mode_supported: false,
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        session.connect().await.expect("connect");

        let limit = write_limit(ApduLengthSource::Absent);
        session
            .write_memory_region(0xFFFC, &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06], limit)
            .await
            .expect("the straddling region is writable");

        let tail: Vec<Seen> = device
            .seen()
            .into_iter()
            .filter(|entry| matches!(entry, Seen::MemoryWrite { .. } | Seen::MemoryRead { .. }))
            .collect();
        assert_eq!(
            tail,
            vec![
                Seen::MemoryWrite {
                    address: 0xFFFC,
                    data: vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
                    service: MemoryService::UserMemory,
                },
                Seen::MemoryRead {
                    address: 0xFFFC,
                    number: 6,
                    service: MemoryService::UserMemory,
                },
            ],
            "both halves of the round trip must use the user-memory service"
        );
    }

    /// And the plain public read picks its service the same way: the whole
    /// point of `service_for` is that `base + length` decides, not `base`.
    #[tokio::test]
    async fn a_read_that_would_cross_ffffh_uses_the_user_memory_service() {
        let device = SimulatedDevice::new();
        let mut session = read_only(&device);
        session.connect().await.expect("connect");
        session
            .read_memory(0xFFFA, 8)
            .await
            .expect("the simulator answers either service");
        assert!(
            device.seen().into_iter().any(|entry| entry
                == Seen::MemoryRead {
                    address: 0xFFFA,
                    number: 8,
                    service: MemoryService::UserMemory,
                }),
            "a read ending above FFFFh belongs to A_UserMemory_Read"
        );
    }

    /// The read-back has to be load-bearing, or it is decoration: a device
    /// that stores something else must be caught in **both** branches.
    #[tokio::test]
    async fn a_device_that_stores_something_else_is_caught_in_both_branches() {
        for verify_mode_supported in [true, false] {
            let device = SimulatedDevice::with_config(SimulatorConfig {
                verify_mode_supported,
                corrupt_memory_writes: true,
                ..SimulatorConfig::default()
            });
            let mut session = writer(&device, WriteScope::Download);
            session.connect().await.expect("connect");
            let limit = write_limit(ApduLengthSource::Absent);
            let error = session
                .write_memory_region(0x4000, &[0x0F, 0x10], limit)
                .await
                .expect_err("a corrupted store must be caught");
            match error {
                SessionError::ReadBackMismatch {
                    address,
                    written,
                    read,
                } => {
                    assert_eq!(address, 0x4000);
                    assert_eq!(written, vec![0x0F, 0x10]);
                    assert_eq!(read, vec![0xF0, 0x10]);
                }
                other => panic!(
                    "expected a read-back mismatch (verify mode supported: \
                     {verify_mode_supported}), got {other}"
                ),
            }
        }
    }

    /// A protected region answers `number = 0`, which is a refusal and not
    /// an empty success.
    #[tokio::test]
    async fn a_protected_region_is_a_refusal_not_an_empty_success() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            protected_memory: Some((0x4000, 0x5000)),
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        session.connect().await.expect("connect");
        let limit = write_limit(ApduLengthSource::Absent);
        let error = session
            .write_memory_region(0x4000, &[0x01], limit)
            .await
            .expect_err("a protected region must refuse");
        assert!(matches!(error, SessionError::MemoryRefused { .. }));
    }

    /// §6.5 on `base + length`: a region ending above `FFFFh` goes out as
    /// `A_UserMemoryWrite` for the whole of itself, in chunks of no more
    /// than the fifteen octets that service can carry.
    #[tokio::test]
    async fn a_region_straddling_ffff_goes_out_as_user_memory_throughout() {
        let device = SimulatedDevice::new();
        let mut session = writer(&device, WriteScope::Download);
        session.connect().await.expect("connect");

        let data: Vec<u8> = (0..8).collect();
        let limit = write_limit(ApduLengthSource::DeviceObject(254));
        assert_eq!(limit.max_octets(), 63);
        session
            .write_memory_region(0xFFFC, &data, limit)
            .await
            .expect("the region is writable");

        let writes: Vec<Seen> = device
            .seen()
            .into_iter()
            .filter(|entry| matches!(entry, Seen::MemoryWrite { .. }))
            .collect();
        assert_eq!(
            writes.len(),
            1,
            "eight octets fit one A_UserMemoryWrite: {writes:?}"
        );
        assert_eq!(
            device.memory(0xFFFC, 8),
            data.iter().copied().map(Some).collect::<Vec<_>>()
        );
    }

    // ------------------------------------------------- the load machinery

    #[tokio::test]
    async fn a_load_event_is_written_and_the_resulting_state_read_back() {
        let device = SimulatedDevice::new();
        let mut session = writer(&device, WriteScope::Download);
        session.connect().await.expect("connect");
        let state = session
            .write_load_event(ObjectIndex::APPLICATION_PROGRAM, LoadEvent::StartLoading)
            .await
            .expect("Start Loading from Unloaded is Table 94's only outcome");
        assert_eq!(state, LoadState::Loading);
        assert_eq!(
            device.load_state(ObjectIndex::APPLICATION_PROGRAM),
            LoadState::Loading
        );
    }

    /// §14 item 16 and §10.8: reads work, the write is silently dropped,
    /// and the client must report the observation — not success.
    #[tokio::test]
    async fn a_silently_dropped_event_write_is_reported_as_an_unchanged_state() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            drop_load_state_writes: true,
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        session.connect().await.expect("connect");
        let error = session
            .write_load_event(ObjectIndex::APPLICATION_PROGRAM, LoadEvent::StartLoading)
            .await
            .expect_err("a dropped write must not look like a success");
        match error {
            SessionError::StateUnchanged { state, event, .. } => {
                assert_eq!(state, LoadState::Unloaded);
                assert_eq!(event, LoadEvent::StartLoading);
                let text = error.to_string();
                assert!(
                    text.contains("access level") && text.contains("not an error"),
                    "the report must list the possible causes, including the \
                     access level and the one that is not a fault at all: {text}"
                );
            }
            other => panic!("expected an unchanged state, got {other}"),
        }
    }

    /// §5.5: `LoadCompleting` is "still working", and a device that goes
    /// quiet in it is not a failure.
    #[tokio::test]
    async fn load_completing_is_waited_out_and_silence_in_it_is_tolerated() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            load_completing_polls: 3,
            ..SimulatorConfig::default()
        });
        device.preset_load_state(ObjectIndex::APPLICATION_PROGRAM, LoadState::Loading);
        let mut session = writer(&device, WriteScope::Download);
        session.connect().await.expect("connect");
        let state = session
            .write_load_event(ObjectIndex::APPLICATION_PROGRAM, LoadEvent::LoadCompleted)
            .await
            .expect("the transition settles once the checksum is done");
        assert_eq!(state, LoadState::Loaded);
    }

    /// C5, RES §4.23.2.4.1: *"...periodically during the maximum
    /// transition time and once more when the maximum transition time has
    /// passed."* A device that only settles well after the 50 ms deadline
    /// must still be heard: the pre-fix code returned `TransitionTimedOut`
    /// the instant the deadline passed and never asked again, so this
    /// device would never have been heard from.
    ///
    /// The 40 ms `poll_interval` is deliberately as large as
    /// `max_transition` itself, so the one extra attempt (fired at ~80 ms,
    /// the first poll after the deadline) lands in a window — roughly
    /// 80–120 ms — comfortably clear of both the last pre-deadline poll
    /// (~40 ms) and of any plausible scheduler jitter.
    #[tokio::test]
    async fn a_late_answer_past_the_deadline_still_succeeds() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            settle_load_state_after: Some(Duration::from_millis(100)),
            settled_load_state: LoadState::Loaded,
            ..SimulatorConfig::default()
        });
        device.preset_load_state(ObjectIndex::APPLICATION_PROGRAM, LoadState::LoadCompleting);
        let mut session = ManagementSession::read_only(
            &device,
            device.address(),
            AuthorisationPlan::Skip,
            SessionTiming {
                max_transition: Duration::from_millis(50),
                poll_interval: Duration::from_millis(40),
                ..fast()
            },
        )
        .expect("the simulated device is contactable");
        session.connect().await.expect("connect");
        let outcomes = permitted_outcomes(
            LoadState::Loading,
            Stimulus::Event(LoadEvent::LoadCompleted),
            None,
        );
        let observed = session
            .wait_for_load_state(
                ObjectIndex::APPLICATION_PROGRAM,
                LoadEvent::LoadCompleted,
                LoadState::Loading,
                &outcomes,
            )
            .await
            .expect(
                "the device settles at 100ms, past the 50ms deadline; the \
                 one-more attempt (fired at ~80ms and ~120ms) must catch it",
            );
        assert_eq!(observed, LoadState::Loaded);
    }

    /// C5: the clause's "once more" is exactly one attempt, not a second
    /// polling loop. A device that never settles must still time out in
    /// bounded time — a retry loop instead of a single extra attempt would
    /// hang here forever, which is what the bounding [`tokio::time::timeout`]
    /// below is for.
    ///
    /// A zero `max_transition` makes the deadline check fail on the very
    /// first read, deterministically, regardless of scheduler timing: read
    /// #1 is the ordinary poll that finds the deadline already passed, and
    /// read #2 is the one extra attempt the clause promises. The bounding
    /// timeout alone cannot tell one extra attempt from two — both return
    /// in single-digit milliseconds at this `poll_interval` — so the read
    /// count below is the assertion that actually pins "once", not "twice".
    #[tokio::test]
    async fn a_device_that_never_settles_gets_exactly_one_attempt_past_the_deadline() {
        let device = SimulatedDevice::new();
        device.preset_load_state(ObjectIndex::APPLICATION_PROGRAM, LoadState::LoadCompleting);
        let mut session = ManagementSession::read_only(
            &device,
            device.address(),
            AuthorisationPlan::Skip,
            SessionTiming {
                max_transition: Duration::ZERO,
                poll_interval: Duration::from_millis(1),
                ..fast()
            },
        )
        .expect("the simulated device is contactable");
        session.connect().await.expect("connect");
        let outcomes = permitted_outcomes(
            LoadState::Loading,
            Stimulus::Event(LoadEvent::LoadCompleted),
            None,
        );
        let result = tokio::time::timeout(
            Duration::from_millis(500),
            session.wait_for_load_state(
                ObjectIndex::APPLICATION_PROGRAM,
                LoadEvent::LoadCompleted,
                LoadState::Loading,
                &outcomes,
            ),
        )
        .await
        .expect(
            "a single extra attempt must give up well inside 500ms; a \
             retry loop instead of one more try would never return at all",
        );
        match result {
            Err(SessionError::TransitionTimedOut { .. }) => {}
            other => panic!("expected a transition time-out, got {other:?}"),
        }
        assert_eq!(
            device.load_state_reads(),
            2,
            "read #1 crosses the zeroed deadline, read #2 is the one \
             attempt past it; a third read would mean the retry fired twice"
        );
    }

    /// RES Table 94 has no cell that turns `Start Loading` in `Unloaded`
    /// into `Error`, so a device that does it is reported rather than
    /// believed.
    #[tokio::test]
    async fn a_state_outside_table_94_is_reported_as_an_illegal_transition() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            fail_on_event: Some(LoadEvent::StartLoading),
            error_code: 2,
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        session.connect().await.expect("connect");
        let error = session
            .write_load_event(ObjectIndex::APPLICATION_PROGRAM, LoadEvent::StartLoading)
            .await
            .expect_err("Error is not an outcome Table 94 permits here");
        match error {
            SessionError::IllegalTransition { from, observed, .. } => {
                assert_eq!(from, LoadState::Unloaded);
                assert_eq!(observed, LoadState::Error);
            }
            other => panic!("expected an illegal transition, got {other}"),
        }
    }

    /// §5.5 and §9.1 step 3: the error code has to be read *before* the
    /// unload, because leaving `Error` clears it.
    #[tokio::test]
    async fn the_error_code_survives_until_the_unload_and_not_past_it() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            fail_on_event: Some(LoadEvent::StartLoading),
            error_code: 2,
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        session.connect().await.expect("connect");
        let _ = session
            .write_load_event(ObjectIndex::APPLICATION_PROGRAM, LoadEvent::StartLoading)
            .await;

        let before_unload = session
            .read_error_code(ObjectIndex::APPLICATION_PROGRAM)
            .await
            .expect("the error code is readable while the part is in Error");

        let mut session = ManagementSession::authorised(
            &device,
            AuthorisationPlan::Skip,
            fast(),
            simulator_authorisation(&device, WriteScope::Unload),
        )
        .expect("a simulator unload is authorised");
        session.connect().await.expect("connect");
        let state = session
            .write_load_event(ObjectIndex::APPLICATION_PROGRAM, LoadEvent::Unload)
            .await
            .expect("Unload is the only escape from Error");
        assert_eq!(state, LoadState::Unloaded);

        let after_unload = session
            .read_error_code(ObjectIndex::APPLICATION_PROGRAM)
            .await
            .expect("the error code is still a readable property");
        assert_ne!(
            before_unload, after_unload,
            "the unload cleared the evidence, which is why §9.1 reads it first"
        );
    }

    /// §14 item 9 and §9.2: a zero reference is a failure in both its
    /// meanings, and neither of them ever becomes a write offset.
    #[tokio::test]
    async fn a_zero_reference_is_a_failure_in_both_of_its_meanings() {
        // Meaning one: the allocation was attempted in `Loading` and failed.
        let never_allocates = SimulatedDevice::with_config(SimulatorConfig {
            reference_always_zero: true,
            ..SimulatorConfig::default()
        });
        never_allocates.preset_load_state(ObjectIndex::APPLICATION_PROGRAM, LoadState::Loading);
        let mut session = writer(&never_allocates, WriteScope::Download);
        session.connect().await.expect("connect");
        let error = session
            .read_table_reference(ObjectIndex::APPLICATION_PROGRAM)
            .await
            .expect_err("zero is not an address");
        assert!(matches!(error, SessionError::AllocationFailed { .. }));

        // Meaning two: the allocation was attempted outside `Loading` and
        // was ignored (§7.6), which looks exactly the same from here.
        let device = SimulatedDevice::new();
        device.preset_load_state(ObjectIndex::APPLICATION_PROGRAM, LoadState::Loaded);
        let mut session = writer(&device, WriteScope::Download);
        session.connect().await.expect("connect");
        let _ = session
            .write_load_event(
                ObjectIndex::APPLICATION_PROGRAM,
                LoadEvent::AdditionalLoadControls,
            )
            .await;
        let error = session
            .read_table_reference(ObjectIndex::APPLICATION_PROGRAM)
            .await
            .expect_err("an ignored allocation leaves zero behind");
        assert!(matches!(error, SessionError::AllocationFailed { .. }));
    }

    /// §14 items 5 and 15: the connection dies mid-procedure, and the
    /// client re-authorises *and* re-asserts Verify Mode before carrying on.
    #[tokio::test]
    async fn a_dropped_connection_is_re_authorised_and_verify_mode_re_asserted() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            key: Some(0x0000_0042),
            key_level: 0,
            drop_connection_on_load_state_read: Some(2),
            ..SimulatorConfig::default()
        });
        device.preset_load_state(ObjectIndex::APPLICATION_PROGRAM, LoadState::Loading);
        let key = AccessKey::new(0x0000_0042).expect("not the sentinel");
        let mut session = ManagementSession::authorised(
            &device,
            AuthorisationPlan::WithKey(key),
            fast(),
            simulator_authorisation(&device, WriteScope::Download),
        )
        .expect("a simulator write session");
        session.connect().await.expect("connect");
        assert_eq!(device.authorize_requests(), 1);
        assert_eq!(device_control_writes(&device), 1);

        let state = session
            .write_load_event(ObjectIndex::APPLICATION_PROGRAM, LoadEvent::LoadCompleted)
            .await
            .expect("the wait loop re-establishes the connection and carries on");
        assert_eq!(state, LoadState::Loaded);
        assert_eq!(session.reconnects(), 1, "exactly one reconnect happened");
        assert_eq!(
            device.authorize_requests(),
            2,
            "authorisation lives and dies with the connection"
        );
        assert_eq!(
            device_control_writes(&device),
            2,
            "so does Verify Mode, and it must be set again"
        );
        assert_eq!(session.verify_mode(), Some(VerifyMode::Active));
        assert!(device.verify_mode());
    }

    // ------------------------------------------------- programming mode

    /// §14 item 18: bits 1–6 survive, bit 0 and bit 7 invert together, and
    /// no parity is computed from a convention this project has not
    /// established.
    #[tokio::test]
    async fn the_programming_mode_toggle_inverts_two_bits_and_preserves_six() {
        let device = SimulatedDevice::new();
        device.preset_memory(u32::from(CURR_PROG_MODE_ADDRESS), &[0b0101_1010]);
        let mut session = writer(&device, WriteScope::ProgrammingModeToggle);
        session.connect().await.expect("connect");

        let decision = session
            .set_programming_mode(true)
            .await
            .expect("the toggle is a memory write like any other");
        assert_eq!(decision, ProgModeWrite::Write(0b1101_1011));
        assert_eq!(
            device.memory(u32::from(CURR_PROG_MODE_ADDRESS), 1),
            vec![Some(0b1101_1011)]
        );
    }

    /// The guard of RES §4.26.3.4.2/§4.26.3.4.3: already in the requested
    /// mode means no frame at all, because a no-op toggle would invert the
    /// parity against an unchanged mode.
    #[tokio::test]
    async fn the_programming_mode_toggle_writes_nothing_when_the_mode_matches() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::ProgrammingModeToggle);
        session.connect().await.expect("connect");
        let decision = session
            .set_programming_mode(true)
            .await
            .expect("the read alone answers the question");
        assert_eq!(decision, ProgModeWrite::AlreadyInRequestedMode);
        assert!(
            !device.memory_was_written(),
            "a no-op toggle must not put a frame on the wire: {:?}",
            device.seen()
        );
    }

    /// An authorisation for one scope does not authorise another: the
    /// programming-mode session cannot download.
    #[tokio::test]
    async fn an_authorisation_does_not_cover_a_scope_it_was_not_given() {
        let device = SimulatedDevice::new();
        let mut session = writer(&device, WriteScope::ProgrammingModeToggle);
        session.connect().await.expect("connect");
        let limit = write_limit(ApduLengthSource::Absent);
        let error = session
            .write_memory_region(0x4000, &[0x01], limit)
            .await
            .expect_err("a programming-mode authorisation is not a download authorisation");
        assert!(matches!(error, SessionError::Refused(_)), "got {error}");
        assert!(!device.memory_was_written());
    }

    // ------------------------------------------------------------ restart

    /// MP §3.7.1.1.1/§3.7.1.1.3: Basic Restart is unconfirmed at the
    /// Application Layer, so the T_ACK `restart_basic` waits for is the
    /// whole of the confirmation, and the device sees `restart_type: 0`
    /// with no erase code or channel number attached.
    #[tokio::test]
    async fn restart_basic_sends_the_unconfirmed_request() {
        let device = SimulatedDevice::new();
        let mut session = writer(&device, WriteScope::Restart);
        session.connect().await.expect("connect");

        session
            .restart_basic()
            .await
            .expect("the Transport Layer T_ACK is Basic Restart's whole confirmation");

        assert!(
            device.seen().iter().any(|entry| matches!(
                entry,
                Seen::Restart {
                    response: false,
                    restart_type: 0,
                    data
                } if data.is_empty()
            )),
            "the device must see an unconfirmed Basic Restart request: {:?}",
            device.seen()
        );
    }

    /// MP §3.7.3 exception (5), p. 90: the mandatory 6 s wait applies only
    /// to the post-restart path. A mutation that deleted the `sleep` in
    /// `disconnect_after_restart`, or one that moved
    /// `SessionTiming::post_restart_disconnect_wait` onto plain
    /// [`ManagementSession::disconnect`], would both pass a test that only
    /// checks the *outcome* of a restart — this one checks how long it
    /// took, against both paths, so neither mutation goes unnoticed.
    #[tokio::test]
    async fn only_the_post_restart_path_waits_out_the_disconnect_timeout() {
        let timing = fast();

        let restarted = SimulatedDevice::new();
        let mut restarting = writer(&restarted, WriteScope::Restart);
        restarting.connect().await.expect("connect");
        let restart_started = tokio::time::Instant::now();
        restarting.restart_basic().await.expect("basic restart");
        let restart_elapsed = restart_started.elapsed();

        let plain = SimulatedDevice::new();
        let mut plain_session = writer(&plain, WriteScope::Restart);
        plain_session.connect().await.expect("connect");
        let disconnect_started = tokio::time::Instant::now();
        plain_session.disconnect().await;
        let disconnect_elapsed = disconnect_started.elapsed();

        assert!(
            restart_elapsed >= timing.post_restart_disconnect_wait,
            "MP §3.7.3 exception (5), p. 90: restart_basic must not return \
             before the post-restart wait fully elapses, took {restart_elapsed:?}"
        );
        assert!(
            disconnect_elapsed < timing.post_restart_disconnect_wait,
            "plain disconnect() must stay best-effort and unwaited, took {disconnect_elapsed:?}"
        );
    }

    /// MP §3.7.1.2.1: Master Reset carries an Erase Code and a Channel
    /// Number the device must see, and returns whatever
    /// `A_Restart_Response` the device answered with — here, the simulator's
    /// configured Error Code and Process Time, DPT 7.005, both round-tripped
    /// exactly rather than rounded.
    #[tokio::test]
    async fn restart_master_reset_carries_erase_code_and_decodes_the_response() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            restart_error_code: 0,
            restart_process_time: Duration::from_secs(9),
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Restart);
        session.connect().await.expect("connect");

        let response = session
            .restart_master_reset(ERASE_CODE_CONFIRMED_RESTART, 3)
            .await
            .expect("the simulator answers a Master Reset request");

        assert_eq!(response.error_code, 0);
        assert_eq!(
            response.process_time,
            Duration::from_secs(9),
            "MP §3.7.1.2.2, pp. 80-81: Process Time must survive exactly, not rounded"
        );
        assert!(
            device.seen().iter().any(|entry| matches!(
                entry,
                Seen::Restart {
                    response: false,
                    restart_type: 1,
                    data
                } if data.as_slice() == [ERASE_CODE_CONFIRMED_RESTART, 3]
            )),
            "the device must see the Erase Code and Channel Number: {:?}",
            device.seen()
        );
    }

    /// MP §3.7.1.2.2, p. 81: *"the process time is thus a minimal time for
    /// the MaC to wait, not a maximal time"* — a device that reports less
    /// than the standard floor does not get to shorten the wait, and one
    /// that reports more is believed.
    #[test]
    fn recovery_wait_never_goes_below_the_configured_floor() {
        let timing = fast();

        let generous = MasterResetResponse {
            error_code: 0,
            process_time: Duration::from_secs(30),
        };
        assert_eq!(generous.recovery_wait(&timing), Duration::from_secs(30));

        let terse = MasterResetResponse {
            error_code: 0,
            process_time: Duration::ZERO,
        };
        assert_eq!(
            terse.recovery_wait(&timing),
            timing.restart_responsive_again
        );
    }

    /// AL §3.3.7 Figure 41: exactly three octets or the response is
    /// malformed, not silently truncated or zero-padded.
    #[test]
    fn decode_master_reset_response_rejects_anything_but_three_octets() {
        assert!(matches!(
            decode_master_reset_response(&[0x00, 0x00]),
            Err(SessionError::MalformedRestartResponse { got: 2 })
        ));
        assert!(matches!(
            decode_master_reset_response(&[0x00, 0x00, 0x05, 0x00]),
            Err(SessionError::MalformedRestartResponse { got: 4 })
        ));
        assert_eq!(
            decode_master_reset_response(&[0x00, 0x00, 0x05]).expect("three octets decode"),
            MasterResetResponse {
                error_code: 0,
                process_time: Duration::from_secs(5),
            }
        );
    }
}
