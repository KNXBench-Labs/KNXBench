//! The cited download, partial-download, unload and recovery procedures, driven step by step.
//!
//! Spec §11.2 asks for *"a procedure model, not a script"*, and `knx-core`'s
//! `commissioning::procedure` is that model: the step lists of CP §3.5.2,
//! CP §3.5.3, CP §3.5.4 and spec §9.1, in the Standard's own numbering. This
//! module is what walks them against a real (well, simulated) device, and it
//! records which numbered step produced which frame so that a report and the
//! clause can be read side by side.
//!
//! Three properties are worth stating before the code:
//!
//! * **Nothing here writes without [`ManagementSession`]'s authorisation.**
//!   Every write goes through the session, which refuses without §2.3's
//!   authorisation value and refuses again unless both ends of the wire are a
//!   simulator. A sequencer is not a second door.
//! * **Nothing here retries a failed procedure in a loop.** Spec §9.1 step 6:
//!   *"Anything else is a failed recovery, reported as such — not retried in a
//!   loop."* A failure returns, with what was observed attached.
//! * **Everything a procedure will need is resolved before its first write.**
//!   Spec §9.3: *"never begin a download it does not have all the data to
//!   finish"* — which is why [`DownloadPlan::new`] refuses a part with no
//!   payload rather than discovering it halfway through step 06.
//!
//! Two places where the implementation and the clauses do not line up exactly,
//! both deliberate and both visible in the trace:
//!
//! * CP §3.5.2 numbers `Connect` 01, `Read Device Descriptor Type 0` 02 and
//!   `Authorize` 03. This session authorises as part of connecting, because
//!   spec §10.3 binds authorisation and Verify Mode to the connection rather
//!   than to the operation, so the descriptor read lands *after* the
//!   authorisation rather than before it. The trace records 01, 03, 02 in that
//!   order rather than pretending otherwise.
//! * CP §3.5.3 splits the inner loop across its Nr. 06 (*"allocate and compare
//!   the CRC"*) and its Nr. 08 (*"load the part"*). §7.2 is one loop, so the
//!   trace records it once, under Nr. 06, and Nr. 08 never appears.

use std::fmt;

use knx_core::commissioning::error_code::SystemErrorClass;
use knx_core::commissioning::load_control::{
    allocation_subtype_for, data_relative_allocation, relative_allocation, require_subtype,
    AllocationMode, AllocationSubtypeError, LoadControlPayload, LoadControlSubtype,
    DATA_RELATIVE_ALLOCATION_SIZE_OCTETS,
};
use knx_core::commissioning::load_state::{LoadEvent, LoadState, MaskVersion};
use knx_core::commissioning::mcb::MemoryControlBlock;
use knx_core::commissioning::memory::WriteLimit;
use knx_core::commissioning::mutation::WriteScope;
use knx_core::commissioning::procedure::ProcedureKind;
use knx_core::commissioning::properties::{ObjectIndex, PID_PROGRAM_VERSION};

use super::{ManagementSession, SessionError};
use crate::management::ManagementTransport;

/// One loadable part, with everything needed to load it already in hand.
///
/// `[A]` The pairing of a part with an object index is this project's, not the
/// Standard's: CP §3.5.2 names its five parts (Application Program 2 and 1,
/// Group Object Table, Address Table, Association Table) and spec §3.2 records
/// that the index they live at comes from product data. So the caller says
/// which index, and this type carries it without claiming to have derived it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadablePart {
    object_index: ObjectIndex,
    name: String,
    data: Vec<u8>,
    version: Vec<u8>,
    kind: PartKind,
    stored_mcb: Option<Vec<u8>>,
}

/// Which of the five loadable Interface Objects a part's object is.
///
/// The five, and this declaration order, are the normative download order:
/// row position in CP §3.5.2 Nr. 06-10 (the complete-download procedure)
/// and in CP §3.5.3 AP2 Nr. 08-12 (the partial-download variants) both list
/// Application Program 2, Application Program 1, the Group Object Table,
/// the Group Address Table and the Association Table in exactly this
/// sequence. `[C8]` will validate a plan against that order; this type only
/// names the five, it does not yet enforce it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PartKind {
    /// RES Table 91, p. 290.
    ApplicationProgram2,
    /// RES Table 90, p. 288.
    ApplicationProgram1,
    /// RES Table 85, p. 270.
    GroupObjectTable,
    /// RES Table 77, p. 238.
    GroupAddressTable,
    /// RES Table 80, p. 249.
    AssociationTable,
}

impl PartKind {
    /// Whether RES gives this part's object `PID_PROGRAM_VERSION`.
    ///
    /// `[C1]` True for the two application programs: RES Table 90, p. 288
    /// (Application Program 1) and Table 91, p. 290 (Application Program 2)
    /// both list the property. False for the three tables: RES Table 77,
    /// p. 238 (Group Address Table), Table 80, p. 249 (Association Table)
    /// and Table 85, p. 270 (Group Object Table) do not. `load_one_part`
    /// uses this to decide whether the version write is unconditional or
    /// merely attempted, and the simulator's [`super::simulator::
    /// SimulatorConfig::application_program_objects`] is the same fact
    /// stated the other way round.
    pub const fn has_program_version(self) -> bool {
        matches!(self, Self::ApplicationProgram1 | Self::ApplicationProgram2)
    }
}

impl LoadablePart {
    /// A part whose payload is already resolved.
    ///
    /// An empty payload is refused here rather than at the first chunk: spec
    /// §9.3 requires the whole payload of every part to exist before the first
    /// event is written, because the device cannot be put back. A part whose
    /// [`PartKind::has_program_version`] is true and has no version is
    /// refused the same way: RES Table 90 (p. 288) / Table 91 (p. 290) list
    /// `PID_PROGRAM_VERSION` for it, CP §3.5.2 Nr. 06/07 (p. 43) requires the
    /// write, and a plan with no version to write cannot execute that step —
    /// whether the property is itself mandatory is Volume 6 Annex A's
    /// question, not decided here (see C18).
    pub fn new(
        object_index: ObjectIndex,
        name: impl Into<String>,
        data: Vec<u8>,
        version: Vec<u8>,
        kind: PartKind,
    ) -> Result<Self, PlanError> {
        if data.is_empty() {
            return Err(PlanError::EmptyPart { object_index });
        }
        if kind.has_program_version() && version.is_empty() {
            return Err(PlanError::MissingVersion { object_index });
        }
        Ok(Self {
            object_index,
            name: name.into(),
            data,
            version,
            kind,
            stored_mcb: None,
        })
    }

    /// The Memory Control Block a previous download stored for this part,
    /// which is the input CP §3.5.3's CRC comparison needs (spec §7.2 step 7).
    pub fn with_stored_mcb(mut self, mcb: Vec<u8>) -> Self {
        self.stored_mcb = Some(mcb);
        self
    }

    /// Which Interface Object this part's Load State Machine lives in.
    pub fn object_index(&self) -> ObjectIndex {
        self.object_index
    }

    /// What the part is called, for a report a human reads.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The payload, in full.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Which RES property list this part's object follows (`[C1]`).
    pub fn kind(&self) -> PartKind {
        self.kind
    }
}

/// Everything a download needs to know that does not come from the device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadPlan {
    expected_manufacturer_id: u16,
    parts: Vec<LoadablePart>,
    allocation_mode: AllocationMode,
    router_object: Option<ObjectIndex>,
}

impl DownloadPlan {
    /// A plan for one device.
    ///
    /// The parts are in the order they will be loaded, which is the order
    /// CP §3.5.2 steps 06 to 10 give for the five parts it names. The same
    /// order is used for the unload of step 05, because that clause's own
    /// order and its loading order agree on Application Program 2 first.
    pub fn new(expected_manufacturer_id: u16, parts: Vec<LoadablePart>) -> Result<Self, PlanError> {
        if parts.is_empty() {
            return Err(PlanError::NoParts);
        }
        for (position, part) in parts.iter().enumerate() {
            if parts[..position]
                .iter()
                .any(|earlier| earlier.object_index == part.object_index)
            {
                return Err(PlanError::DuplicatePart {
                    object_index: part.object_index,
                });
            }
        }
        Ok(Self {
            expected_manufacturer_id,
            parts,
            allocation_mode: AllocationMode::default(),
            router_object: None,
        })
    }

    /// Whether an allocation should keep the memory it gets or fill it, and
    /// with what: MP §3.31.3.4's Mode octet bit 0 and its `fill` octet,
    /// which `AllocationMode::Fill(u8)` carries together because they are
    /// only ever meaningful together.
    pub fn with_allocation_mode(mut self, mode: AllocationMode) -> Self {
        self.allocation_mode = mode;
        self
    }

    /// The Router Object to consult for `PID_MAX_APDU_LENGTH` when the Device
    /// Object has no such property — which, per spec §6.4, means 12 octets and
    /// not the value found there.
    pub fn with_router_object(mut self, router_object: ObjectIndex) -> Self {
        self.router_object = Some(router_object);
        self
    }

    /// The parts, in load order.
    pub fn parts(&self) -> &[LoadablePart] {
        &self.parts
    }

    fn position_of(&self, object_index: ObjectIndex) -> Option<usize> {
        self.parts
            .iter()
            .position(|part| part.object_index == object_index)
    }
}

/// Why a plan could not be built. Every one of these is caught before a
/// procedure starts, which is the point of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanError {
    /// A download of nothing is not a download.
    NoParts,
    /// A part with no payload. Spec §9.3: the payload exists before the first
    /// write, or the procedure does not start.
    EmptyPart {
        /// The offending part.
        object_index: ObjectIndex,
    },
    /// Two parts claiming the same Interface Object, which would have the
    /// second silently overwrite the first.
    DuplicatePart {
        /// The index claimed twice.
        object_index: ObjectIndex,
    },
    /// An [`PartKind::ApplicationProgram1`] or [`PartKind::ApplicationProgram2`]
    /// part with no version. RES Table 90 (p. 288) / Table 91 (p. 290) list
    /// `PID_PROGRAM_VERSION` for these two objects and CP §3.5.2 Nr. 06/07
    /// (p. 43) requires the write, so a plan with nothing to write there
    /// cannot execute that step; a plan missing it is refused before the
    /// first write rather than found out about in `Loading`. Whether the
    /// property is itself mandatory is Volume 6 Annex A's question, not
    /// decided here (see C18).
    MissingVersion {
        /// The offending part.
        object_index: ObjectIndex,
    },
}

impl std::error::Error for PlanError {}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanError::NoParts => f.write_str("a download plan with no loadable parts in it"),
            PlanError::EmptyPart { object_index } => write!(
                f,
                "the part at {object_index} has no payload, and a download that cannot \
                 finish must not start (spec §9.3)"
            ),
            PlanError::DuplicatePart { object_index } => write!(
                f,
                "two parts both claim {object_index}, so one of them would be loaded over \
                 the other"
            ),
            PlanError::MissingVersion { object_index } => write!(
                f,
                "the application program at {object_index} has no version to write, and CP \
                 §3.5.2 Nr. 06/07 requires that write (RES Table 90/91 list the property there)"
            ),
        }
    }
}

/// One numbered step of one cited procedure, as it happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepRecord {
    /// Which procedure's numbering this step belongs to.
    pub kind: ProcedureKind,
    /// The step number inside that procedure.
    pub number: u8,
    /// What the step is called, taken from the step list rather than retyped.
    pub title: &'static str,
}

impl fmt::Display for StepRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {:02} {}", self.kind, self.number, self.title)
    }
}

/// What the CRC comparison of CP §3.5.3 found, when it was performed at all.
///
/// The comparison is RES §4.2.27, Table 12, p. 39's CRC field alone, not the
/// eight octets `PID_MCB_TABLE` answers with. Note what is *not* here: a
/// "differential download" outcome. The clause says *"If the CRC matches,
/// then MaC shall use differential download algorithm"* and that algorithm
/// is not specified in either knowledge base (spec §7.4, §12). So the
/// comparison is performed, reported, and not acted on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrcComparison {
    /// This procedure does not compare CRCs. A complete download replaces
    /// everything and has nothing to compare against.
    NotCompared,
    /// The plan carried no stored Memory Control Block for this part, so a
    /// previous download either never happened or skipped §7.2 step 7.
    NoStoredCrc,
    /// The device's current Memory Control Block equals the stored one.
    Matched,
    /// They differ, which is the ordinary case for a part being replaced.
    /// Also reported when either octet set does not fit RES §4.2.27, Table
    /// 12's eight octets: unparseable is not provably matching.
    Differed,
    /// RES §4.2.27.1.1, Table 13, p. 39, bit 0 of the CRC Control Byte is
    /// set: the device itself says the protected memory area may have
    /// changed since the load, so its CRC — matching or not — proves
    /// nothing.
    MayHaveChanged,
}

impl fmt::Display for CrcComparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CrcComparison::NotCompared => "not compared",
            CrcComparison::NoStoredCrc => "no stored CRC to compare against",
            CrcComparison::Matched => "matches the stored CRC",
            CrcComparison::Differed => "differs from the stored CRC",
            CrcComparison::MayHaveChanged => {
                "the device says the protected memory may have changed since the load, \
                 so its CRC cannot be trusted"
            }
        })
    }
}

/// What happened to the `PID_PROGRAM_VERSION` write for one part (`[C1]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionOutcome {
    /// The part carried no version, which is only valid for a part whose
    /// [`PartKind::has_program_version`] is false: `LoadablePart::new`
    /// refuses an empty version otherwise. CP §3.5.2's table steps 08/09/10
    /// (p. 44) ask for no such write, so a plan built for that procedure
    /// can leave a table's version empty to match it exactly.
    NotAttempted,
    /// The device accepted the write and read the octets back.
    Written(Vec<u8>),
    /// The device refused. Expected, and not a procedure failure, for the
    /// three table kinds: RES Table 77 (p. 238), Table 80 (p. 249) and
    /// Table 85 (p. 270) do not list `PID_PROGRAM_VERSION` for the Group
    /// Address Table, the Association Table or the Group Object Table.
    /// CP §3.5.3's three table variants ask for the write anyway
    /// (pp. 51-52, 54, 56); no clause reconciling the two was found by the
    /// audit behind this task (see C18). Both readings leave the part
    /// `Loaded`.
    ///
    /// `[D]` AL §3.4.4.2, p. 66, answers `nr_of_elem = 0` both when
    /// *"Interface Object or Property doesn't exist"* and when *"the
    /// requester does not have the required access rights"* — one shape,
    /// two causes. This
    /// variant cannot and does not distinguish them; it only records that
    /// the write did not take effect.
    Refused,
}

impl fmt::Display for VersionOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionOutcome::NotAttempted => f.write_str("no version write attempted"),
            VersionOutcome::Written(data) => write!(f, "version written: {data:02x?}"),
            VersionOutcome::Refused => f.write_str("version write refused"),
        }
    }
}

/// What happened to one loadable part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartOutcome {
    /// Which part.
    pub object_index: ObjectIndex,
    /// The base address `PID_TABLE_REFERENCE` handed back, which is never
    /// zero: zero is a failure and is reported as one.
    pub base_address: u32,
    /// How many chunks the payload went out in (spec §6.4).
    pub chunks: usize,
    /// The state the Load State Machine settled in.
    pub state: LoadState,
    /// What the CRC comparison found, if this procedure performs one.
    pub crc: CrcComparison,
    /// What happened to the `PID_PROGRAM_VERSION` write (`[C1]`).
    pub version: VersionOutcome,
    /// The Memory Control Block read after the load, whose CRC the next
    /// partial download will compare (spec §7.2 step 7).
    pub mcb: Vec<u8>,
}

/// What a procedure did, in enough detail to be checked against the clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadReport {
    /// Which procedure was run.
    pub kind: ProcedureKind,
    /// Every numbered step that happened, in order, including the inner §7.2
    /// loop's own steps.
    pub steps: Vec<StepRecord>,
    /// One entry per part the procedure touched.
    pub parts: Vec<PartOutcome>,
    /// Every `PID_ERROR_CODE` read, in the order it was read — which, in the
    /// recovery procedure, is before anything was unloaded.
    pub error_codes: Vec<(ObjectIndex, SystemErrorClass)>,
    /// The part whose failed allocation escalated a partial download into a
    /// reload of every following segment (CP §3.5.3's *"Continue at Nr. 07"*).
    pub escalated_from: Option<ObjectIndex>,
    /// The parts the recovery procedure had to unload because they were not
    /// `Loaded`.
    pub unloaded: Vec<ObjectIndex>,
}

impl DownloadReport {
    fn new(kind: ProcedureKind) -> Self {
        Self {
            kind,
            steps: Vec::new(),
            parts: Vec::new(),
            error_codes: Vec::new(),
            escalated_from: None,
            unloaded: Vec::new(),
        }
    }

    /// The step numbers of `kind`'s own procedure, in the order they ran —
    /// the inner loop's steps excluded, because they belong to §7.2's
    /// numbering and not to this one.
    pub fn outer_step_numbers(&self) -> Vec<u8> {
        self.steps
            .iter()
            .filter(|step| step.kind == self.kind)
            .map(|step| step.number)
            .collect()
    }

    /// The step numbers of the inner §7.2 loop, in the order they ran.
    pub fn inner_step_numbers(&self) -> Vec<u8> {
        self.steps
            .iter()
            .filter(|step| step.kind == ProcedureKind::LoadOnePart)
            .map(|step| step.number)
            .collect()
    }
}

/// Why a procedure stopped.
#[derive(Debug)]
pub enum DownloadError {
    /// The session refused, or the device did. Carries what was observed.
    Session(SessionError),
    /// The plan itself was not usable.
    Plan(PlanError),
    /// The device's mask has no allocation subtype this phase can build, or
    /// the one it profiles is not applicable (spec §7.3 design rules 1 to 3).
    Allocation(AllocationSubtypeError),
    /// CP §3.5.2 step 04's guard: the device is not from the manufacturer the
    /// application was built for. Nothing has been written at this point, and
    /// nothing will be.
    ManufacturerMismatch {
        /// What the plan expected.
        expected: u16,
        /// What `PID_MANUFACTURER_ID` answered.
        found: u16,
    },
    /// A part named in the plan was asked for by an index the plan has no
    /// part for.
    UnknownPart {
        /// The index asked about.
        object_index: ObjectIndex,
    },
    /// The confirmation step of spec §9.1 found a part that is still not
    /// `Loaded`. Reported, not retried.
    NotRecovered {
        /// Which part.
        object_index: ObjectIndex,
        /// What it reads instead.
        state: LoadState,
    },
}

impl std::error::Error for DownloadError {}

impl From<SessionError> for DownloadError {
    fn from(err: SessionError) -> Self {
        DownloadError::Session(err)
    }
}

impl From<PlanError> for DownloadError {
    fn from(err: PlanError) -> Self {
        DownloadError::Plan(err)
    }
}

impl From<AllocationSubtypeError> for DownloadError {
    fn from(err: AllocationSubtypeError) -> Self {
        DownloadError::Allocation(err)
    }
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadError::Session(err) => write!(f, "{err}"),
            DownloadError::Plan(err) => write!(f, "{err}"),
            DownloadError::Allocation(err) => write!(f, "{err}"),
            DownloadError::ManufacturerMismatch { expected, found } => write!(
                f,
                "the device reports manufacturer {found:04X}h where the application expects \
                 {expected:04X}h, so nothing was written (CP §3.5.2 step 04)"
            ),
            DownloadError::UnknownPart { object_index } => {
                write!(f, "this plan has no part at {object_index}")
            }
            DownloadError::NotRecovered {
                object_index,
                state,
            } => write!(
                f,
                "recovery left {object_index} in {state} rather than Loaded, which is a \
                 failed recovery and is not retried in a loop (spec §9.1 step 6)"
            ),
        }
    }
}

/// Walks the cited procedures against one session.
///
/// Owns the plan and borrows the session, so the session stays the only thing
/// that talks to a device and this stays the only thing that decides in what
/// order.
#[derive(Debug)]
pub struct Downloader<'s, 't, T: ManagementTransport> {
    session: &'s mut ManagementSession<'t, T>,
    plan: DownloadPlan,
}

impl<'s, 't, T: ManagementTransport> Downloader<'s, 't, T> {
    /// A sequencer for one device and one plan.
    pub fn new(session: &'s mut ManagementSession<'t, T>, plan: DownloadPlan) -> Self {
        Self { session, plan }
    }

    /// The plan this sequencer will run.
    pub fn plan(&self) -> &DownloadPlan {
        &self.plan
    }

    /// CP §3.5.2, the complete download: unload everything, then load
    /// everything, then disconnect.
    ///
    /// Step 05 unloads every part *before* step 06 loads the first one, which
    /// is the clause's own order and means the device passes through a state
    /// with no valid configuration at all (spec §7.1, §9.3). That window is
    /// the reason §13 rates this procedure the way it does; it is not an
    /// implementation choice that could be tightened.
    pub async fn complete_download(&mut self) -> Result<DownloadReport, DownloadError> {
        let kind = ProcedureKind::CompleteDownload;
        let mut report = DownloadReport::new(kind);
        let (mask, limit) = open(self.session, &self.plan, kind, &mut report).await?;

        record(&mut report, kind, 5, "unload the device");
        for part in &self.plan.parts {
            self.session
                .write_load_event(part.object_index, LoadEvent::Unload)
                .await?;
        }

        for (position, part) in self.plan.parts.iter().enumerate() {
            // CP §3.5.2 numbers the five parts it lists 06 to 10. A plan of
            // five parts therefore reproduces the clause's numbering exactly,
            // and a longer one continues the count rather than stopping at 10.
            // The addition happens in `usize`: `6 + u8::try_from(position)`
            // overflowed at 250 parts, which is a panic in debug and a step
            // number of 0 in release.
            let number = u8::try_from(position + 6).unwrap_or(u8::MAX);
            record(&mut report, kind, number, "load one loadable part");
            let outcome = load_one_part(
                self.session,
                part,
                mask,
                self.plan.allocation_mode,
                limit,
                false,
                &mut report,
            )
            .await?;
            report.parts.push(outcome);
        }

        // Step 11 exists in the trace and does nothing: `A_Key_Write` is out
        // of scope for this phase (spec §10.7), and a step silently missing
        // from a trace is indistinguishable from a step that was forgotten.
        record(
            &mut report,
            kind,
            11,
            "modify access keys (not implemented)",
        );
        record(&mut report, kind, 12, "disconnect");
        self.session.disconnect().await;
        Ok(report)
    }

    /// CP §3.5.3's first variant, *"Partial Download of the 'application
    /// program 2'"*, numbered 01 to 14: unload one part, reload one part, and
    /// escalate to the following segments if its allocation fails.
    ///
    /// The escalation is the clause's own: *"⇒ Continue at Nr. 07"*, which is
    /// *"unload all the following segments"* and reload them in ascending
    /// order. A partial download can therefore turn into something very close
    /// to a full one while it is running, and the report says so
    /// ([`DownloadReport::escalated_from`]) rather than reporting a tidier
    /// story than what happened.
    pub async fn partial_download(
        &mut self,
        object_index: ObjectIndex,
    ) -> Result<DownloadReport, DownloadError> {
        let kind = ProcedureKind::PartialDownload;
        let position = self
            .plan
            .position_of(object_index)
            .ok_or(DownloadError::UnknownPart { object_index })?;
        let mut report = DownloadReport::new(kind);
        let (mask, limit) = open(self.session, &self.plan, kind, &mut report).await?;

        record(&mut report, kind, 5, "unload only the part being replaced");
        self.session
            .write_load_event(object_index, LoadEvent::Unload)
            .await?;

        record(&mut report, kind, 6, "allocate and compare the CRC");
        let attempt = load_one_part(
            self.session,
            &self.plan.parts[position],
            mask,
            self.plan.allocation_mode,
            limit,
            true,
            &mut report,
        )
        .await;

        match attempt {
            Ok(outcome) => report.parts.push(outcome),
            Err(DownloadError::Session(SessionError::AllocationFailed { .. })) => {
                record(&mut report, kind, 7, "on failed allocation, escalate");
                report.escalated_from = Some(object_index);
                // *"Unload all the following segments"* — and this part with
                // them: its own allocation just failed while it sat in
                // Loading, and §7.6 says only an unload frees that.
                for part in &self.plan.parts[position..] {
                    self.session
                        .write_load_event(part.object_index, LoadEvent::Unload)
                        .await?;
                }
                for part in &self.plan.parts[position..] {
                    let outcome = load_one_part(
                        self.session,
                        part,
                        mask,
                        self.plan.allocation_mode,
                        limit,
                        false,
                        &mut report,
                    )
                    .await?;
                    report.parts.push(outcome);
                }
            }
            Err(err) => return Err(err),
        }

        // Nr. 06 of this variant ends *"⇒ Continue at Nr. 13"*, so the
        // access keys and the disconnect are 13 and 14 and not 8 and 9. Nr. 13
        // is recorded and empty for the same reason it is in a complete
        // download: `A_Key_Write` is out of scope for this phase (spec §10.7),
        // and a step silently missing from a trace is indistinguishable from a
        // step that was forgotten.
        record(
            &mut report,
            kind,
            13,
            "modify access keys (not implemented)",
        );
        record(&mut report, kind, 14, "disconnect");
        self.session.disconnect().await;
        Ok(report)
    }

    /// CP §3.5.4 steps 01 to 06: unload every part and disconnect.
    ///
    /// Step 07 of that clause —
    /// `SerialNumber_IndividualAddress_Write(FFFFh)` by broadcast — is not
    /// implemented and not reachable from here. It makes a device
    /// unaddressable by design (spec §7.5, §13), and this phase builds the six
    /// steps before it and stops.
    pub async fn unload(&mut self) -> Result<DownloadReport, DownloadError> {
        let kind = ProcedureKind::Unload;
        let mut report = DownloadReport::new(kind);
        open(self.session, &self.plan, kind, &mut report).await?;

        record(&mut report, kind, 5, "unload every part");
        for part in &self.plan.parts {
            let state = self
                .session
                .write_load_event(part.object_index, LoadEvent::Unload)
                .await?;
            report.parts.push(PartOutcome {
                object_index: part.object_index,
                base_address: 0,
                chunks: 0,
                state,
                crc: CrcComparison::NotCompared,
                version: VersionOutcome::NotAttempted,
                mcb: Vec::new(),
            });
            report.unloaded.push(part.object_index);
        }

        record(&mut report, kind, 6, "disconnect");
        self.session.disconnect().await;
        Ok(report)
    }

    /// Spec §9.1, recovery after an interrupted download.
    ///
    /// The order of the first two reads is load-bearing: `PID_ERROR_CODE` is
    /// read for every part *before* anything is unloaded, because `[D]`
    /// RES §4.2.28 clears it when the state leaves `Error` and an unload would
    /// therefore destroy the only evidence of what went wrong.
    ///
    /// Step 5 re-runs the loading half of §7.1 for **every** part, including
    /// parts that read `Loaded`. That is the clause's own wording — *"Re-run
    /// the complete download of §7.1 from step 06"* — and it is also the safer
    /// reading: after an interrupted download the parts that survived may be
    /// the *old* project's parts, and a device holding some old tables and
    /// some new ones is consistent with nothing. The cost is real and is
    /// stated in §9.3: a part that reads `Loaded` is invalidated on the way.
    pub async fn recover(&mut self) -> Result<DownloadReport, DownloadError> {
        let kind = ProcedureKind::Recovery;
        let mut report = DownloadReport::new(kind);
        let (mask, limit) = open(self.session, &self.plan, kind, &mut report).await?;

        record(&mut report, kind, 2, "read every load state");
        let mut states = Vec::with_capacity(self.plan.parts.len());
        for part in &self.plan.parts {
            let state = self.session.read_load_state(part.object_index).await?;
            states.push((part.object_index, state));
        }

        record(
            &mut report,
            kind,
            3,
            "read PID_ERROR_CODE before unloading anything",
        );
        for (object_index, _) in &states {
            let class = self.session.read_error_code(*object_index).await?;
            report.error_codes.push((*object_index, class));
        }

        record(&mut report, kind, 4, "unload every part that is not Loaded");
        for (object_index, state) in &states {
            if *state == LoadState::Loaded {
                continue;
            }
            self.session
                .write_load_event(*object_index, LoadEvent::Unload)
                .await?;
            report.unloaded.push(*object_index);
        }

        record(
            &mut report,
            kind,
            5,
            "re-run the complete download from step 06",
        );
        for part in &self.plan.parts {
            let outcome = load_one_part(
                self.session,
                part,
                mask,
                self.plan.allocation_mode,
                limit,
                false,
                &mut report,
            )
            .await?;
            report.parts.push(outcome);
        }

        record(&mut report, kind, 6, "confirm every part reports Loaded");
        for part in &self.plan.parts {
            let state = self.session.read_load_state(part.object_index).await?;
            if state != LoadState::Loaded {
                return Err(DownloadError::NotRecovered {
                    object_index: part.object_index,
                    state,
                });
            }
        }

        self.session.disconnect().await;
        Ok(report)
    }
}

fn record(report: &mut DownloadReport, kind: ProcedureKind, number: u8, title: &'static str) {
    report.steps.push(StepRecord {
        kind,
        number,
        title,
    });
}

/// Steps 01 to 04, which every procedure in §7 and §9.1 opens with.
///
/// Returns the mask version and the write length, both of which every later
/// step depends on: the mask decides the allocation subtype (§7.3) and narrows
/// RES Table 94 (§5.4), and the length decides the chunking (§6.4).
async fn open<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    plan: &DownloadPlan,
    kind: ProcedureKind,
    report: &mut DownloadReport,
) -> Result<(MaskVersion, WriteLimit), DownloadError> {
    record(report, kind, 1, "connect");
    // One call, two numbered steps: `connect()` authorises as it connects
    // (spec §10.3), so Nr. 03 happens here rather than after Nr. 02.
    session.connect().await?;
    record(report, kind, 3, "get access rights");

    record(report, kind, 2, "verify the device version");
    let mask = session.read_mask_version().await?;
    session.adopt_mask(mask);

    // Not a numbered step in any clause, and required before the first write
    // by §6.4: how many octets this device will accept per memory write.
    let limit = session.write_limit(plan.router_object).await?;

    record(report, kind, 4, "check the manufacturer ID");
    let found = session.read_manufacturer_id().await?;
    if found != plan.expected_manufacturer_id {
        return Err(DownloadError::ManufacturerMismatch {
            expected: plan.expected_manufacturer_id,
            found,
        });
    }
    Ok((mask, limit))
}

/// §7.2, the inner loop, for one part.
///
/// `compare_crc` adds CP §3.5.3's Memory Control Block comparison between
/// steps 3 and 4. It changes what is reported and nothing about what is
/// written: a matching CRC is *not* used to skip the data write, because step
/// 05 of the partial download has already unloaded the part and `[D]` RES
/// Table 93 says the data is then *"undefined"* — so the octets a matching CRC
/// was computed over are no longer there to keep. The algorithm the clause
/// names for that case, *"differential download"*, is not specified anywhere in
/// either knowledge base (spec §7.4, §12).
async fn load_one_part<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    part: &LoadablePart,
    mask: MaskVersion,
    mode: AllocationMode,
    limit: WriteLimit,
    compare_crc: bool,
    report: &mut DownloadReport,
) -> Result<PartOutcome, DownloadError> {
    let kind = ProcedureKind::LoadOnePart;
    let object_index = part.object_index;

    record(report, kind, 1, "start");
    session
        .write_load_event(object_index, LoadEvent::StartLoading)
        .await?;

    record(report, kind, 2, "allocate");
    let payload = allocation_payload(mask, mode, part.data.len())?;
    session.write_allocation(object_index, payload).await?;

    record(report, kind, 3, "read back the base address");
    let base_address = session.read_table_reference(object_index).await?;

    let mut crc = CrcComparison::NotCompared;
    if compare_crc {
        let current = session.read_memory_control_block(object_index).await?;
        crc = compare_mcb(part.stored_mcb.as_deref(), &current);
    }

    record(report, kind, 4, "write the data");
    let chunks = session
        .write_memory_region(base_address, &part.data, limit)
        .await?;

    // `[C1]` RES Table 90, p. 288, and Table 91, p. 290, list
    // `PID_PROGRAM_VERSION` for the two application programs, and CP §3.5.2
    // Nr. 06/07, p. 43, requires this write for them; `LoadablePart::new`
    // already refused an application-program part with no version, so this
    // branch always attempts it and any refusal is a genuine procedure
    // failure there. Whether the property is itself mandatory there is
    // Volume 6 Annex A's question, not decided here (see C18). RES Table 77,
    // p. 238; Table 80, p. 249; Table 85, p. 270 do not list the property
    // for the three tables, so a plan may leave a table's version empty
    // (nothing attempted, matching CP §3.5.2 Nr. 08-10, p. 44, which asks
    // for no such write) or supply one anyway to match CP §3.5.3's table
    // variants (pp. 51-52, 54, 56) — in which case a refusal is recorded,
    // not propagated as an error.
    record(
        report,
        kind,
        5,
        if part.version.is_empty() {
            "no version to set (CP §3.5.2 Nr. 08-10, p. 44, lists none here)"
        } else {
            "set the version"
        },
    );
    let version = if part.version.is_empty() {
        VersionOutcome::NotAttempted
    } else {
        match session
            .write_property(
                object_index,
                PID_PROGRAM_VERSION,
                part.version.clone(),
                WriteScope::Download,
            )
            .await
        {
            Ok(read_back) => VersionOutcome::Written(read_back),
            Err(SessionError::PropertyRefused {
                object_index: refused_index,
                property_id: PID_PROGRAM_VERSION,
            }) if refused_index == object_index && !part.kind.has_program_version() => {
                VersionOutcome::Refused
            }
            Err(err) => return Err(err.into()),
        }
    };

    record(report, kind, 6, "complete");
    let state = session
        .write_load_event(object_index, LoadEvent::LoadCompleted)
        .await?;

    record(report, kind, 7, "store the checksum");
    let mcb = session.read_memory_control_block(object_index).await?;

    Ok(PartOutcome {
        object_index,
        base_address,
        chunks,
        state,
        crc,
        version,
        mcb,
    })
}

/// CP §3.5.3's Memory Control Block comparison, RES §4.2.27, Table 12, p. 39:
/// the CRC alone, not the segment size or the access nibbles that share the
/// same eight octets.
///
/// `stored` and `current_octets` both come from [`MemoryControlBlock::parse`];
/// a length that does not fit Table 12 is not a Memory Control Block this
/// crate can read a CRC out of, so it is treated the same as a genuine
/// mismatch — [`CrcComparison::Differed`] never claims a match it cannot
/// prove. RES §4.2.27.1.1, Table 13, p. 39, bit 0 of the *current* read is
/// checked first: when set, the device itself is saying the protected
/// memory may have changed since the load, so a matching CRC proves
/// nothing.
fn compare_mcb(stored: Option<&[u8]>, current_octets: &[u8]) -> CrcComparison {
    let Some(stored_octets) = stored else {
        return CrcComparison::NoStoredCrc;
    };
    let (Ok(stored_mcb), Ok(current_mcb)) = (
        MemoryControlBlock::parse(stored_octets),
        MemoryControlBlock::parse(current_octets),
    ) else {
        return CrcComparison::Differed;
    };
    if current_mcb.protected_memory_may_change() {
        return CrcComparison::MayHaveChanged;
    }
    if stored_mcb.crc == current_mcb.crc {
        CrcComparison::Matched
    } else {
        CrcComparison::Differed
    }
}

/// The allocation payload the device's mask profiles, or a refusal.
///
/// Design spec §7.3 rule 2: *"There is no fallback between allocation styles.
/// Pick by mask, or refuse."* The refusal path is
/// [`AllocationSubtypeError::MaskNotProfiled`], from `allocation_subtype_for`:
/// a mask whose PROF Table 7 row was not transcribed has no style to pick, and
/// inventing the remaining field layouts is what design spec §12's
/// `GAP-T30-02` forbids. `require_subtype` then re-checks the pick against the
/// mask's profile before anything is built, so the two tables cannot disagree
/// silently.
///
/// A part too large for the chosen subtype's size field is also a refusal
/// here. It used to be `u32::MAX`, which asked a device for four gigabytes and
/// meant it.
fn allocation_payload(
    mask: MaskVersion,
    mode: AllocationMode,
    length: usize,
) -> Result<LoadControlPayload, AllocationSubtypeError> {
    let subtype = allocation_subtype_for(mask)?;
    require_subtype(mask, subtype)?;
    match subtype {
        LoadControlSubtype::DataRelativeAllocation => {
            let size = u32::try_from(length).map_err(|_| AllocationSubtypeError::PartTooLarge {
                subtype,
                requested: length,
                field_octets: DATA_RELATIVE_ALLOCATION_SIZE_OCTETS,
            })?;
            Ok(data_relative_allocation(size, mode))
        }
        // `relative_allocation` performs its own two-octet check, because the
        // width of that field is MP §3.31.3.4's business and not this
        // function's.
        LoadControlSubtype::RelativeAllocation => relative_allocation(length),
        // `allocation_subtype_for` returns only the two subtypes above. If a
        // transcribed row ever names one of the other six, this is the same
        // refusal as a row that was never transcribed at all: no payload is
        // built for a layout this module does not carry.
        _ => Err(AllocationSubtypeError::MaskNotProfiled(mask)),
    }
}

#[cfg(test)]
mod tests {
    use super::super::simulator::{Interruption, Seen, SimulatedDevice, SimulatorConfig};
    use super::super::{ManagementSession, SessionTiming};
    use super::*;
    use knx_core::commissioning::authorisation::AuthorisationPlan;
    use knx_core::commissioning::load_control::MASK_0300;
    use knx_core::commissioning::mutation::WriteAuthorisation;
    use knx_core::commissioning::procedure::ProcedureKind;
    use knx_core::commissioning::properties::PID_LOAD_STATE_CONTROL;
    use std::time::Duration;

    /// The manufacturer the simulator reports, so the guard of CP §3.5.2
    /// step 04 passes when it is supposed to.
    const SIMULATED_MANUFACTURER: u16 = 0x0002;

    /// The cited timings with the waiting taken out: a procedure test proves
    /// the procedure's order, not that thirty seconds is thirty seconds.
    fn fast() -> SessionTiming {
        SessionTiming {
            connection_timeout: Duration::from_millis(50),
            response_timeout: Duration::from_millis(50),
            poll_interval: Duration::from_millis(1),
            max_transition: Duration::from_millis(40),
            programming_delay: Duration::from_millis(0),
        }
    }

    fn writer<'t>(
        device: &'t SimulatedDevice,
        scope: WriteScope,
    ) -> ManagementSession<'t, SimulatedDevice> {
        let authorisation = WriteAuthorisation::for_simulator(device.address(), scope)
            .expect("the simulated device is not an excluded address");
        ManagementSession::authorised(device, AuthorisationPlan::Skip, fast(), authorisation)
            .expect("a simulator authorisation against a simulator transport is accepted")
    }

    /// Object 3 in every fixture below is "Application Program 2"; this is
    /// the index a test must register with
    /// [`SimulatorConfig::application_program_objects`] for its version
    /// write to succeed rather than be tolerated as a refusal.
    const AP2_OBJECT: u8 = 3;

    fn part(index: u8, name: &str, length: usize, kind: PartKind) -> LoadablePart {
        let data = (0..length).map(|octet| octet as u8 ^ index).collect();
        LoadablePart::new(ObjectIndex::new(index), name, data, vec![0x01, 0x02], kind)
            .expect("a part with a payload and, for an application program, a version")
    }

    /// A table part with no version to write, the shape CP §3.5.2 Nr. 08-10
    /// (p. 44) actually asks for: it lists no `PID_PROGRAM_VERSION`
    /// write at all, unlike CP §3.5.3's table variants.
    fn table_part_with_no_version(
        index: u8,
        name: &str,
        length: usize,
        kind: PartKind,
    ) -> LoadablePart {
        let data = (0..length).map(|octet| octet as u8 ^ index).collect();
        LoadablePart::new(ObjectIndex::new(index), name, data, Vec::new(), kind)
            .expect("a table part with a payload and, correctly, no version")
    }

    fn plan(parts: Vec<LoadablePart>) -> DownloadPlan {
        DownloadPlan::new(SIMULATED_MANUFACTURER, parts).expect("a usable plan")
    }

    /// A device with `AP2_OBJECT` registered as an application program, on
    /// top of whatever else a test needs to configure, so that fixtures
    /// built from [`two_parts`] and friends can write its version without
    /// the tolerant `[C1]` path being what makes them pass. The single
    /// construction path for that registration, so no call site repeats
    /// the `HashSet` literal.
    fn ap2_device_with(config: SimulatorConfig) -> SimulatedDevice {
        SimulatedDevice::with_config(SimulatorConfig {
            application_program_objects: [AP2_OBJECT].into_iter().collect(),
            ..config
        })
    }

    fn ap2_device() -> SimulatedDevice {
        ap2_device_with(SimulatorConfig::default())
    }

    fn two_parts() -> DownloadPlan {
        plan(vec![
            part(
                3,
                "Application Program 2",
                20,
                PartKind::ApplicationProgram2,
            ),
            table_part_with_no_version(1, "Address Table", 9, PartKind::GroupAddressTable),
        ])
    }

    fn load_state_writes(device: &SimulatedDevice) -> Vec<(u8, u8)> {
        device
            .seen()
            .into_iter()
            .filter_map(|entry| match entry {
                Seen::PropertyWrite {
                    object_index,
                    property_id: PID_LOAD_STATE_CONTROL,
                    data,
                } => Some((object_index, data.first().copied().unwrap_or(0xFF))),
                _ => None,
            })
            .collect()
    }

    /// The whole of CP §3.5.2, in the clause's own order, with the two
    /// documented deviations visible: 03 before 02 because the session
    /// authorises as it connects, and no step 07 of §7.5 anywhere.
    #[tokio::test]
    async fn a_complete_download_walks_cp_3_5_2_in_the_clauses_order() {
        let device = ap2_device();
        let mut session = writer(&device, WriteScope::Download);
        let report = Downloader::new(&mut session, two_parts())
            .complete_download()
            .await
            .expect("a working simulator completes a download");

        assert_eq!(report.kind, ProcedureKind::CompleteDownload);
        assert_eq!(
            report.outer_step_numbers(),
            vec![1, 3, 2, 4, 5, 6, 7, 11, 12],
            "trace was {:?}",
            report.steps
        );
        // Two parts, seven inner steps each, and none of them skipped.
        assert_eq!(
            report.inner_step_numbers(),
            vec![1, 2, 3, 4, 5, 6, 7, 1, 2, 3, 4, 5, 6, 7]
        );
        assert!(report
            .parts
            .iter()
            .all(|outcome| outcome.state == LoadState::Loaded));
        assert!(report
            .parts
            .iter()
            .all(|outcome| outcome.base_address != 0 && outcome.chunks > 0));
        assert!(report
            .parts
            .iter()
            .all(|outcome| outcome.crc == CrcComparison::NotCompared));
        // `[C1]` CP §3.5.2 Nr. 06/07 (p. 43) writes the version for the
        // application program; Nr. 09 (p. 44) lists no such write for the
        // Address Table, so this plan's table part carries none.
        assert!(matches!(
            report.parts[0].version,
            VersionOutcome::Written(_)
        ));
        assert_eq!(report.parts[1].version, VersionOutcome::NotAttempted);
        // `[C1]` F7: the outcome alone doesn't pin the step-5 label a future
        // edit could collapse back to one unconditional string; assert the
        // two recorded titles by their exact text, not `.contains("version")`
        // (which both would satisfy).
        let step_5_titles: Vec<&str> = report
            .steps
            .iter()
            .filter(|step| step.kind == ProcedureKind::LoadOnePart && step.number == 5)
            .map(|step| step.title)
            .collect();
        assert_eq!(
            step_5_titles,
            vec![
                "set the version",
                "no version to set (CP §3.5.2 Nr. 08-10, p. 44, lists none here)",
            ]
        );

        // §7.1 step 05: everything is unloaded before anything is loaded.
        let events = load_state_writes(&device);
        let first_start = events
            .iter()
            .position(|(_, event)| *event == LoadEvent::StartLoading.octet())
            .expect("a start-loading event");
        let unloads = events
            .iter()
            .take(first_start)
            .filter(|(_, event)| *event == LoadEvent::Unload.octet())
            .count();
        assert_eq!(unloads, 2, "both parts are unloaded before the first load");
    }

    /// CP §3.5.2 step 04 is a guard, and a guard that fails leaves the device
    /// exactly as it was.
    #[tokio::test]
    async fn the_manufacturer_guard_stops_before_a_single_write() {
        let device = SimulatedDevice::new();
        let mut session = writer(&device, WriteScope::Download);
        let wrong = DownloadPlan::new(
            0x00FF,
            vec![part(
                3,
                "Application Program 2",
                4,
                PartKind::ApplicationProgram2,
            )],
        )
        .expect("a usable plan");
        let error = Downloader::new(&mut session, wrong)
            .complete_download()
            .await
            .expect_err("a foreign manufacturer ID stops the download");

        assert!(
            matches!(
                error,
                DownloadError::ManufacturerMismatch {
                    expected: 0x00FF,
                    found: SIMULATED_MANUFACTURER
                }
            ),
            "got {error}"
        );
        assert!(!device.memory_was_written());
        assert!(
            load_state_writes(&device).is_empty(),
            "no Load State Machine was touched: {:?}",
            device.seen()
        );
    }

    /// §2.3 again, one layer up: a sequencer is not a second door. A session
    /// with no authorisation value runs no procedure that writes.
    #[tokio::test]
    async fn a_read_only_session_cannot_be_sequenced_into_writing() {
        let device = SimulatedDevice::new();
        let mut session = ManagementSession::read_only(
            &device,
            device.address(),
            AuthorisationPlan::Skip,
            fast(),
        )
        .expect("the simulated device is contactable");
        let error = Downloader::new(&mut session, two_parts())
            .complete_download()
            .await
            .expect_err("a read-only session has nothing to write with");

        assert!(
            matches!(
                error,
                DownloadError::Session(SessionError::NoAuthorisation { .. })
            ),
            "got {error}"
        );
        assert!(!device.memory_was_written());
        assert!(load_state_writes(&device).is_empty());
    }

    /// §14 item 8: the download is interrupted at every step of §7.2 in turn,
    /// and §9.1 recovers from each one — with `PID_ERROR_CODE` read before
    /// anything is unloaded, because an unload destroys it.
    #[tokio::test]
    async fn an_interruption_at_every_step_of_the_inner_loop_is_recovered() {
        for step in Interruption::ALL {
            let device = ap2_device_with(SimulatorConfig {
                interrupt_at: Some(step),
                ..SimulatorConfig::default()
            });

            let mut session = writer(&device, WriteScope::Download);
            let interrupted = Downloader::new(&mut session, two_parts())
                .complete_download()
                .await;
            assert!(
                interrupted.is_err(),
                "an interruption at {step:?} must not report success"
            );

            // A new connection, as a client that lost one would open. The
            // device kept its load states across it: `[D]` RES §4.23.1 stores
            // them in non-volatile memory, so this is the situation §9.1 is
            // about and not a fresh device.
            let mut recovery = writer(&device, WriteScope::Download);
            let report = Downloader::new(&mut recovery, two_parts())
                .recover()
                .await
                .unwrap_or_else(|err| panic!("recovery after {step:?} failed: {err}"));

            assert_eq!(report.kind, ProcedureKind::Recovery);
            assert_eq!(
                report.outer_step_numbers(),
                vec![1, 3, 2, 4, 2, 3, 4, 5, 6],
                "trace after {step:?} was {:?}",
                report.steps
            );
            for part in two_parts().parts() {
                assert_eq!(
                    device.load_state(part.object_index()),
                    LoadState::Loaded,
                    "{} is not Loaded after recovering from {step:?}",
                    part.name()
                );
            }

            // The order that matters: every error-code read happened before
            // the first unload of the recovery.
            let reads = recovery_error_code_reads(&device);
            assert_eq!(
                reads.len(),
                2,
                "one error-code read per part, after {step:?}"
            );
            if let Some(first_unload) = first_recovery_unload(&device) {
                assert!(
                    reads.iter().all(|at| *at < first_unload),
                    "an error code was read after the unload that clears it, at {step:?}"
                );
            }
            assert_eq!(report.error_codes.len(), 2);
        }
    }

    /// Where the recovery session's `PID_ERROR_CODE` reads sit in the frame
    /// log, counted from the reconnect that started the recovery.
    fn recovery_error_code_reads(device: &SimulatedDevice) -> Vec<usize> {
        let seen = device.seen();
        let start = last_connect(&seen);
        seen.iter()
            .enumerate()
            .skip(start)
            .filter(|(_, entry)| {
                matches!(
                    entry,
                    Seen::PropertyRead {
                        property_id: knx_core::commissioning::properties::PID_ERROR_CODE,
                        ..
                    }
                )
            })
            .map(|(at, _)| at)
            .collect()
    }

    fn first_recovery_unload(device: &SimulatedDevice) -> Option<usize> {
        let seen = device.seen();
        let start = last_connect(&seen);
        seen.iter()
            .enumerate()
            .skip(start)
            .position(|(_, entry)| {
                matches!(
                    entry,
                    Seen::PropertyWrite {
                        property_id: PID_LOAD_STATE_CONTROL,
                        data,
                        ..
                    } if data.first() == Some(&LoadEvent::Unload.octet())
                )
            })
            .map(|at| at + start)
    }

    fn last_connect(seen: &[Seen]) -> usize {
        seen.iter()
            .rposition(|entry| matches!(entry, Seen::Connect))
            .unwrap_or(0)
    }

    /// §14 item 10: a failed allocation does not abort. CP §3.5.3's own
    /// recovery is *"⇒ Continue at Nr. 07"*, which unloads every following
    /// segment and reloads them in ascending order — turning a partial
    /// download into very nearly a full one.
    #[tokio::test]
    async fn a_failed_allocation_escalates_to_every_following_segment() {
        let device = ap2_device_with(SimulatorConfig {
            allocation_fails_once_for: Some(3),
            ..SimulatorConfig::default()
        });
        let parts = plan(vec![
            part(
                3,
                "Application Program 2",
                16,
                PartKind::ApplicationProgram2,
            ),
            part(1, "Address Table", 8, PartKind::GroupAddressTable),
            part(2, "Association Table", 6, PartKind::AssociationTable),
        ]);
        let mut session = writer(&device, WriteScope::Download);
        let report = Downloader::new(&mut session, parts)
            .partial_download(ObjectIndex::new(3))
            .await
            .expect("the escalation completes the download it turned into");

        assert_eq!(report.escalated_from, Some(ObjectIndex::new(3)));
        assert_eq!(
            report.outer_step_numbers(),
            vec![1, 3, 2, 4, 5, 6, 7, 13, 14],
            "steps 08 to 12 are the escalation's own reloads, recorded under \
             ProcedureKind::LoadOnePart rather than out here: {:?}",
            report.steps
        );
        // The failed attempt plus three successful loads.
        assert_eq!(report.parts.len(), 3);
        assert!(report
            .parts
            .iter()
            .all(|outcome| outcome.state == LoadState::Loaded));
        for index in [3u8, 1, 2] {
            assert_eq!(
                device.load_state(ObjectIndex::new(index)),
                LoadState::Loaded,
                "object {index} is not Loaded after the escalation"
            );
        }
    }

    /// A CRC that matches is reported and is *not* used to skip the write:
    /// step 05 has already unloaded the part, so `[D]` RES Table 93 has
    /// declared the data undefined, and the algorithm CP §3.5.3 names for
    /// this case is not specified anywhere (spec §7.4, §12).
    #[tokio::test]
    async fn a_matching_crc_is_reported_and_the_data_is_written_anyway() {
        let device = ap2_device();
        let stored = device.mcb(ObjectIndex::new(3));
        let parts = plan(vec![
            part(
                3,
                "Application Program 2",
                12,
                PartKind::ApplicationProgram2,
            )
            .with_stored_mcb(stored),
            part(1, "Address Table", 5, PartKind::GroupAddressTable),
        ]);
        let mut session = writer(&device, WriteScope::Download);
        let report = Downloader::new(&mut session, parts)
            .partial_download(ObjectIndex::new(3))
            .await
            .expect("a partial download of one part");

        assert_eq!(report.parts.len(), 1);
        let outcome = &report.parts[0];
        assert_eq!(outcome.crc, CrcComparison::Matched);
        assert!(outcome.chunks > 0, "the data went out regardless");
        assert!(device.memory_was_written());
        // The other part was never touched: that is what makes it partial.
        assert_eq!(
            device.load_state(ObjectIndex::new(1)),
            LoadState::Unloaded,
            "a partial download loaded a part it was not asked about"
        );
    }

    /// The ordinary case, and the one where the plan has nothing to compare.
    #[tokio::test]
    async fn a_crc_either_differs_or_was_never_stored() {
        let device = ap2_device();
        let parts = plan(vec![part(
            3,
            "Application Program 2",
            6,
            PartKind::ApplicationProgram2,
        )
        .with_stored_mcb(vec![0xDE, 0xAD])]);
        let mut session = writer(&device, WriteScope::Download);
        let differed = Downloader::new(&mut session, parts)
            .partial_download(ObjectIndex::new(3))
            .await
            .expect("a partial download of one part");
        assert_eq!(differed.parts[0].crc, CrcComparison::Differed);

        let device = ap2_device();
        let mut session = writer(&device, WriteScope::Download);
        let unknown = Downloader::new(
            &mut session,
            plan(vec![part(
                3,
                "Application Program 2",
                6,
                PartKind::ApplicationProgram2,
            )]),
        )
        .partial_download(ObjectIndex::new(3))
        .await
        .expect("a partial download of one part");
        assert_eq!(unknown.parts[0].crc, CrcComparison::NoStoredCrc);
    }

    /// RES §4.2.27, Table 12, p. 39: Segment Size is octets 0-3, not the
    /// CRC. A partial download allocates before comparing
    /// (`load_one_part` step 2 precedes step 3's MCB read), so a changed
    /// payload length changes the device's current Segment Size on every
    /// partial download — that must not turn into `Differed`.
    #[tokio::test]
    async fn a_changed_segment_size_with_the_same_crc_still_matches() {
        let device = ap2_device();
        let stored = MemoryControlBlock {
            segment_size: 12,
            crc_control_byte: 0,
            read_access: 0,
            write_access: 0,
            crc: 0xBEEF,
        }
        .to_octets();
        // The allocation this same download performs picks a different
        // segment size (the part below is 40 octets, not 12) but the same
        // CRC — this is what step 2 having already run before step 3 reads
        // looks like on the wire.
        let current = MemoryControlBlock {
            segment_size: 40,
            crc_control_byte: 0,
            read_access: 0xF,
            write_access: 0xF,
            crc: 0xBEEF,
        }
        .to_octets();
        device.preset_mcb(ObjectIndex::new(3), &current);
        let parts = plan(vec![part(
            3,
            "Application Program 2",
            40,
            PartKind::ApplicationProgram2,
        )
        .with_stored_mcb(stored.to_vec())]);
        let mut session = writer(&device, WriteScope::Download);
        let report = Downloader::new(&mut session, parts)
            .partial_download(ObjectIndex::new(3))
            .await
            .expect("a partial download of one part");
        assert_eq!(report.parts[0].crc, CrcComparison::Matched);
    }

    /// RES §4.2.27.1.1, Table 13, p. 39, bit 0: when the device's current
    /// read sets it, the CRC comparison must not report `Matched` even if
    /// the CRC octets are numerically equal — the device is saying the
    /// comparison cannot be trusted.
    #[tokio::test]
    async fn crc_control_byte_bit_0_set_reports_may_have_changed_regardless_of_the_crc() {
        let device = ap2_device();
        let stored = MemoryControlBlock {
            segment_size: 4,
            crc_control_byte: 0,
            read_access: 0,
            write_access: 0,
            crc: 0x1234,
        }
        .to_octets();
        let current = MemoryControlBlock {
            segment_size: 4,
            crc_control_byte: 0b0000_0001,
            read_access: 0,
            write_access: 0,
            crc: 0x1234,
        }
        .to_octets();
        device.preset_mcb(ObjectIndex::new(3), &current);
        let parts = plan(vec![part(
            3,
            "Application Program 2",
            4,
            PartKind::ApplicationProgram2,
        )
        .with_stored_mcb(stored.to_vec())]);
        let mut session = writer(&device, WriteScope::Download);
        let report = Downloader::new(&mut session, parts)
            .partial_download(ObjectIndex::new(3))
            .await
            .expect("a partial download of one part");
        assert_eq!(report.parts[0].crc, CrcComparison::MayHaveChanged);
    }

    /// [`compare_mcb`] directly: the same two behaviours as the integration
    /// tests above, without a session in the way, so a broken comparison
    /// cannot hide behind an unrelated procedure failure.
    #[test]
    fn compare_mcb_reports_the_crc_field_alone() {
        let stored = MemoryControlBlock {
            segment_size: 12,
            crc_control_byte: 0,
            read_access: 0,
            write_access: 0,
            crc: 0xBEEF,
        }
        .to_octets();
        let current = MemoryControlBlock {
            segment_size: 999,
            crc_control_byte: 0,
            read_access: 0xF,
            write_access: 0xF,
            crc: 0xBEEF,
        }
        .to_octets();
        assert_eq!(compare_mcb(Some(&stored), &current), CrcComparison::Matched);

        let differing_crc = MemoryControlBlock {
            crc: 0xDEAD,
            ..MemoryControlBlock::parse(&current).unwrap()
        }
        .to_octets();
        assert_eq!(
            compare_mcb(Some(&stored), &differing_crc),
            CrcComparison::Differed
        );
    }

    #[test]
    fn compare_mcb_bit_0_outranks_a_matching_crc() {
        let stored = MemoryControlBlock {
            segment_size: 4,
            crc_control_byte: 0,
            read_access: 0,
            write_access: 0,
            crc: 0x1234,
        }
        .to_octets();
        let current = MemoryControlBlock {
            segment_size: 4,
            crc_control_byte: 0b0000_0001,
            read_access: 0,
            write_access: 0,
            crc: 0x1234,
        }
        .to_octets();
        assert_eq!(
            compare_mcb(Some(&stored), &current),
            CrcComparison::MayHaveChanged
        );
    }

    /// Unparseable octets on either side must not be read as a match: RES
    /// §4.2.27, Table 12, p. 39 fixes the width at eight octets, and a
    /// stored value from before this crate parsed the block (or garbage)
    /// gets the conservative verdict.
    #[test]
    fn compare_mcb_treats_unparseable_octets_as_differed() {
        let valid = MemoryControlBlock {
            segment_size: 0,
            crc_control_byte: 0,
            read_access: 0,
            write_access: 0,
            crc: 0,
        }
        .to_octets();
        assert_eq!(
            compare_mcb(Some(&[0xDE, 0xAD]), &valid),
            CrcComparison::Differed
        );
        assert_eq!(
            compare_mcb(Some(&valid), &[0xDE, 0xAD]),
            CrcComparison::Differed
        );
    }

    /// CP §3.5.4 steps 01 to 06, and no step 07: nothing here can make a
    /// device unaddressable, because nothing here broadcasts at all.
    #[tokio::test]
    async fn the_unload_procedure_stops_at_step_06() {
        let device = ap2_device();
        let mut session = writer(&device, WriteScope::Download);
        Downloader::new(&mut session, two_parts())
            .complete_download()
            .await
            .expect("a download to unload again");

        let mut session = writer(&device, WriteScope::Unload);
        let report = Downloader::new(&mut session, two_parts())
            .unload()
            .await
            .expect("the unload procedure");

        assert_eq!(report.outer_step_numbers(), vec![1, 3, 2, 4, 5, 6]);
        assert!(report
            .parts
            .iter()
            .all(|outcome| outcome.state == LoadState::Unloaded));
        let serial_number_services = device
            .seen()
            .into_iter()
            .filter(|entry| match entry {
                Seen::Other(name) => name.contains("SerialNumber") || name.contains("Individual"),
                _ => false,
            })
            .count();
        assert_eq!(
            serial_number_services, 0,
            "step 07 of CP §3.5.4 is not implemented and must not appear"
        );
    }

    /// Spec §7.3 design rules 1 and 3: the allocation subtype comes from the
    /// mask, there is no fallback, and a mask whose Table 7 row was not
    /// transcribed is a refusal rather than a guess.
    #[tokio::test]
    async fn a_mask_with_no_transcribed_allocation_subtype_is_refused() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0011,
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        let error = Downloader::new(&mut session, two_parts())
            .complete_download()
            .await
            .expect_err("an unprofiled mask has no allocation style to pick");
        assert!(matches!(error, DownloadError::Allocation(_)), "got {error}");
        assert!(!device.memory_was_written());
    }

    /// The `0300h` mask takes `0Ah` Relative Allocation, and the same
    /// sequencer produces it without a fallback anywhere in sight.
    #[tokio::test]
    async fn the_0300_mask_allocates_with_subtype_0a() {
        let device = ap2_device_with(SimulatorConfig {
            mask_version: MASK_0300.0,
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        Downloader::new(
            &mut session,
            plan(vec![part(
                3,
                "Application Program 2",
                6,
                PartKind::ApplicationProgram2,
            )]),
        )
        .complete_download()
        .await
        .expect("a 0300h device downloads too");

        let allocations: Vec<Vec<u8>> = device
            .seen()
            .into_iter()
            .filter_map(|entry| match entry {
                Seen::PropertyWrite {
                    property_id: PID_LOAD_STATE_CONTROL,
                    data,
                    ..
                } if data.first() == Some(&LoadEvent::AdditionalLoadControls.octet()) => Some(data),
                _ => None,
            })
            .collect();
        assert_eq!(allocations.len(), 1);
        assert_eq!(allocations[0][1], 0x0A, "0300h profiles 0Ah, not 0Bh");
        // `[D]` MP §3.31.3.4: subtype `0Ah` carries the size in **two**
        // octets and pads with six fill octets, so a 6-octet part is
        // `03 0A 00 06` followed by zeroes.
        assert_eq!(
            allocations[0],
            vec![0x03, 0x0A, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            "the requested size is two octets, most significant first"
        );
    }

    /// `[D]` MP §3.31.3.4 gives subtype `0Ah` a two-octet size field, so a
    /// part of more than `FFFFh` octets cannot be asked for at all on a
    /// `0300h` device. It is refused before the first write, not truncated
    /// into an allocation of the wrong size.
    #[tokio::test]
    async fn a_part_too_large_for_the_0300_size_field_is_refused_before_any_write() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: MASK_0300.0,
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        let error = Downloader::new(
            &mut session,
            plan(vec![part(
                3,
                "Application Program 2",
                0x1_0000,
                PartKind::ApplicationProgram2,
            )]),
        )
        .complete_download()
        .await
        .expect_err("65536 octets do not fit two octets");
        assert!(
            matches!(
                error,
                DownloadError::Allocation(AllocationSubtypeError::PartTooLarge {
                    field_octets: 2,
                    ..
                })
            ),
            "got {error}"
        );
        assert!(!device.memory_was_written());
    }

    /// Spec §9.3: a plan that could not be finished is refused before it
    /// starts, and two parts cannot share one Interface Object.
    #[test]
    fn a_plan_refuses_what_it_could_not_finish() {
        let empty = LoadablePart::new(
            ObjectIndex::new(3),
            "Application Program 2",
            Vec::new(),
            vec![],
            PartKind::ApplicationProgram2,
        );
        assert!(matches!(empty, Err(PlanError::EmptyPart { .. })));

        let no_version = LoadablePart::new(
            ObjectIndex::new(3),
            "Application Program 2",
            vec![0x01],
            Vec::new(),
            PartKind::ApplicationProgram2,
        );
        assert!(
            matches!(no_version, Err(PlanError::MissingVersion { .. })),
            "RES Table 90, p. 288 / Table 91, p. 290 list the version, and CP §3.5.2 \
             Nr. 06/07, p. 43, requires the write"
        );

        let duplicate = DownloadPlan::new(
            SIMULATED_MANUFACTURER,
            vec![
                part(3, "one", 4, PartKind::GroupAddressTable),
                part(3, "the same object", 4, PartKind::GroupAddressTable),
            ],
        );
        assert!(matches!(duplicate, Err(PlanError::DuplicatePart { .. })));

        assert!(matches!(
            DownloadPlan::new(SIMULATED_MANUFACTURER, Vec::new()),
            Err(PlanError::NoParts)
        ));
    }

    /// A part the plan never heard of is refused before the connection is
    /// opened, not halfway through a procedure.
    #[tokio::test]
    async fn a_partial_download_of_an_unknown_part_is_refused() {
        let device = SimulatedDevice::new();
        let mut session = writer(&device, WriteScope::Download);
        let error = Downloader::new(&mut session, two_parts())
            .partial_download(ObjectIndex::new(7))
            .await
            .expect_err("object 7 is not in this plan");
        assert!(
            matches!(error, DownloadError::UnknownPart { .. }),
            "got {error}"
        );
        assert!(device.seen().is_empty(), "not even a T_Connect was sent");
    }

    /// `[C1]` acceptance test: CP §3.5.3's Association Table variant, p. 56,
    /// asks for the version write that RES Table 80, p. 249, does not list
    /// for that object. A real device answers `PropertyRefused`, and both
    /// readings of the CP/RES contradiction leave the table `Loaded` — not
    /// stranded in `Loading`, which is what an unconditional write did.
    #[tokio::test]
    async fn a_partial_download_of_the_association_table_tolerates_a_refused_version() {
        // No `application_program_objects` registered: object 2 is not one
        // of them, so the simulator refuses the version write the way RES
        // Table 80, p. 249, says a real device would.
        let device = SimulatedDevice::new();
        let parts = plan(vec![part(
            2,
            "Association Table",
            6,
            PartKind::AssociationTable,
        )]);
        let mut session = writer(&device, WriteScope::Download);
        let report = Downloader::new(&mut session, parts)
            .partial_download(ObjectIndex::new(2))
            .await
            .expect("a table's refused version write is reported, not a procedure failure");

        assert_eq!(report.parts.len(), 1);
        assert_eq!(report.parts[0].version, VersionOutcome::Refused);
        assert_eq!(report.parts[0].state, LoadState::Loaded);
        assert_eq!(
            device.load_state(ObjectIndex::new(2)),
            LoadState::Loaded,
            "neither reading of the CP §3.5.3 / RES Table 80 contradiction strands this \
             table in Loading"
        );
    }

    /// `[C1]` simulator half: RES Table 77, p. 238; Table 80, p. 249;
    /// Table 85, p. 270 do not give `PID_PROGRAM_VERSION` to the Group
    /// Address Table, the Association Table or the Group Object Table. A
    /// device that was never told an object is one of the two application
    /// programs (RES Table 90, p. 288; Table 91, p. 290) must refuse a
    /// write of it, the same as `[D]` AL §3.4.4.2, p. 66, requires for a
    /// property that does not exist on that object.
    ///
    /// Before `[C1]`, `simulator.rs`'s fallback stored any
    /// `(object_index, property_id)` pair unconditionally, so this write
    /// would have succeeded — which is exactly what hid the defect this
    /// task fixes.
    #[tokio::test]
    async fn the_simulator_refuses_program_version_on_an_object_outside_tables_90_and_91() {
        let device = SimulatedDevice::new();
        let mut session = writer(&device, WriteScope::Download);
        session
            .connect()
            .await
            .expect("a working simulator accepts a connection");
        let error = session
            .write_property(
                ObjectIndex::new(1),
                PID_PROGRAM_VERSION,
                vec![0x01, 0x02],
                WriteScope::Download,
            )
            .await
            .expect_err("object 1 was never registered as an application program");
        assert!(
            matches!(
                error,
                SessionError::PropertyRefused {
                    object_index,
                    property_id: PID_PROGRAM_VERSION,
                } if object_index == ObjectIndex::new(1)
            ),
            "got {error}"
        );
    }

    /// `[C1]` F9: the `PropertyRefused` arm in `load_one_part` tolerates a
    /// refused version write only for a part whose
    /// [`PartKind::has_program_version`] is false. This is the negative half
    /// of `a_partial_download_of_the_association_table_tolerates_a_refused_
    /// version`: a refusal on an application-program part — which RES Table
    /// 90, p. 288, and Table 91, p. 290, both give the property — must still
    /// fail the procedure, not be swallowed the way a table's refusal is.
    /// `[D]` AL §3.4.4.2, p. 66, gives a real device no way to tell the two
    /// refusals apart on the wire, so only `part.kind` can.
    ///
    /// Deleting `&& !part.kind.has_program_version()` from that arm leaves
    /// this test the only one in the suite that notices: every other test
    /// either supplies a version the simulator accepts, or refuses on a
    /// table kind the guard was always meant to tolerate.
    #[tokio::test]
    async fn a_refused_version_write_on_an_application_program_fails_the_download() {
        // No `application_program_objects` registered for object 3: the
        // simulator refuses the version write for this application-program
        // part the same way it would refuse one for a table, and the two
        // must not be confused by the arm that tolerates the table's case.
        let device = SimulatedDevice::new();
        let parts = plan(vec![part(
            3,
            "Application Program 2",
            4,
            PartKind::ApplicationProgram2,
        )]);
        let mut session = writer(&device, WriteScope::Download);
        let error = Downloader::new(&mut session, parts)
            .partial_download(ObjectIndex::new(3))
            .await
            .expect_err(
                "a refused version write on an application program is a procedure failure, \
                 not something to tolerate",
            );
        assert!(
            matches!(
                error,
                DownloadError::Session(SessionError::PropertyRefused {
                    object_index,
                    property_id: PID_PROGRAM_VERSION,
                }) if object_index == ObjectIndex::new(3)
            ),
            "got {error}"
        );
    }
}
