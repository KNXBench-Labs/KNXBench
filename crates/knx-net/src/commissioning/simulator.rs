//! A device simulator that answers management services, including badly, so no test needs a bus.
//!
//! Spec §11.3 asks for *"a simulator that can be told to misbehave"* and
//! lists the ways: silence, a failure at a named step, no protected areas,
//! load-state writes that are read but ignored, a reference that never
//! allocates, an absent Verify Mode, a connection that dies mid-download.
//! Each of those is a field of [`SimulatorConfig`] and each has a test.
//!
//! What this is not: a conformant KNX device. It answers the subset of the
//! Application Layer the download procedures of spec §7 use, with the
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

use std::collections::HashMap;
use std::sync::Mutex;

use knx_core::commissioning::load_control::LoadControlSubtype;
use knx_core::commissioning::load_state::{LoadEvent, LoadState};
use knx_core::commissioning::memory::MemoryService;
use knx_core::commissioning::mutation::TargetKind;
use knx_core::commissioning::properties::{
    verify_mode_active, ObjectIndex, PID_DEVICE_CONTROL, PID_ERROR_CODE, PID_LOAD_STATE_CONTROL,
    PID_MANUFACTURER_ID, PID_MAX_APDU_LENGTH, PID_MCB_TABLE, PID_PROGRAM_VERSION,
    PID_TABLE_REFERENCE,
};
use knx_core::IndividualAddress;
use tokio::sync::broadcast;

use crate::cemi::{ApplicationService, Destination, LDataFrame, LDataMessageKind, Tpci};
use crate::client::{BusError, TunnelEvent};
use crate::management::ManagementTransport;

/// The address the simulated management client appears to have.
///
/// Inside the approved read range of spec §2.2 rather than outside it, so
/// that a fixture address never looks like an invitation to try it on a bus.
pub const SIMULATED_CLIENT_ADDRESS: (u8, u8, u8) = (1, 1, 24);

/// The address the simulated device appears to have.
pub const SIMULATED_DEVICE_ADDRESS: (u8, u8, u8) = (1, 1, 25);

/// Where the simulator's memory hands out allocations, when it allocates.
const ALLOCATION_BASE: u32 = 0x4000;

/// The Memory Control Block octets an object answers when nothing has put
/// others there.
///
/// Eight octets with two of them standing in for a CRC. The layout is *not*
/// a claim: spec §12 records that the MCB's internal structure is not in
/// either knowledge base, so the client carries these octets and compares
/// them whole, which is all §7.4 asks of it.
const DEFAULT_MCB: [u8; 8] = [0x00, 0x00, 0x10, 0x00, 0xAB, 0xCD, 0x00, 0x00];

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
    /// footnote permits and spec §5.5 requires the client to tolerate.
    pub silent_in_load_completing: bool,
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
    /// higher level is refused with `nr_of_elem = 0`, which AL §3.4.4.2
    /// gives as the answer for insufficient access rights.
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
    pub programming_mode: bool,
    /// The mask version Device Descriptor Type 0 answers. `07B0h` by
    /// default: a System B mask, which is the profile spec §7's CP §3.5.2
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
    /// This is spec §11.3's *"must be able to be told to fail at a chosen
    /// step"*, and it is what §14 item 8's interruption at every step is
    /// built on. A one-shot, like every other drop here, so that the
    /// recovery procedure has a device to recover.
    pub interrupt_at: Option<Interruption>,
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
            verify_mode_supported: true,
            drop_load_state_writes: false,
            reference_always_zero: false,
            drop_connection_after: None,
            drop_connection_on_load_state_read: None,
            fail_on_event: None,
            error_code: 2,
            load_completing_polls: 0,
            silent_in_load_completing: false,
            max_apdu_length: Some(15),
            router_max_apdu_length: None,
            free_access_level: 0,
            key: None,
            key_level: 0,
            write_requires_level: 15,
            protected_memory: None,
            corrupt_memory_writes: false,
            programming_mode: false,
            mask_version: 0x07B0,
            allocation_fails_once_for: None,
            interrupt_at: None,
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
    /// Anything else, by name.
    Other(&'static str),
}

#[derive(Debug)]
struct State {
    connected: bool,
    /// Whether the one connection drop the configuration asks for has
    /// happened. Once, not on every frame afterwards: a connection that
    /// dies again immediately is a different failure mode, and an
    /// unreachable device is already [`SimulatorConfig::silent`].
    dropped: bool,
    /// How many reads of `PID_LOAD_STATE_CONTROL` have arrived.
    load_state_reads: u32,
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
    /// The Memory Control Block per object, whose CRC octets spec §7.2 step
    /// 7 stores and §7.4 compares.
    mcb: HashMap<u8, Vec<u8>>,
    properties: HashMap<(u8, u8), Vec<u8>>,
    memory: HashMap<u32, u8>,
    seen: Vec<Seen>,
}

/// A device that answers management services, wrongly on request.
pub struct SimulatedDevice {
    address: IndividualAddress,
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
        properties.insert((0, PID_PROGRAM_VERSION), vec![0x00, 0x02, 0x12, 0x34, 0x01]);
        if let Some(length) = config.max_apdu_length {
            properties.insert((0, PID_MAX_APDU_LENGTH), length.to_be_bytes().to_vec());
        }
        if let Some(length) = config.router_max_apdu_length {
            properties.insert((6, PID_MAX_APDU_LENGTH), length.to_be_bytes().to_vec());
        }

        let mut memory = HashMap::new();
        memory.insert(0x0060, u8::from(config.programming_mode));

        let state = State {
            connected: false,
            dropped: false,
            load_state_reads: 0,
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
            address,
            client,
            config,
            events,
            state: Mutex::new(state),
        }
    }

    /// Where the simulated device lives.
    pub fn address(&self) -> IndividualAddress {
        self.address
    }

    /// Puts a load state in place without a download having produced it, so
    /// a test of one transition does not have to drive every earlier one.
    pub fn preset_load_state(&self, object_index: ObjectIndex, state: LoadState) {
        self.lock().load.insert(object_index.octet(), state);
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

    /// Whether Verify Mode is on as far as the device is concerned.
    pub fn verify_mode(&self) -> bool {
        self.lock().verify_mode
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
        // A broadcast send with no receivers is not an error here: a test
        // that has not subscribed is a test that does not care.
        let _ = self.events.send(TunnelEvent::Telegram(LDataFrame {
            kind: LDataMessageKind::Indication,
            source: self.address,
            destination: Destination::Individual(self.client),
            transport,
            service,
        }));
    }

    fn emit_connect_confirmation(&self) {
        let _ = self.events.send(TunnelEvent::Telegram(LDataFrame {
            kind: LDataMessageKind::Confirmation { error: false },
            source: self.client,
            destination: Destination::Individual(self.address),
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
    /// legend reads "R: recommended … O: optional", and both are legal
    /// device behaviour, so this simulator simply exercises the optional
    /// one. Plus the two ways spec §11.3 asks it to go wrong.
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
            }
            let by_read = is_load_state_read
                && self.config.drop_connection_on_load_state_read == Some(state.load_state_reads);
            let by_step = self
                .config
                .interrupt_at
                .is_some_and(|step| step.strikes(&service));
            if (by_count || by_read || by_step) && state.connected && !state.dropped {
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
            ApplicationService::NoApplicationPdu => return,
            other => Seen::Other(other.variant_name()),
        };
        self.lock().seen.push(entry);
    }

    fn answer(&self, service: ApplicationService) {
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
                // AL §3.4.2.1 Figure 38: two octets, most significant
                // first, and step 02 of every procedure in spec §7 reads
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
                // `[D]` CP NOTE 9 with AL §3.4.4.2: the answer to a write of
                // a PDT_CONTROL property is a read of it, and a read of this
                // one is the resulting state — never the ten octets sent.
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
        if destination != Destination::Individual(self.address) {
            // A frame for somebody else is dropped rather than answered:
            // the simulator is one device, not a bus.
            return Ok(());
        }
        if transport == Tpci::Connect {
            self.emit_connect_confirmation();
        }
        self.handle(transport, service);
        Ok(())
    }

    fn target_kind(&self) -> TargetKind {
        TargetKind::Simulator
    }
}
