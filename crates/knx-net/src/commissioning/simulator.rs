//! A device simulator that answers management services, including badly, so no test needs a bus.
//!
//! Spec §11.3 asks for *"a simulator that can be told to misbehave"* and
//! lists the ways: silence, a failure at a named step, no protected areas,
//! load-state writes that are read but ignored, a reference that never
//! allocates, an absent Verify Mode, a connection that dies mid-download.
//! Each of those is a field of [`SimulatorConfig`] and each has a test.
//!
//! What this is not: a conformant KNX device. It answers the subset of the
//! Application Layer the download procedures of design spec §7 use, with the
//! encodings those clauses cite, and it answers nothing else. Its purpose is
//! to make the client's behaviour observable — in particular the behaviour
//! that consists of *not* sending something.
//!
//! Two deliberate asymmetries with a real bus, both recorded in
//! `KNOWN_LIMITATIONS.md`: services are handed over as decoded values rather
//! than octets, so nothing here exercises the cEMI codec, and every answer
//! is immediate, so nothing here exercises real timing. A simulator proves
//! the client sends what the Standard says a client sends; it proves nothing
//! whatsoever about what a physical device does with it.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use knx_core::commissioning::load_control::LoadControlSubtype;
use knx_core::commissioning::load_state::{LoadEvent, LoadState};
use knx_core::commissioning::memory::MemoryService;
use knx_core::commissioning::mutation::TargetKind;
use knx_core::commissioning::properties::{
    verify_mode_active, ObjectIndex, PID_DEVICE_CONTROL, PID_DOWNLOAD_COUNTER, PID_ERROR_CODE,
    PID_LOAD_STATE_CONTROL, PID_MANUFACTURER_ID, PID_MAX_APDU_LENGTH, PID_MCB_TABLE,
    PID_PROGRAM_VERSION, PID_TABLE_REFERENCE,
};
use knx_core::{GroupValue, IndividualAddress};
use tokio::sync::broadcast;

use super::ERASE_CODE_CONFIRMED_RESTART;
use crate::cemi::{
    ApplicationService, Destination, LDataFrame, LDataMessageKind, Tpci, BROADCAST_DESTINATION,
};
use crate::client::{BusError, TunnelEvent};
use crate::management::ManagementTransport;

/// The address the simulated management client appears to have.
///
/// Inside the approved read range of design spec §2.2 rather than outside it, so
/// that a fixture address never looks like an invitation to try it on a bus.
pub const SIMULATED_CLIENT_ADDRESS: (u8, u8, u8) = (1, 1, 24);

/// The address the simulated device appears to have.
pub const SIMULATED_DEVICE_ADDRESS: (u8, u8, u8) = (1, 1, 25);

/// Where the simulator's memory hands out allocations, when it allocates.
const ALLOCATION_BASE: u32 = 0x4000;

/// The Memory Control Block octets an object answers when nothing has put
/// others there.
///
/// Laid out per RES §4.2.27, Table 12, p. 39: Segment Size 1 `0x00001000`,
/// CRC Control Byte `0xAA` (bit 0 clear — "CRC is always valid", RES
/// §4.2.27.1.1, Table 13, p. 39, the default a "device that works" gives),
/// Read Access 1 / Write Access 1 nibbles `0xC`/`0xD`, CRC `0x0000`.
const DEFAULT_MCB: [u8; 8] = [0x00, 0x00, 0x10, 0x00, 0xAA, 0xCD, 0x00, 0x00];

/// Every `Additional Load Control` subtype, so the simulator can tell an
/// allocation from a task record without a second table of octets.
const ALL_SUBTYPES: [LoadControlSubtype; 8] = [
    LoadControlSubtype::AllocAbsDataSeg,
    LoadControlSubtype::AllocAbsStackSeg,
    LoadControlSubtype::AllocAbsTaskSeg,
    LoadControlSubtype::TaskPtr,
    LoadControlSubtype::TaskCtrl1,
    LoadControlSubtype::TaskCtrl2,
    LoadControlSubtype::RelativeAllocation,
    LoadControlSubtype::DataRelativeAllocation,
];

/// The inverse of [`LoadEvent::octet`], which the domain crate has no use
/// for: only a device decodes an event, and only this one is a device.
fn load_event_from_octet(octet: u8) -> Option<LoadEvent> {
    LoadEvent::ALL
        .into_iter()
        .find(|event| event.octet() == octet)
}

/// Whether `service` is a read of `wanted` and `number` falls inside
/// `window`.
///
/// `number` is the caller's own transmission counter, already incremented
/// for this frame, and it counts every retransmission separately — so a
/// window `MAX_TRANSMISSIONS` wide is exactly one of the client's ladders
/// and not that many requests.
fn numbered_read_in(
    service: &ApplicationService,
    wanted: u8,
    number: u32,
    window: &Option<Range<u32>>,
) -> bool {
    let Some(window) = window else {
        return false;
    };
    let is_wanted_read = matches!(
        service,
        ApplicationService::PropertyValueRead { property_id, .. } if *property_id == wanted
    );
    is_wanted_read && window.contains(&number)
}

/// How the simulated device is to misbehave.
///
/// [`Default`] is a device that works: everything else in this struct is a
/// way of not working, and a test says which one it wants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatorConfig {
    /// Answer nothing at all. Spec §11.3's first failure mode and §9.2's
    /// first row: the client must report a time-out and must not report a
    /// cause it did not observe.
    pub silent: bool,
    /// Drop this many `T_DATA_CONNECTED` transmissions — no `T_ACK`, no
    /// answer, as if lost in transit — before answering normally.
    ///
    /// The narrow, temporary cousin of [`SimulatorConfig::silent`], for
    /// C4's TL §3, p. 15 retry count: a permanently silent device proves
    /// the client eventually gives up, this one proves the client's retry
    /// still gets an answer through on a transmission that would have been
    /// one too many for the pre-fix cap.
    pub silent_for_first_numbered_data_frames: u32,
    /// Accept `PID_DEVICE_CONTROL` bit 2 and honour it. When false the
    /// device keeps the bit clear, which is what PROF footnote 8 requires
    /// of a device that does not implement Verify Mode.
    pub verify_mode_supported: bool,
    /// Read `PID_LOAD_STATE_CONTROL` happily, ignore every write to it.
    /// Spec §9.2 row 3: the state is unchanged and the causes are several.
    pub drop_load_state_writes: bool,
    /// Answer `PID_TABLE_REFERENCE` = 0 whatever is allocated. `[D]`
    /// CP §3.5.2's failure value.
    pub reference_always_zero: bool,
    /// Break the connection down once, after this many frames.
    pub drop_connection_after: Option<usize>,
    /// Break the connection down once, on the *n*-th read of
    /// `PID_LOAD_STATE_CONTROL`, counting from one.
    ///
    /// The deterministic version of [`SimulatorConfig::drop_connection_after`]
    /// for §14 item 15's *"drops the connection mid-download"*: a frame
    /// count depends on how many frames the client happens to send, and a
    /// test should not have to know that.
    pub drop_connection_on_load_state_read: Option<u32>,
    /// Enter `Error` when this event is written, and set
    /// `PID_ERROR_CODE` to `error_code`.
    pub fail_on_event: Option<LoadEvent>,
    /// The `DPT_ErrorClass_System` octet to report once in `Error`.
    pub error_code: u8,
    /// How many reads answer `LoadCompleting` before the state settles.
    pub load_completing_polls: u8,
    /// Answer nothing while in `LoadCompleting`, which RES Table 94's
    /// footnote permits and design spec §5.5 requires the client to tolerate.
    ///
    /// The T_ACK still goes out: this device is busy at the application
    /// layer and alive at the Transport Layer, which is RES §4.23.2.4.1's
    /// *"If the MaS responds during state LoadCompleting, an established
    /// TL-connection is kept alive"*.
    pub silent_in_load_completing: bool,
    /// Withhold the *answer* to the `PID_LOAD_STATE_CONTROL` reads numbered
    /// in this half-open range, counting every transmission separately and
    /// from one. The T_ACK still goes out.
    ///
    /// The device this models is busy, not gone: RES §4.23.2.4.1's *"If the
    /// MaS responds during state LoadCompleting, an established
    /// TL-connection is kept alive"*. Numbered rather than state-gated so
    /// that a test can say which poll goes quiet without depending on how
    /// long anything took.
    pub unanswered_load_state_reads: Option<Range<u32>>,
    /// Acknowledge *nothing* for the `PID_LOAD_STATE_CONTROL` reads numbered
    /// in this half-open range — no T_ACK, no answer — counted the same way.
    ///
    /// The device this models is offline, which RES §4.23.2.4.1's NOTE 86
    /// expects: *"A device may be offline during state LoadCompleting. A
    /// running TL-connection may be lost..."* A window four wide is one
    /// whole `MAX_TRANSMISSIONS` ladder, so the client's repetitions run out
    /// and TL's action A6 has to release the connection.
    pub unacknowledged_load_state_reads: Option<Range<u32>>,
    /// Withhold the *answer* to the `PID_DEVICE_CONTROL` reads numbered in
    /// this half-open range, counting every transmission separately and from
    /// one. The T_ACK still goes out.
    ///
    /// The read this addresses is the first frame of
    /// [`ManagementSession::connect`]'s Verify Mode step, which runs *after*
    /// the `T_Connect` confirmation has already put a connection in the
    /// session's hands. A device that acknowledges it and withholds the
    /// answer is the same busy-not-gone device
    /// [`SimulatorConfig::unanswered_load_state_reads`] models, met at the
    /// one moment where the client holds a connection it has not finished
    /// building.
    ///
    /// [`ManagementSession::connect`]: crate::commissioning::ManagementSession::connect
    pub unanswered_device_control_reads: Option<Range<u32>>,
    /// Break the connection down once, on the *n*-th read of
    /// `PID_DEVICE_CONTROL`, counting from one.
    ///
    /// The `PID_DEVICE_CONTROL` twin of
    /// [`SimulatorConfig::drop_connection_on_load_state_read`], and the only
    /// way to make a *re-establishment* — rather than a poll — end in
    /// [`SessionError::ConnectionLost`]: the frame arrives inside
    /// `connect()`, so the error comes back from the re-establishment
    /// itself.
    ///
    /// [`SessionError::ConnectionLost`]: crate::commissioning::SessionError::ConnectionLost
    pub drop_connection_on_device_control_read: Option<u32>,
    /// Answer the `T_Connect` frames numbered in this half-open range with a
    /// *negative* `L_Data.con`, counting from one — no recorded
    /// [`Seen::Connect`] and no connection, because a frame the Data Link
    /// Layer reports as unacknowledged never reached the device.
    ///
    /// `[D]` TL §3.7 sends `T_Connect` with `ack_request` set, so the
    /// confirmation carries whether the addressed device acknowledged it at
    /// layer 2. A device that is absent, protected or simply busy on the
    /// segment produces the negative one, and the client reports it as
    /// [`SessionError::ConnectRejected`].
    ///
    /// [`SessionError::ConnectRejected`]: crate::commissioning::SessionError::ConnectRejected
    pub rejected_connects: Option<Range<u32>>,
    /// Answer nothing at all to the `T_Connect` frames numbered in this
    /// half-open range, counting from one — no `L_Data.con`, no recorded
    /// [`Seen::Connect`], no connection.
    ///
    /// The device this models is the one RES §4.23.2.4.1's NOTE 86 has in
    /// mind for the whole of its *"periodically"* clause: *"A device may be
    /// offline during state LoadCompleting."* An offline device does not
    /// answer a re-establishment attempt either, so a client that treats one
    /// refused attempt as fatal never waits out the transition at all.
    pub unanswered_connects: Option<Range<u32>>,
    /// `PID_MAX_APDU_LENGTH` of the Device Object, if it has one.
    pub max_apdu_length: Option<u16>,
    /// `PID_MAX_APDU_LENGTH` of the Router Object, if it has one. Spec
    /// §6.4: finding it only here means 12, not the value read.
    pub router_max_apdu_length: Option<u16>,
    /// The level `FFFFFFFFh` earns. 0 is *"no protected areas"*.
    pub free_access_level: u8,
    /// The key that earns [`SimulatorConfig::key_level`].
    pub key: Option<u32>,
    /// The level the key earns.
    pub key_level: u8,
    /// The worst level still allowed to write. A session at a numerically
    /// higher level is refused with `nr_of_elem = 0`, which AL §3.4.4.2,
    /// p. 66, gives as the answer for insufficient access rights.
    pub write_requires_level: u8,
    /// A memory region that refuses every write with `number = 0`.
    pub protected_memory: Option<(u32, u32)>,
    /// Store every memory write with its first octet inverted.
    ///
    /// Not a documented device behaviour and not claimed to be one: it is
    /// the only way to prove that the client's verification read is doing
    /// something, which is §14 item 4's whole point. A device that quietly
    /// stored something else is the failure the read-back exists to catch.
    pub corrupt_memory_writes: bool,
    /// The octet at `0060h`, the programming-mode byte.
    ///
    /// This is also what `NM_IndividualAddress_Write` step 2 and step 3
    /// need from this device: MP §2.3, p. 14's broadcast
    /// `A_IndividualAddress_Read` gets an `A_IndividualAddress_Response`
    /// from this device only while this is set, and AL §3.2.2, p. 18's
    /// broadcast `A_IndividualAddress_Write` renames this device only
    /// while this is set too — *"The application process shall ignore the
    /// A_IndividualAddress_Write.ind primitive if the device is not in
    /// ´programming mode´."* There is no second flag for the broadcast
    /// behaviour: one device, one notion of whether its programming button
    /// is pressed.
    pub programming_mode: bool,
    /// Other addresses that answer a broadcast `A_IndividualAddress_Read`
    /// alongside this device, as if they too were in programming mode.
    ///
    /// MP §2.3 step 2, p. 14: *"if more than one response is received ⇒
    /// more than one device in Programming Mode"* — a rule that only means
    /// anything once a test can put more than one responder on the bus.
    /// This simulator is one [`SimulatedDevice`], not a bus, so extra
    /// responders are named here rather than built as separate devices;
    /// none of them answer anything but this one broadcast service, and
    /// none of them can be connected to.
    pub other_programming_mode_devices: Vec<IndividualAddress>,
    /// One telegram from an unrelated device, emitted inside the
    /// `A_IndividualAddress_Read` window, plus its `L_Data.con` echo.
    ///
    /// A live installation does not fall silent while a Management Client
    /// counts, and MP §2.3 step 2, p. 14 counts one thing only:
    /// *"A_IndividualAddress_Response-PDU"*. Without traffic to ignore,
    /// [`ManagementSession::broadcast_individual_address_read`]'s frame
    /// filter is untestable and a client that counted every frame would
    /// pass — then abort on the first busy bus it met.
    ///
    /// [`ManagementSession::broadcast_individual_address_read`]: crate::commissioning::ManagementSession::broadcast_individual_address_read
    pub unrelated_traffic_during_broadcast: Option<IndividualAddress>,
    /// Answer a `DeviceDescriptorRead { descriptor_type: 0 }` with a
    /// `T_Disconnect` instead of the usual `DeviceDescriptorResponse`.
    ///
    /// MP §2.3, p. 15, "to 1.": *"If an A_Disconnect-PDU is received
    /// instead of an A_DeviceDescriptor_Response-PDU, than a device with
    /// this Individual Address exists but it may either already have
    /// another Transport Layer connection open and not accept any further
    /// Transport Layer connections, or does not support connection
    /// oriented communication mode."* `NM_IndividualAddress_Write` step 1
    /// reads it as `Occupancy::OccupiedAfterDisconnect` and carries on —
    /// *"The Management Client shall continue with the Management
    /// Procedure in every case"* (same clause; `KNOWN_LIMITATIONS.md`
    /// §108).
    pub device_descriptor_read_gets_disconnect: bool,
    /// Acknowledge a `DeviceDescriptorRead { descriptor_type: 0 }` at the
    /// Transport Layer and then never answer it.
    ///
    /// The other half of MP §2.3 step 1's silence, and the opposite
    /// verdict from [`Self::device_descriptor_read_gets_disconnect`]:
    /// p. 14, *"If no A_DeviceDescriptor_Response-PDU is received after
    /// time-out ⇒ IA_new is not occupied"*. The `T_ACK` is the point — it
    /// stops the Transport Layer from releasing the connection, so the
    /// client is left holding an open connection with nothing on it, which
    /// is the only shape in which step 4's `Abort the connection of the
    /// client side Transport Layer` (p. 15) can be observed being skipped.
    pub device_descriptor_read_unanswered: bool,
    /// The mask version Device Descriptor Type 0 answers. `07B0h` by
    /// default: a System B mask, which is the profile design spec §7's CP §3.5.2
    /// walkthrough covers.
    pub mask_version: u16,
    /// Answer `PID_TABLE_REFERENCE` = 0 for this one object the first time
    /// an allocation is attempted for it, and allocate normally afterwards.
    ///
    /// The narrow version of [`SimulatorConfig::reference_always_zero`],
    /// for §14 item 10: the escalation of CP §3.5.3 has to be able to
    /// *succeed*, so the failure it escalates from must be a one-off.
    pub allocation_fails_once_for: Option<u8>,
    /// Break the connection down once, at a named step of the §7.2 inner
    /// loop.
    ///
    /// This is design spec §11.3's *"must be able to be told to fail at a chosen
    /// step"*, and it is what §14 item 8's interruption at every step is
    /// built on. A one-shot, like every other drop here, so that the
    /// recovery procedure has a device to recover.
    pub interrupt_at: Option<Interruption>,
    /// Object indices that are Application Program 1 or Application
    /// Program 2 — the only objects RES gives `PID_PROGRAM_VERSION`.
    ///
    /// `[C1]` RES Table 90, p. 288 and Table 91, p. 290 list the property
    /// for those two objects. RES Table 77, p. 238 (Group Address Table),
    /// Table 80, p. 249 (Association Table) and Table 85, p. 270 (Group
    /// Object Table) do not. Which object index is which Interface Object
    /// is product data (design spec §3.2), not something this simulator can
    /// infer, so a test says which of its indices are the two application
    /// programs and every other index refuses the write the way `[D]`
    /// AL §3.4.4.2, p. 66, says an unlisted property is refused.
    pub application_program_objects: HashSet<u8>,
    /// `PID_DOWNLOAD_COUNTER` of the Device Object, if this simulated device
    /// has one at all. `None` — the default — models a device that omits
    /// it, which Volume 6 Profiles Annex A permits (`[C13]`, `[C18]`): RES
    /// §4.2.30.1, p. 41, only requires the property "if the device has any
    /// Download Counter", and the annex's Device Object tables (pp.
    /// 138-140) list PID 30 for none of them.
    pub download_counter: Option<u16>,
    /// Break the connection down once, on the read of
    /// `PID_DOWNLOAD_COUNTER` — [`Downloader::partial_download`] issues
    /// exactly one, so unlike the load-state and device-control triggers
    /// this needs no count.
    ///
    /// C13 fix round 1, finding 3: proves that a disconnect or time-out
    /// mid-read comes back as whatever [`SessionError`] it actually is —
    /// here [`SessionError::ConnectionLost`] — and is never folded into
    /// [`DownloadCounterCheck::Absent`], which is reserved for
    /// `SessionError::PropertyRefused` alone.
    ///
    /// [`Downloader::partial_download`]: crate::commissioning::download::Downloader::partial_download
    /// [`DownloadCounterCheck::Absent`]: crate::commissioning::download::DownloadCounterCheck::Absent
    pub drop_connection_on_download_counter_read: bool,
    /// Once this much wall-clock time has passed since the *first*
    /// `PID_LOAD_STATE_CONTROL` read arrived, later reads of it answer
    /// [`SimulatorConfig::settled_load_state`] instead of whatever
    /// [`SimulatedDevice::preset_load_state`] put there. Measured from the
    /// first read rather than from device construction, so a test's outcome
    /// does not depend on the gap between building the device and starting
    /// to poll it.
    ///
    /// C5's deterministic way of building a device that only settles once
    /// `max_transition` is already spent: a test sets this comfortably past
    /// the session's `max_transition` and can then tell, from whether
    /// `wait_for_load_state` ever sees the settled value, whether its
    /// RES §4.23.2.4.1 "once more" attempt actually happens.
    pub settle_load_state_after: Option<Duration>,
    /// The state [`SimulatorConfig::settle_load_state_after`] settles to.
    /// Meaningless while that field is `None`.
    pub settled_load_state: LoadState,
    /// `error_code` on the `A_Restart_Response` this device answers a
    /// Master Reset request with. 0 is *"no error"*; the simulator does not
    /// invent a table of the others, since MP §3.7.1.2.2 leaves the value
    /// to the responding device.
    pub restart_error_code: u8,
    /// `process_time` on the same response, `DPT_TimePeriodSec` (MP
    /// §3.7.1.2.2, pp. 80-81): *"a minimal time for the MaC to wait, not a
    /// maximal time"*. `Duration::ZERO` by default so a test that does not
    /// care about the wait does not pay for one.
    pub restart_process_time: Duration,
    /// Answer a Master Reset with a two-octet `A_Restart_Response` instead
    /// of the required three — no device does this on purpose, but
    /// [`decode_master_reset_response`] must reject whatever a device
    /// actually sends, and a test needs a device that sends something
    /// wrong to prove it.
    ///
    /// [`decode_master_reset_response`]: super::decode_master_reset_response
    pub restart_response_malformed: bool,
    /// Acknowledge nothing for a Restart request — no `T_ACK`, no
    /// `A_Restart_Response` — so `exchange()` runs out TL clause 4's own
    /// retries on this one request. Narrower than
    /// [`SimulatorConfig::silent`], which would also silence the Verify
    /// Mode exchange `connect()` runs first and so never let a session
    /// reach the restart call at all.
    pub restart_unanswered: bool,
}

/// The step of the §7.2 inner loop a simulated interruption strikes at.
///
/// Named by the service that carries the step rather than by a step number,
/// because the device sees services and not procedures — and because a
/// count of frames is not reproducible when the client's own framing
/// changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Interruption {
    /// Step 1: the `Start Loading` event write.
    StartLoading,
    /// Step 2: the `Additional Load Controls` allocation write.
    Allocate,
    /// Step 3: the `PID_TABLE_REFERENCE` read-back.
    ReferenceRead,
    /// Step 4: a memory write carrying the data.
    DataWrite,
    /// Step 5: the `PID_PROGRAM_VERSION` write.
    VersionWrite,
    /// Step 6: the `Load Completed` event write.
    LoadCompleted,
    /// Step 7: the `PID_MCB_TABLE` read that stores the CRC.
    ChecksumRead,
}

impl Interruption {
    /// Every interruption point, so a test can walk all of them rather than
    /// list the ones somebody remembered.
    pub const ALL: [Interruption; 7] = [
        Interruption::StartLoading,
        Interruption::Allocate,
        Interruption::ReferenceRead,
        Interruption::DataWrite,
        Interruption::VersionWrite,
        Interruption::LoadCompleted,
        Interruption::ChecksumRead,
    ];

    /// Whether this service is the step this interruption strikes at.
    fn strikes(self, service: &ApplicationService) -> bool {
        let load_event = |data: &[u8]| data.first().copied().and_then(load_event_from_octet);
        match (self, service) {
            (
                Interruption::StartLoading,
                ApplicationService::PropertyValueWrite {
                    property_id: PID_LOAD_STATE_CONTROL,
                    data,
                    ..
                },
            ) => load_event(data) == Some(LoadEvent::StartLoading),
            (
                Interruption::Allocate,
                ApplicationService::PropertyValueWrite {
                    property_id: PID_LOAD_STATE_CONTROL,
                    data,
                    ..
                },
            ) => load_event(data) == Some(LoadEvent::AdditionalLoadControls),
            (
                Interruption::LoadCompleted,
                ApplicationService::PropertyValueWrite {
                    property_id: PID_LOAD_STATE_CONTROL,
                    data,
                    ..
                },
            ) => load_event(data) == Some(LoadEvent::LoadCompleted),
            (
                Interruption::VersionWrite,
                ApplicationService::PropertyValueWrite {
                    property_id: PID_PROGRAM_VERSION,
                    ..
                },
            ) => true,
            (
                Interruption::ReferenceRead,
                ApplicationService::PropertyValueRead {
                    property_id: PID_TABLE_REFERENCE,
                    ..
                },
            ) => true,
            (
                Interruption::ChecksumRead,
                ApplicationService::PropertyValueRead {
                    property_id: PID_MCB_TABLE,
                    ..
                },
            ) => true,
            (
                Interruption::DataWrite,
                ApplicationService::MemoryWrite { .. } | ApplicationService::UserMemoryWrite { .. },
            ) => true,
            _ => false,
        }
    }
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            silent: false,
            silent_for_first_numbered_data_frames: 0,
            verify_mode_supported: true,
            drop_load_state_writes: false,
            reference_always_zero: false,
            drop_connection_after: None,
            drop_connection_on_load_state_read: None,
            fail_on_event: None,
            error_code: 2,
            load_completing_polls: 0,
            silent_in_load_completing: false,
            unanswered_load_state_reads: None,
            unacknowledged_load_state_reads: None,
            unanswered_device_control_reads: None,
            drop_connection_on_device_control_read: None,
            rejected_connects: None,
            unanswered_connects: None,
            max_apdu_length: Some(15),
            router_max_apdu_length: None,
            free_access_level: 0,
            key: None,
            key_level: 0,
            write_requires_level: 15,
            protected_memory: None,
            corrupt_memory_writes: false,
            programming_mode: false,
            other_programming_mode_devices: Vec::new(),
            unrelated_traffic_during_broadcast: None,
            device_descriptor_read_gets_disconnect: false,
            device_descriptor_read_unanswered: false,
            mask_version: 0x07B0,
            allocation_fails_once_for: None,
            interrupt_at: None,
            application_program_objects: HashSet::new(),
            download_counter: None,
            drop_connection_on_download_counter_read: false,
            settle_load_state_after: None,
            settled_load_state: LoadState::Unloaded,
            restart_error_code: 0,
            restart_process_time: Duration::ZERO,
            restart_response_malformed: false,
            restart_unanswered: false,
        }
    }
}

/// One thing the simulator was asked to do, in order, for a test to assert
/// on — including the ones it refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seen {
    /// A `T_Connect`.
    Connect,
    /// A `T_Disconnect`.
    Disconnect,
    /// A broadcast `A_IndividualAddress_Read` (MP §2.3 steps 2 and 3): kept
    /// as its own count so a test can prove the mandatory re-verification
    /// immediately before the write actually sent a second one, rather
    /// than trusting the write to have happened at all.
    IndividualAddressReadBroadcast,
    /// An `A_Authorize_Request`, and the key it carried.
    Authorize(u32),
    /// A property read.
    PropertyRead {
        /// The object.
        object_index: u8,
        /// The property.
        property_id: u8,
    },
    /// A property write.
    PropertyWrite {
        /// The object.
        object_index: u8,
        /// The property.
        property_id: u8,
        /// The octets.
        data: Vec<u8>,
    },
    /// A memory read.
    MemoryRead {
        /// Where.
        address: u32,
        /// How many octets.
        number: u8,
        /// Which of CP §3.5.2's two services carried it. Recorded because
        /// reading a region back through the other one reads a different
        /// address space, and a test cannot see that in the octets.
        service: MemoryService,
    },
    /// A memory write. The test that matters most asserts this list is
    /// empty.
    MemoryWrite {
        /// Where.
        address: u32,
        /// The octets.
        data: Vec<u8>,
        /// Which of CP §3.5.2's two services carried it.
        service: MemoryService,
    },
    /// An `A_Restart`, request or response, in whichever shape it arrived —
    /// `response` and `restart_type` are cemi.rs's own inline bits, kept
    /// uninterpreted here for the same reason `cemi.rs` keeps `data`
    /// uninterpreted: telling a Basic Restart from a Master Reset, or a
    /// request from a response, is a test's job, not the log's.
    Restart {
        /// Set on an `A_Restart_Response`, clear on an `A_Restart` request.
        response: bool,
        /// 0 = Basic Restart, 1 = Master Reset (`cemi.rs`'s
        /// `InvalidRestartType` already refuses anything else).
        restart_type: u8,
        /// Empty for a Basic Restart request; `[erase_code,
        /// channel_number]` for a Master Reset request; `[error_code,
        /// process_time_hi, process_time_lo]` for a Master Reset response
        /// (MP §3.7.1.2.2, p. 81).
        data: Vec<u8>,
    },
    /// Anything else, by name.
    Other(&'static str),
}

#[derive(Debug)]
struct State {
    /// Where the simulated device currently lives. Starts at
    /// [`SIMULATED_DEVICE_ADDRESS`] and only ever moves in response to a
    /// broadcast `A_IndividualAddress_Write` accepted while
    /// [`SimulatorConfig::programming_mode`] is set — MP §2.3 step 3's
    /// whole point. Kept in `State`, not a plain field on
    /// [`SimulatedDevice`], because a rename is the one thing about this
    /// device that a test needs to observe *changing* mid-run: step 4
    /// reconnects to the new address and must reach this same device.
    address: IndividualAddress,
    /// When the first `PID_LOAD_STATE_CONTROL` read arrived, for
    /// [`SimulatorConfig::settle_load_state_after`] to measure against.
    /// Measuring from device construction instead would make a test's
    /// outcome depend on the gap between building the device and its first
    /// poll — invisible and untested.
    first_load_state_read: Option<Instant>,
    connected: bool,
    /// Whether the one connection drop the configuration asks for has
    /// happened. Once, not on every frame afterwards: a connection that
    /// dies again immediately is a different failure mode, and an
    /// unreachable device is already [`SimulatorConfig::silent`].
    dropped: bool,
    /// How many reads of `PID_LOAD_STATE_CONTROL` have arrived.
    load_state_reads: u32,
    /// How many reads of `PID_DEVICE_CONTROL` have arrived, counted the same
    /// way: one per transmission, not one per connection.
    device_control_reads: u32,
    /// How many `T_Connect` frames have arrived, answered or not — what
    /// [`SimulatorConfig::unanswered_connects`] counts against.
    connects: u32,
    /// How many `T_DATA_CONNECTED` transmissions have arrived, counting
    /// every retransmission of the same sequence number — what
    /// [`SimulatorConfig::silent_for_first_numbered_data_frames`] counts
    /// against.
    numbered_data_frames: u32,
    level: u8,
    verify_mode: bool,
    send_seq: u8,
    frames: usize,
    load: HashMap<u8, LoadState>,
    completing: HashMap<u8, u8>,
    error_code: HashMap<u8, u8>,
    reference: HashMap<u8, u32>,
    /// How many allocations have been attempted per object, so that
    /// [`SimulatorConfig::allocation_fails_once_for`] can fail exactly the
    /// first one.
    allocation_attempts: HashMap<u8, u32>,
    /// The Memory Control Block per object, whose CRC octets design spec §7.2 step
    /// 7 stores and §7.4 compares.
    mcb: HashMap<u8, Vec<u8>>,
    properties: HashMap<(u8, u8), Vec<u8>>,
    memory: HashMap<u32, u8>,
    seen: Vec<Seen>,
}

/// A device that answers management services, wrongly on request.
pub struct SimulatedDevice {
    client: IndividualAddress,
    config: SimulatorConfig,
    events: broadcast::Sender<TunnelEvent>,
    state: Mutex<State>,
}

impl SimulatedDevice {
    /// A working device at [`SIMULATED_DEVICE_ADDRESS`].
    pub fn new() -> Self {
        Self::with_config(SimulatorConfig::default())
    }

    /// A device configured to misbehave in the ways `config` names.
    pub fn with_config(config: SimulatorConfig) -> Self {
        let (area, line, device) = SIMULATED_DEVICE_ADDRESS;
        let address = IndividualAddress::new(area, line, device)
            .expect("the simulated device address must be a valid individual address");
        let (area, line, device) = SIMULATED_CLIENT_ADDRESS;
        let client = IndividualAddress::new(area, line, device)
            .expect("the simulated client address must be a valid individual address");
        let (events, _) = broadcast::channel(256);

        let mut properties: HashMap<(u8, u8), Vec<u8>> = HashMap::new();
        properties.insert((0, PID_DEVICE_CONTROL), vec![0x00]);
        properties.insert((0, PID_MANUFACTURER_ID), vec![0x00, 0x02]);
        // `[C1]` F10: seeded only when a test has told this device that
        // object 0 is one of the two application programs, so a read of
        // `PID_PROGRAM_VERSION` and `property_write`'s refusal of the same
        // pair (below) agree on whether object 0 carries the property —
        // RES Table 90, p. 288, and Table 91, p. 290, are what decide that,
        // not this constructor.
        if config.application_program_objects.contains(&0) {
            properties.insert((0, PID_PROGRAM_VERSION), vec![0x00, 0x02, 0x12, 0x34, 0x01]);
        }
        if let Some(length) = config.max_apdu_length {
            properties.insert((0, PID_MAX_APDU_LENGTH), length.to_be_bytes().to_vec());
        }
        if let Some(length) = config.router_max_apdu_length {
            properties.insert((6, PID_MAX_APDU_LENGTH), length.to_be_bytes().to_vec());
        }
        if let Some(counter) = config.download_counter {
            properties.insert((0, PID_DOWNLOAD_COUNTER), counter.to_be_bytes().to_vec());
        }

        let mut memory = HashMap::new();
        memory.insert(0x0060, u8::from(config.programming_mode));

        let state = State {
            address,
            first_load_state_read: None,
            connected: false,
            dropped: false,
            load_state_reads: 0,
            device_control_reads: 0,
            connects: 0,
            numbered_data_frames: 0,
            level: config.free_access_level,
            verify_mode: false,
            send_seq: 0,
            frames: 0,
            load: HashMap::new(),
            completing: HashMap::new(),
            error_code: HashMap::new(),
            reference: HashMap::new(),
            allocation_attempts: HashMap::new(),
            mcb: HashMap::new(),
            properties,
            memory,
            seen: Vec::new(),
        };

        Self {
            client,
            config,
            events,
            state: Mutex::new(state),
        }
    }

    /// Where the simulated device currently lives — the address it was
    /// built with, unless a broadcast `A_IndividualAddress_Write` has since
    /// renamed it (see [`State::address`]).
    pub fn address(&self) -> IndividualAddress {
        self.lock().address
    }

    /// Puts a load state in place without a download having produced it, so
    /// a test of one transition does not have to drive every earlier one.
    pub fn preset_load_state(&self, object_index: ObjectIndex, state: LoadState) {
        self.lock().load.insert(object_index.octet(), state);
    }

    /// Puts a `LoadCompleting` countdown in place without a download
    /// having produced it: the next `count` *answered* load-state reads
    /// report `LoadCompleting`, and the one after that reports whatever
    /// [`SimulatedDevice::preset_load_state`] left behind.
    ///
    /// A counter rather than a clock, so a test that needs a state to
    /// settle can say *after how many reads* instead of *after how long*.
    /// Reads the device does not answer do not count, which is the point:
    /// a quiet poll must not be able to spend the countdown.
    pub fn preset_load_completing_polls(&self, object_index: ObjectIndex, count: u8) {
        self.lock().completing.insert(object_index.octet(), count);
    }

    /// Puts octets in the device's memory.
    pub fn preset_memory(&self, base: u32, data: &[u8]) {
        let mut state = self.lock();
        for (offset, octet) in data.iter().enumerate() {
            state.memory.insert(base + offset as u32, *octet);
        }
    }

    /// Puts a Memory Control Block in place for one object, so that a
    /// partial download has a CRC to compare against.
    pub fn preset_mcb(&self, object_index: ObjectIndex, octets: &[u8]) {
        self.lock()
            .mcb
            .insert(object_index.octet(), octets.to_vec());
    }

    /// The Memory Control Block one object currently answers.
    pub fn mcb(&self, object_index: ObjectIndex) -> Vec<u8> {
        self.lock()
            .mcb
            .get(&object_index.octet())
            .cloned()
            .unwrap_or_else(|| DEFAULT_MCB.to_vec())
    }

    /// What the device holds at `base`, or `None` for an octet never
    /// written.
    pub fn memory(&self, base: u32, length: usize) -> Vec<Option<u8>> {
        let state = self.lock();
        (0..length as u32)
            .map(|offset| state.memory.get(&(base + offset)).copied())
            .collect()
    }

    /// The load state the device currently reports.
    pub fn load_state(&self, object_index: ObjectIndex) -> LoadState {
        self.lock()
            .load
            .get(&object_index.octet())
            .copied()
            .unwrap_or(LoadState::Unloaded)
    }

    /// Everything the device was asked to do, in order.
    pub fn seen(&self) -> Vec<Seen> {
        self.lock().seen.clone()
    }

    /// Whether anything at all reached the device's memory.
    pub fn memory_was_written(&self) -> bool {
        self.lock()
            .seen
            .iter()
            .any(|entry| matches!(entry, Seen::MemoryWrite { .. }))
    }

    /// How many `A_Authorize_Request` frames arrived.
    pub fn authorize_requests(&self) -> usize {
        self.lock()
            .seen
            .iter()
            .filter(|entry| matches!(entry, Seen::Authorize(_)))
            .count()
    }

    /// How many broadcast `A_IndividualAddress_Read` frames arrived.
    pub fn individual_address_read_broadcasts(&self) -> usize {
        self.lock()
            .seen
            .iter()
            .filter(|entry| matches!(entry, Seen::IndividualAddressReadBroadcast))
            .count()
    }

    /// Whether Verify Mode is on as far as the device is concerned.
    pub fn verify_mode(&self) -> bool {
        self.lock().verify_mode
    }

    /// How many `PID_LOAD_STATE_CONTROL` reads have arrived, so a test can
    /// pin an exact attempt count instead of trusting an outer time-out to
    /// notice an extra one.
    pub fn load_state_reads(&self) -> u32 {
        self.lock().load_state_reads
    }

    /// How many `PID_DEVICE_CONTROL` reads have arrived — one per
    /// `connect()` that got as far as its Verify Mode step, which is what
    /// makes a re-established connection countable from the device's side.
    pub fn device_control_reads(&self) -> u32 {
        self.lock().device_control_reads
    }

    /// Tears the connection down from the device's side, as a real one does
    /// after 6 s of silence.
    pub fn break_connection(&self) {
        self.lock().connected = false;
        self.emit(Tpci::Disconnect, ApplicationService::NoApplicationPdu);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn emit(&self, transport: Tpci, service: ApplicationService) {
        let source = self.lock().address;
        self.emit_from(source, transport, service);
    }

    /// [`Self::emit`], with an explicit source address instead of this
    /// device's own — the broadcast `A_IndividualAddress_Read` handler
    /// uses this to speak for [`SimulatorConfig::other_programming_mode_devices`],
    /// synthetic responders this one `SimulatedDevice` answers on behalf
    /// of without being them.
    fn emit_from(&self, source: IndividualAddress, transport: Tpci, service: ApplicationService) {
        // A broadcast send with no receivers is not an error here: a test
        // that has not subscribed is a test that does not care.
        let _ = self.events.send(TunnelEvent::Telegram(LDataFrame {
            kind: LDataMessageKind::Indication,
            source,
            destination: Destination::Individual(self.client),
            transport,
            service,
        }));
    }

    fn emit_connect_confirmation(&self, error: bool) {
        let address = self.lock().address;
        let _ = self.events.send(TunnelEvent::Telegram(LDataFrame {
            kind: LDataMessageKind::Confirmation { error },
            source: self.client,
            destination: Destination::Individual(address),
            transport: Tpci::Connect,
            service: ApplicationService::NoApplicationPdu,
        }));
    }

    fn emit_answer(&self, service: ApplicationService) {
        let seq = {
            let mut state = self.lock();
            let seq = state.send_seq & 0x0F;
            state.send_seq = (state.send_seq + 1) & 0x0F;
            seq
        };
        self.emit(Tpci::NumberedData { seq }, service);
    }

    /// Whether a write is allowed at the level this connection holds.
    fn may_write(&self) -> bool {
        let state = self.lock();
        state.level <= self.config.write_requires_level
    }

    fn property_read(&self, object_index: u8, property_id: u8) -> Option<Vec<u8>> {
        let mut state = self.lock();
        match property_id {
            PID_LOAD_STATE_CONTROL => {
                if let Some(delay) = self.config.settle_load_state_after {
                    // Set moments ago, in `handle`, by this very read if it
                    // is the first one — never `None` here.
                    let first_read = state
                        .first_load_state_read
                        .expect("a load-state read is in progress");
                    if first_read.elapsed() >= delay {
                        return Some(vec![self.config.settled_load_state.octet()]);
                    }
                }
                let current = state
                    .load
                    .get(&object_index)
                    .copied()
                    .unwrap_or(LoadState::Unloaded);
                let remaining = state.completing.get(&object_index).copied().unwrap_or(0);
                if remaining > 0 {
                    state.completing.insert(object_index, remaining - 1);
                    return Some(vec![LoadState::LoadCompleting.octet()]);
                }
                Some(vec![current.octet()])
            }
            PID_ERROR_CODE => Some(vec![state
                .error_code
                .get(&object_index)
                .copied()
                .unwrap_or(0)]),
            PID_TABLE_REFERENCE => {
                let reference = state.reference.get(&object_index).copied().unwrap_or(0);
                Some(reference.to_be_bytes().to_vec())
            }
            PID_MCB_TABLE => Some(
                state
                    .mcb
                    .get(&object_index)
                    .cloned()
                    .unwrap_or_else(|| DEFAULT_MCB.to_vec()),
            ),
            _ => state.properties.get(&(object_index, property_id)).cloned(),
        }
    }

    /// The Load State Machine of RES Table 94, p. 296, as far as a simulator
    /// needs it: mostly the table's recommended (`R`) transition for each
    /// event, except `LoadCompleted` and `AdditionalLoadControls` received in
    /// `Unloaded`, where Table 94 marks `Error` as the optional (`O`)
    /// transition rather than the recommended one (staying `Unloaded`) — the
    /// legend defines, in its own printed order, I (intermediate state), M
    /// (mandatory), O (optional) and R (recommended), and both O and R are
    /// legal device behaviour, so this simulator simply exercises the
    /// optional one. Plus the two ways design spec §11.3 asks it to go wrong.
    fn apply_event(&self, object_index: u8, event: LoadEvent) {
        if self.config.drop_load_state_writes {
            return;
        }
        let before = self.load_state(ObjectIndex::new(object_index));
        if self.config.fail_on_event == Some(event) {
            let mut state = self.lock();
            state.load.insert(object_index, LoadState::Error);
            state
                .error_code
                .insert(object_index, self.config.error_code);
            return;
        }
        let after = match (before, event) {
            (_, LoadEvent::NoOperation) => before,
            (LoadState::Error, LoadEvent::Unload) => LoadState::Unloaded,
            // `[D]` RES Table 94: Error is left by Unload and by nothing
            // else, so every other event in Error is a no-op.
            (LoadState::Error, _) => LoadState::Error,
            (_, LoadEvent::StartLoading) => LoadState::Loading,
            (LoadState::Loading, LoadEvent::LoadCompleted) => LoadState::Loaded,
            (_, LoadEvent::LoadCompleted) => LoadState::Error,
            (_, LoadEvent::Unload) => LoadState::Unloaded,
            (LoadState::Loading, LoadEvent::AdditionalLoadControls) => LoadState::Loading,
            (_, LoadEvent::AdditionalLoadControls) => LoadState::Error,
        };
        let mut state = self.lock();
        state.load.insert(object_index, after);
        if after == LoadState::Unloaded {
            // §7.6: an unload frees the memory and zeroes the reference.
            state.reference.remove(&object_index);
        }
        if after != LoadState::Error {
            state.error_code.remove(&object_index);
        }
        if after == LoadState::Loaded && self.config.load_completing_polls > 0 {
            state
                .completing
                .insert(object_index, self.config.load_completing_polls);
        }
    }

    /// An `Additional Load Control` whose subtype allocates, in `Loading`.
    fn apply_allocation(&self, object_index: u8, payload: &[u8]) {
        if payload.len() < 2 {
            return;
        }
        let subtype = payload[1];
        let allocates = ALL_SUBTYPES
            .into_iter()
            .any(|candidate| candidate.octet() == subtype && candidate.is_allocation());
        if !allocates {
            return;
        }
        // `[D]` §7.6: an allocation outside Loading is ignored, which is
        // one of the two things a zero reference can mean.
        if self.load_state(ObjectIndex::new(object_index)) != LoadState::Loading {
            return;
        }
        if self.config.reference_always_zero {
            return;
        }
        let attempt = {
            let mut state = self.lock();
            let attempt = state
                .allocation_attempts
                .entry(object_index)
                .and_modify(|count| *count += 1)
                .or_insert(1);
            *attempt
        };
        if self.config.allocation_fails_once_for == Some(object_index) && attempt == 1 {
            // CP §3.5.3's failed allocation: no reference, no error, no
            // complaint. The client finds out by reading zero.
            return;
        }
        let base = ALLOCATION_BASE + u32::from(object_index) * 0x1000;
        self.lock().reference.insert(object_index, base);
    }

    /// Answers a frame sent to the broadcast destination `0/0/0`.
    ///
    /// MP §2.3, pp. 14-15: `A_IndividualAddress_Read` and
    /// `A_IndividualAddress_Write` are the only two broadcast services
    /// design spec §7's download procedures ever use. Every device this
    /// simulator can pretend to be — itself, plus
    /// [`SimulatorConfig::other_programming_mode_devices`] — answers on
    /// its own behalf, exactly as distinct devices sharing one bus would;
    /// none of this goes through [`Self::handle`], `T_Connect`, or any
    /// notion of a connection, because a broadcast is connectionless by
    /// definition — AL §3.2.1, p. 18: *"A broadcast communication mode
    /// shall be connectionless and shall connect one device with all
    /// others."*
    fn handle_broadcast(&self, transport: Tpci, service: ApplicationService) {
        if transport != Tpci::UnnumberedData {
            return;
        }
        match service {
            ApplicationService::IndividualAddressRead => {
                let own_address = {
                    let mut state = self.lock();
                    state.seen.push(Seen::IndividualAddressReadBroadcast);
                    state.address
                };
                if self.config.programming_mode {
                    self.emit_from(
                        own_address,
                        Tpci::UnnumberedData,
                        ApplicationService::IndividualAddressResponse,
                    );
                }
                for &other in &self.config.other_programming_mode_devices {
                    self.emit_from(
                        other,
                        Tpci::UnnumberedData,
                        ApplicationService::IndividualAddressResponse,
                    );
                }
                if let Some(noisy) = self.config.unrelated_traffic_during_broadcast {
                    // Somebody else's group telegram, landing in the
                    // middle of the window. Nothing about it is an
                    // `A_IndividualAddress_Response-PDU`, so MP §2.3 step
                    // 2's count must not move.
                    self.emit_from(
                        noisy,
                        Tpci::UnnumberedData,
                        ApplicationService::GroupValueWrite(GroupValue::Short(1)),
                    );
                    // And the `L_Data.con` for that same telegram, which
                    // differs from a response only in `kind`. Synthetic:
                    // no real device confirms an
                    // `A_IndividualAddress_Response` it did not send. It
                    // exists so the counting loop's `Indication` half has
                    // something to reject that its service half would
                    // wave through.
                    let _ = self.events.send(TunnelEvent::Telegram(LDataFrame {
                        kind: LDataMessageKind::Confirmation { error: false },
                        source: noisy,
                        destination: BROADCAST_DESTINATION,
                        transport: Tpci::UnnumberedData,
                        service: ApplicationService::IndividualAddressResponse,
                    }));
                }
            }
            // Unconfirmed (AL §3.5.4 does not apply here — there is no
            // connection, hence no Verify Mode, to answer through): this
            // device either takes the new address in silence or, out of
            // programming mode, ignores the write in silence. AL §3.2.2,
            // p. 18: *"The application process shall ignore the
            // A_IndividualAddress_Write.ind primitive if the device is not
            // in ´programming mode´. Otherwise the local Individual
            // Address shall be set to the new address."* MP §2.3's own
            // exception handling has no "to 3." — step 3 is the one step
            // the clause raises no exception for.
            ApplicationService::IndividualAddressWrite { address }
                if self.config.programming_mode =>
            {
                self.lock().address = address;
            }
            _ => {}
        }
    }

    fn handle(&self, transport: Tpci, service: ApplicationService) {
        {
            let mut state = self.lock();
            state.frames += 1;
            let by_count = self
                .config
                .drop_connection_after
                .is_some_and(|limit| state.frames > limit);
            let is_load_state_read = matches!(
                service,
                ApplicationService::PropertyValueRead {
                    property_id: PID_LOAD_STATE_CONTROL,
                    ..
                }
            );
            if is_load_state_read {
                state.load_state_reads += 1;
                state.first_load_state_read.get_or_insert_with(Instant::now);
            }
            let is_device_control_read = matches!(
                service,
                ApplicationService::PropertyValueRead {
                    property_id: PID_DEVICE_CONTROL,
                    ..
                }
            );
            if is_device_control_read {
                state.device_control_reads += 1;
            }
            let by_read = is_load_state_read
                && self.config.drop_connection_on_load_state_read == Some(state.load_state_reads);
            // The same trigger one property along, and the only one that
            // fires inside `connect()`: the Verify Mode read is the first
            // frame the client sends on a connection it has just been
            // confirmed.
            let by_device_control_read = is_device_control_read
                && self.config.drop_connection_on_device_control_read
                    == Some(state.device_control_reads);
            let by_download_counter_read = self.config.drop_connection_on_download_counter_read
                && matches!(
                    service,
                    ApplicationService::PropertyValueRead {
                        property_id: PID_DOWNLOAD_COUNTER,
                        ..
                    }
                );
            let by_step = self
                .config
                .interrupt_at
                .is_some_and(|step| step.strikes(&service));
            if (by_count
                || by_read
                || by_device_control_read
                || by_download_counter_read
                || by_step)
                && state.connected
                && !state.dropped
            {
                state.connected = false;
                state.dropped = true;
                state.verify_mode = false;
                // RES §4.2.14.7.3: the bit dies with the connection,
                // however the connection died.
                if let Some(control) = state.properties.get_mut(&(0, PID_DEVICE_CONTROL)) {
                    *control = vec![0x00];
                }
                drop(state);
                self.emit(Tpci::Disconnect, ApplicationService::NoApplicationPdu);
                return;
            }
        }

        match transport {
            Tpci::Connect => {
                let mut state = self.lock();
                state.connected = true;
                state.level = self.config.free_access_level;
                state.verify_mode = false;
                state.send_seq = 0;
                state.seen.push(Seen::Connect);
                return;
            }
            Tpci::Disconnect => {
                let mut state = self.lock();
                state.connected = false;
                // RES §4.2.14.7.4: Verify Mode dies with the connection.
                state.verify_mode = false;
                if let Some(control) = state.properties.get_mut(&(0, PID_DEVICE_CONTROL)) {
                    *control = vec![0x00];
                }
                state.seen.push(Seen::Disconnect);
                return;
            }
            Tpci::Ack { .. } | Tpci::Nak { .. } => return,
            Tpci::NumberedData { seq } => {
                if !self.lock().connected {
                    return;
                }
                if self.config.silent {
                    // Not even a T_ACK: silence in §11.3 means silence.
                    self.record(&service);
                    return;
                }
                if self.config.restart_unanswered
                    && matches!(
                        service,
                        ApplicationService::Restart {
                            response: false,
                            ..
                        }
                    )
                {
                    // Same shape as `unacknowledged_load_state_reads`: no
                    // `T_ACK`, nothing recorded, so the client's four
                    // transmissions and TL's own release afterwards are
                    // what the test is watching, not this drop.
                    return;
                }
                if self.load_state_read_in(&service, &self.config.unacknowledged_load_state_reads) {
                    // An offline device acknowledges nothing and remembers
                    // nothing, so this one does not record the frame either.
                    // The client's four transmissions and the release that
                    // follows them are what the test is watching.
                    return;
                }
                let still_dropping = {
                    let mut state = self.lock();
                    state.numbered_data_frames += 1;
                    state.numbered_data_frames <= self.config.silent_for_first_numbered_data_frames
                };
                if still_dropping {
                    // A transmission lost in transit: no T_ACK, no answer,
                    // nothing recorded — the client's own retry is what is
                    // under test here, not this drop.
                    return;
                }
                self.emit(Tpci::Ack { seq }, ApplicationService::NoApplicationPdu);
            }
            Tpci::UnnumberedData | Tpci::Unknown(_) => {}
        }

        self.record(&service);
        self.answer(service);
    }

    fn record(&self, service: &ApplicationService) {
        let entry = match service {
            ApplicationService::AuthorizeRequest { key } => {
                Seen::Authorize(u32::from_be_bytes(*key))
            }
            ApplicationService::PropertyValueRead {
                object_index,
                property_id,
                ..
            } => Seen::PropertyRead {
                object_index: *object_index,
                property_id: *property_id,
            },
            ApplicationService::PropertyValueWrite {
                object_index,
                property_id,
                data,
                ..
            } => Seen::PropertyWrite {
                object_index: *object_index,
                property_id: *property_id,
                data: data.clone(),
            },
            ApplicationService::MemoryRead { number, address } => Seen::MemoryRead {
                address: u32::from(*address),
                number: *number,
                service: MemoryService::Memory,
            },
            ApplicationService::UserMemoryRead { number, address } => Seen::MemoryRead {
                address: *address,
                number: *number,
                service: MemoryService::UserMemory,
            },
            ApplicationService::MemoryWrite { address, data } => Seen::MemoryWrite {
                address: u32::from(*address),
                data: data.clone(),
                service: MemoryService::Memory,
            },
            ApplicationService::UserMemoryWrite { address, data } => Seen::MemoryWrite {
                address: *address,
                data: data.clone(),
                service: MemoryService::UserMemory,
            },
            ApplicationService::Restart {
                response,
                restart_type,
                data,
            } => Seen::Restart {
                response: *response,
                restart_type: *restart_type,
                data: data.clone(),
            },
            ApplicationService::NoApplicationPdu => return,
            other => Seen::Other(other.variant_name()),
        };
        self.lock().seen.push(entry);
    }

    /// Whether this frame is a `PID_LOAD_STATE_CONTROL` read whose number
    /// falls inside `window`.
    ///
    /// The number is `State::load_state_reads`, which `handle` has already
    /// incremented for this frame, and which counts every transmission —
    /// so a window four wide is exactly one of the client's
    /// `MAX_TRANSMISSIONS` ladders and not four polls.
    fn load_state_read_in(
        &self,
        service: &ApplicationService,
        window: &Option<Range<u32>>,
    ) -> bool {
        let number = self.lock().load_state_reads;
        numbered_read_in(service, PID_LOAD_STATE_CONTROL, number, window)
    }

    /// The same question about `PID_DEVICE_CONTROL`, whose counter `handle`
    /// has likewise already incremented for this frame.
    fn device_control_read_in(
        &self,
        service: &ApplicationService,
        window: &Option<Range<u32>>,
    ) -> bool {
        let number = self.lock().device_control_reads;
        numbered_read_in(service, PID_DEVICE_CONTROL, number, window)
    }

    fn answer(&self, service: ApplicationService) {
        if self.load_state_read_in(&service, &self.config.unanswered_load_state_reads) {
            // The T_ACK went out from `handle` moments ago; only the
            // application answer is withheld. A device that is busy rather
            // than gone.
            return;
        }
        if self.device_control_read_in(&service, &self.config.unanswered_device_control_reads) {
            // Likewise acknowledged and unanswered, one property along. The
            // read is not performed either: a device that never answered did
            // not change anything on the way.
            return;
        }
        match service {
            ApplicationService::AuthorizeRequest { key } => {
                let key = u32::from_be_bytes(key);
                let level = match self.config.key {
                    Some(expected) if key == expected => self.config.key_level,
                    // `[D]` MP §3.5.1: an unknown key drops the partner to
                    // the minimum level, which is worse than not trying.
                    Some(_) if key != 0xFFFF_FFFF => 15,
                    _ => self.config.free_access_level,
                };
                self.lock().level = level;
                self.emit_answer(ApplicationService::AuthorizeResponse { level });
            }
            ApplicationService::PropertyValueRead {
                object_index,
                property_id,
                nr_of_elem,
                start_index,
            } => {
                let data = self.property_read(object_index, property_id);
                let (nr_of_elem, data) = match data {
                    Some(data) => (nr_of_elem.max(1), data),
                    None => (0, Vec::new()),
                };
                self.emit_answer(ApplicationService::PropertyValueResponse {
                    object_index,
                    property_id,
                    nr_of_elem,
                    start_index,
                    data,
                });
            }
            ApplicationService::PropertyValueWrite {
                object_index,
                property_id,
                nr_of_elem,
                start_index,
                data,
            } => {
                let answer = self.property_write(object_index, property_id, &data);
                let (nr_of_elem, data) = match answer {
                    Some(read_back) => (nr_of_elem.max(1), read_back),
                    None => (0, Vec::new()),
                };
                if self.config.silent_in_load_completing
                    && property_id == PID_LOAD_STATE_CONTROL
                    && self.load_state(ObjectIndex::new(object_index)) == LoadState::LoadCompleting
                {
                    return;
                }
                self.emit_answer(ApplicationService::PropertyValueResponse {
                    object_index,
                    property_id,
                    nr_of_elem,
                    start_index,
                    data,
                });
            }
            ApplicationService::DeviceDescriptorRead { descriptor_type: 0 } => {
                if self.config.device_descriptor_read_unanswered {
                    // The `T_ACK` has already gone out in `handle`'s
                    // `NumberedData` arm, so the client's Transport Layer
                    // stays in `OPEN_IDLE` and the silence belongs to the
                    // application layer. MP §2.3, p. 14: *"If no
                    // A_DeviceDescriptor_Response-PDU is received after
                    // time-out ⇒ IA_new is not occupied"*.
                    return;
                }
                if self.config.device_descriptor_read_gets_disconnect {
                    // MP §2.3, p. 14, the remark beside the A_Disconnect
                    // arrow: *"If the device that occupies the IA IA_new
                    // does not support Transport Layer connections, it
                    // shall send a T_Disconnect-PDU."*
                    let mut state = self.lock();
                    state.connected = false;
                    state.verify_mode = false;
                    if let Some(control) = state.properties.get_mut(&(0, PID_DEVICE_CONTROL)) {
                        *control = vec![0x00];
                    }
                    drop(state);
                    self.emit(Tpci::Disconnect, ApplicationService::NoApplicationPdu);
                    return;
                }
                // AL §3.4.2.1 Figure 38: two octets, most significant
                // first, and step 02 of every procedure in design spec §7 reads
                // them before it decides anything.
                self.emit_answer(ApplicationService::DeviceDescriptorResponse {
                    descriptor_type: 0,
                    data: self.config.mask_version.to_be_bytes().to_vec(),
                });
            }
            ApplicationService::MemoryRead { number, address } => {
                let data = self.memory_read(u32::from(address), number);
                self.emit_answer(ApplicationService::MemoryResponse { address, data });
            }
            ApplicationService::UserMemoryRead { number, address } => {
                let data = self.memory_read(address, number);
                self.emit_answer(ApplicationService::UserMemoryResponse { address, data });
            }
            ApplicationService::MemoryWrite { address, data } => {
                let answer = self.memory_write(u32::from(address), &data);
                if let Some(read_back) = answer {
                    self.emit_answer(ApplicationService::MemoryResponse {
                        address,
                        data: read_back,
                    });
                }
            }
            ApplicationService::UserMemoryWrite { address, data } => {
                let answer = self.memory_write(address, &data);
                if let Some(read_back) = answer {
                    self.emit_answer(ApplicationService::UserMemoryResponse {
                        address,
                        data: read_back,
                    });
                }
            }
            // MP §3.7.1.1.3, p. 80: *"The Application Layer of the
            // Management Server shall not confirm the A_Restart-service if
            // a Basic Restart is called"* — so `restart_type: 0` earns no
            // arm here and falls to the catch-all below; the T_ACK
            // `handle` already sent is this device's only word on the
            // matter.
            ApplicationService::Restart {
                response: false,
                restart_type: 1,
                data: request,
            } => {
                // Table 4 (MP p. 82) fixes Channel Number at `00h` for Erase
                // Code `01h` (`ERASE_CODE_CONFIRMED_RESTART`); any other
                // Channel Number paired with it is not a request this
                // device's configured happy-path answer applies to. MP
                // §3.7.3 exception (4), p. 90, gives Error Code `03h`
                // ("Invalid Channel Number") for exactly this mismatch, and
                // Process Time `0` — the device is not about to erase
                // anything, so it has nothing to time.
                let (error_code, process_time) = match request.as_slice() {
                    [ERASE_CODE_CONFIRMED_RESTART, channel_number] if *channel_number != 0x00 => {
                        (0x03, 0u16)
                    }
                    _ => (
                        self.config.restart_error_code,
                        // DPT_TimePeriodSec, big-endian (MP §3.7.1.2.2, pp. 80-81).
                        u16::try_from(self.config.restart_process_time.as_secs())
                            .unwrap_or(u16::MAX),
                    ),
                };
                let mut data = vec![error_code];
                data.extend_from_slice(&process_time.to_be_bytes());
                if self.config.restart_response_malformed {
                    // Truncate to two octets: still a `Some(data)` match for
                    // `restart_master_reset`'s matcher, since that matcher
                    // looks only at `restart_type`/`response`, not length —
                    // the malformation has to survive past `exchange()` and
                    // into `decode_master_reset_response` to prove anything.
                    data.truncate(2);
                }
                self.emit_answer(ApplicationService::Restart {
                    response: true,
                    restart_type: 1,
                    data,
                });
            }
            _ => {}
        }
    }

    fn property_write(&self, object_index: u8, property_id: u8, data: &[u8]) -> Option<Vec<u8>> {
        if !self.may_write() {
            return None;
        }
        match property_id {
            PID_LOAD_STATE_CONTROL => {
                let event = data.first().copied().and_then(load_event_from_octet)?;
                self.apply_event(object_index, event);
                if event == LoadEvent::AdditionalLoadControls {
                    self.apply_allocation(object_index, data);
                }
                // `[D]` CP NOTE 9 with AL §3.4.4.2, p. 66: the answer to a
                // write of a PDT_CONTROL property is a read of it, and a
                // read of this one is the resulting state — never the ten
                // octets sent.
                Some(vec![self
                    .load_state(ObjectIndex::new(object_index))
                    .octet()])
            }
            PID_DEVICE_CONTROL => {
                let wanted = *data.first()?;
                let stored = if self.config.verify_mode_supported {
                    wanted
                } else {
                    // PROF footnote: a device without Verify Mode keeps the
                    // bit clear, and says so by reading back clear.
                    wanted & !0b0000_0100
                };
                let mut state = self.lock();
                state.verify_mode = verify_mode_active(stored);
                state
                    .properties
                    .insert((0, PID_DEVICE_CONTROL), vec![stored]);
                Some(vec![stored])
            }
            PID_PROGRAM_VERSION
                if !self
                    .config
                    .application_program_objects
                    .contains(&object_index) =>
            {
                // `[C1]` RES Table 77, p. 238; Table 80, p. 249; Table 85,
                // p. 270: none of the Group Address Table, the Association
                // Table or the Group Object Table has this property. `[D]`
                // AL §3.4.4.2, p. 66: a property that does not exist is
                // answered with `nr_of_elem = 0`, which the `None` below
                // reproduces.
                None
            }
            _ => {
                self.lock()
                    .properties
                    .insert((object_index, property_id), data.to_vec());
                Some(data.to_vec())
            }
        }
    }

    fn memory_read(&self, address: u32, number: u8) -> Vec<u8> {
        if self.is_protected(address) {
            // AL §3.5.3's failure answer: number = 0, no data.
            return Vec::new();
        }
        let state = self.lock();
        (0..u32::from(number))
            .map(|offset| state.memory.get(&(address + offset)).copied().unwrap_or(0))
            .collect()
    }

    fn memory_write(&self, address: u32, data: &[u8]) -> Option<Vec<u8>> {
        if self.is_protected(address) || !self.may_write() {
            // With Verify Mode on the refusal is number = 0; with it off
            // there is no answer at all, and the client's own read-back is
            // what notices.
            return self.lock().verify_mode.then(Vec::new);
        }
        let mut stored = data.to_vec();
        if self.config.corrupt_memory_writes {
            if let Some(first) = stored.first_mut() {
                *first = !*first;
            }
        }
        let mut state = self.lock();
        for (offset, octet) in stored.iter().enumerate() {
            state.memory.insert(address + offset as u32, *octet);
        }
        // `[D]` AL §3.5.4: the device responds only while Verify Mode is
        // active, and then with what it read back — which is what it
        // stored, not what it was sent. The difference is the whole point
        // of reading back.
        state.verify_mode.then_some(stored)
    }

    fn is_protected(&self, address: u32) -> bool {
        match self.config.protected_memory {
            Some((from, to)) => address >= from && address < to,
            None => false,
        }
    }
}

impl Default for SimulatedDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagementTransport for SimulatedDevice {
    fn assigned_address(&self) -> IndividualAddress {
        self.client
    }

    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.events.subscribe()
    }

    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        if destination == BROADCAST_DESTINATION {
            self.handle_broadcast(transport, service);
            return Ok(());
        }
        if destination != Destination::Individual(self.lock().address) {
            // A frame for somebody else is dropped rather than answered:
            // the simulator is one device, not a bus.
            return Ok(());
        }
        if transport == Tpci::Connect {
            let attempt = {
                let mut state = self.lock();
                state.connects += 1;
                state.connects
            };
            if self
                .config
                .unanswered_connects
                .as_ref()
                .is_some_and(|window| window.contains(&attempt))
            {
                // An offline device answers nothing and remembers nothing,
                // so neither the confirmation nor `handle` happens here.
                return Ok(());
            }
            if self
                .config
                .rejected_connects
                .as_ref()
                .is_some_and(|window| window.contains(&attempt))
            {
                // The confirmation comes back, and it says no. A frame the
                // Data Link Layer could not get acknowledged never reached
                // the device, so `handle` does not run for it either.
                self.emit_connect_confirmation(true);
                return Ok(());
            }
            self.emit_connect_confirmation(false);
        }
        self.handle(transport, service);
        Ok(())
    }

    fn target_kind(&self) -> TargetKind {
        TargetKind::Simulator
    }
}
