//! The cited download, partial-download, unload and recovery procedures, driven step by step.
//!
//! Spec §11.2 asks for *"a procedure model, not a script"*, and `knx-core`'s
//! `commissioning::procedure` is that model: the step lists of CP §3.5.2,
//! CP §3.5.3, CP §3.5.4 and design spec §9.1, in the Standard's own numbering. This
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
//!   design spec §10.3 binds authorisation and Verify Mode to the connection rather
//!   than to the operation, so the descriptor read lands *after* the
//!   authorisation rather than before it. The trace records 01, 03, 02 in that
//!   order rather than pretending otherwise.
//! * CP §3.5.3 splits the inner loop across its Nr. 06 (*"allocate and compare
//!   the CRC"*) and its Nr. 08 (*"load the part"*). §7.2 is one loop, so the
//!   trace records it once, under Nr. 06, and Nr. 08 never appears.

use std::fmt;

use knx_core::commissioning::authorisation::AccessKeyDeclaration;
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
use knx_core::commissioning::partial_download_variant::PartialDownloadVariant;
use knx_core::commissioning::procedure::ProcedureKind;
use knx_core::commissioning::properties::{ObjectIndex, PID_PROGRAM_VERSION};

use super::{ManagementSession, SessionError};
use crate::management::ManagementTransport;

/// One loadable part, with everything needed to load it already in hand.
///
/// `[A]` The pairing of a part with an object index is this project's, not the
/// Standard's: CP §3.5.2 names its five parts (Application Program 2 and 1,
/// Group Object Table, Address Table, Association Table) and design spec §3.2 records
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
/// sequence. `[C8]` `DownloadPlan::new` checks a plan's parts against this
/// declaration order (via `Ord`) to enforce that download order — the
/// *memory layout* is a separate matter and is only ever a recommendation
/// (CP §3.5.1.3, p. 40).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
    /// which is the input CP §3.5.3's CRC comparison needs (design spec §7.2 step 7).
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
    /// What CP §3.5.2 Nr. 11 / CP §3.5.3 AP2 Nr. 13 should do (C10).
    /// Defaults to [`AccessKeyDeclaration::NoneRequired`], which is the
    /// state every plan built before this field existed was silently in.
    access_keys: AccessKeyDeclaration,
    /// The `PID_DOWNLOAD_COUNTER` this MaC stored after the preceding
    /// configuration (`[C13]`, CP §3.12.4, p. 99), read from the Device
    /// Object and therefore per-plan rather than per-part. `None` — the
    /// default — is not "unchanged"; see [`DownloadCounterCheck::NoStoredCounter`].
    stored_download_counter: Option<u16>,
}

impl DownloadPlan {
    /// A plan for one device.
    ///
    /// The parts are in the order they will be loaded, which is the order
    /// CP §3.5.2 steps 06 to 10 give for the five parts it names. The same
    /// order is used for the unload of step 05, because that clause's own
    /// order and its loading order agree on Application Program 2 first.
    ///
    /// `[C8]` That order is checked here, against [`PartKind`]'s declaration
    /// order, and a plan may skip kinds (a partial download need not carry
    /// all five) but may not present two it does carry out of the sequence
    /// CP §3.5.2 Nr. 06-10 and CP §3.5.3 AP2 Nr. 08-12 both give it. This is
    /// the *download* order, which is normative and carried only by row
    /// position in those two tables — not the memory *layout*, which
    /// CP §3.5.1.3, p. 40, calls a recommendation and explicitly allows to
    /// differ: *"it shall be possible to arrange the segments in different
    /// ways."* The reason this matters more than a cosmetic ordering: the
    /// escalation slice in [`Downloader::partial_download`]
    /// (`parts[position..]`) takes its entire target set from this order, so
    /// a plan built in the wrong order would escalate the wrong parts with
    /// no error at all — silently, and after the point where the caller
    /// could still fix it.
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
        // `[C8]` The download order is row position in CP §3.5.2 Nr. 06-10 and
        // CP §3.5.3 AP2 Nr. 08-12, both of which `PartKind`'s declaration
        // order reproduces. Two parts of the same kind have no defined
        // relative order in either table (each lists exactly one row per
        // kind), so `<=` rather than `<` refuses that case too instead of
        // guessing at an order the Standard never states.
        for window in parts.windows(2) {
            let (before, after) = (&window[0], &window[1]);
            if after.kind <= before.kind {
                return Err(PlanError::OutOfOrder {
                    object_index: after.object_index,
                    kind: after.kind,
                    preceding_kind: before.kind,
                });
            }
        }
        Ok(Self {
            expected_manufacturer_id,
            parts,
            allocation_mode: AllocationMode::default(),
            router_object: None,
            access_keys: AccessKeyDeclaration::default(),
            stored_download_counter: None,
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
    /// Object has no such property — which, per design spec §6.4, means 12 octets and
    /// not the value found there.
    pub fn with_router_object(mut self, router_object: ObjectIndex) -> Self {
        self.router_object = Some(router_object);
        self
    }

    /// Declares what CP §3.5.2 Nr. 11 / CP §3.5.3 AP2 Nr. 13 must do for
    /// this plan (C10). Left at the default
    /// [`AccessKeyDeclaration::NoneRequired`] when the plan needs nothing
    /// done there.
    pub fn with_access_keys(mut self, declaration: AccessKeyDeclaration) -> Self {
        self.access_keys = declaration;
        self
    }

    /// The `PID_DOWNLOAD_COUNTER` this MaC read and stored after the
    /// preceding configuration (`[C13]`, CP §3.12.4, p. 99). Left at the
    /// default `None` reports [`DownloadCounterCheck::NoStoredCounter`]
    /// rather than [`DownloadCounterCheck::Unchanged`] — a plan that never
    /// calls this has no baseline, not a baseline that happens to match.
    pub fn with_stored_download_counter(mut self, counter: u16) -> Self {
        self.stored_download_counter = Some(counter);
        self
    }

    /// The parts, in load order.
    pub fn parts(&self) -> &[LoadablePart] {
        &self.parts
    }

    /// What this plan declares about the access-key step (C10).
    pub fn access_keys(&self) -> &AccessKeyDeclaration {
        &self.access_keys
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
    /// A part precedes, in the plan's own order, a part CP §3.5.2 Nr. 06-10
    /// and CP §3.5.3 AP2 Nr. 08-12 both place *after* it in the download
    /// order. This is about the download order, which those two clauses'
    /// row position fixes; the memory *layout* is a different question and
    /// CP §3.5.1.3, p. 40, calls it only a recommendation. `[C8]` This is
    /// caught here rather than in [`Downloader::partial_download`] because
    /// that procedure's escalation (`parts[position..]`) trusts the plan's
    /// order completely: a plan that got the order wrong would escalate the
    /// wrong parts with no error at all.
    OutOfOrder {
        /// The part found out of place.
        object_index: ObjectIndex,
        /// Its kind.
        kind: PartKind,
        /// The kind of the part immediately before it in the plan, which
        /// the download order places *after* `kind`, not before it.
        preceding_kind: PartKind,
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
                 finish must not start (design spec §9.3)"
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
            PlanError::OutOfOrder {
                object_index,
                kind,
                preceding_kind,
            } => write!(
                f,
                "the part at {object_index} ({kind:?}) follows a {preceding_kind:?} part, but \
                 CP §3.5.2 Nr. 06-10 and CP §3.5.3 AP2 Nr. 08-12 both give {kind:?} an earlier \
                 row than {preceding_kind:?} in the download order — the target set an \
                 escalation would use is built from this order, so a wrong one is refused here \
                 rather than acted on (the memory layout itself is a separate, non-normative \
                 question: CP §3.5.1.3, p. 40)"
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
/// is not specified in either knowledge base (design spec §7.4, §12). So the
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

/// What CP §3.12.4/3.12.5's and RES §5.3.2.2's Download Counter check found
/// before a partial download (`[C13]`), read from `PID_DOWNLOAD_COUNTER` in
/// the Device Object — never a part's own object.
///
/// CP §3.5.3 itself — the five variants this module actually sequences —
/// imposes nothing about the Download Counter. Both clauses that do are
/// Coupler Model 2.0's own: CP §3.12.4/3.12.5, pp. 99-100, is that model's
/// Filter Table/Router Object download procedure, and RES §5.3.2.2, p. 320,
/// sits in RES §5.3 *"Resources for Coupler Model 2.0"* — the only place in
/// the whole of RES clause 5, *"Resources for Couplers"*, that says *"shall
/// not perform a Partial Download"*. There is no System B clause requiring
/// either behaviour. RES §4.2.30 sits in clause 4, *"Device Resources"*,
/// common to every device; it defines the property generically, and its
/// §4.2.30.3 advisory — *"should firstly read … may conclude"* — is not a
/// refusal obligation for anyone.
///
/// This module applies both Coupler Model 2.0 consequences to the one
/// generic CP §3.5.3 procedure it runs, for every part kind, regardless of
/// which device profile the target actually is (`[C18]`'s masks 07B0h and
/// 17B0h are System B, not Coupler Model 2.0). That is this project's own
/// conservative ruling — without a comparable counter the MaC cannot
/// establish the device is untouched since the last configuration, and
/// data integrity outranks convenience here — not compliance with a
/// Standard obligation that covers System B. Known Limitation §114 spells
/// out what this means in practice: a conformant System B device, which
/// Volume 6 Annex A never requires to carry this property (`[C18]`), is
/// refused every partial download and always gets a complete one.
///
/// The *changed* comparison is sound reading the Device Object's own
/// instance alone: RES §4.2.30.3, p. 42, is explicit that an unchanged
/// Device Object instance lets the client conclude no other instance
/// changed either. The *absent* refusal is not backed the same way —
/// RES §5.3.2.2, p. 320, says *"of the part to be downloaded"*, and
/// RES §4.2.30.1, p. 41, allows a downloadable part its own Download
/// Counter instance distinct from the Device Object's, or none at all.
/// Checking only the Device Object instance for absence is this module's
/// own simplification, not per-part RES §5.3.2.2 compliance — recorded in
/// §114 rather than implemented as a per-part lookup, because it errs
/// toward refusing more often, which is the safe direction.
///
/// `Downloader::partial_download` performs this check once per call,
/// before its first write, and refuses to proceed for two of the five
/// variants below — this is deliberately not shaped like [`CrcComparison`],
/// whose every variant lets the download continue: a stale checksum is
/// merely reported, `[C1]`'s `VersionOutcome::Refused` is tolerated, but a
/// stale or missing Download Counter stops the procedure.
///
/// The type keeps *absent* and *changed* apart on purpose: both currently
/// refuse the same way, but they are not the same fact. A device that never
/// had the property is conformant (Volume 6 Annex A, `[C18]`); one that had
/// it and now disagrees with this MaC's own record may have been touched by
/// someone else. Collapsing the two would report the second as the first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadCounterCheck {
    /// This procedure does not perform the check. `complete_download`,
    /// `unload` and `recover` replace or discard the part outright, so
    /// there is no partial download here for a stale counter to refuse.
    NotChecked,
    /// The plan carried no previously stored Download Counter for this
    /// device (`DownloadPlan::with_stored_download_counter` was never
    /// called). CP §3.12.4, p. 99, puts storing it on this application, not
    /// on the device — *"The MaC shall store the read value of the Download
    /// Counter in its repository"* — so a plan with nothing stored has a
    /// gap in this MaC's own bookkeeping, not one of the three outcomes the
    /// Standard itself describes. The current value is still read and
    /// carried here, and the partial download proceeds: refusing on a gap
    /// this application created, rather than one the device reported, would
    /// be inventing a fourth spec obligation instead of keeping to the
    /// three RES and CP actually state.
    NoStoredCounter(u16),
    /// Present and equal to the value stored after the preceding
    /// configuration. RES §4.2.30.3, p. 42: unchanged means the client "may
    /// conclude that no further instance of `PID_DOWNLOAD_COUNTER` … has
    /// changed value" either — the partial download proceeds.
    Unchanged(u16),
    /// Present but different from the stored value. CP §3.12.5, p. 100:
    /// *"If the read value of the Download Counter differs from the value
    /// that the MaC stored after the preceding configuration, then the MaC
    /// shall not continue with a partial download, but instead perform a
    /// complete download."*
    Changed {
        /// What this MaC had on record.
        stored: u16,
        /// What the device answers now.
        current: u16,
    },
    /// `PID_DOWNLOAD_COUNTER` is not implemented on this device at all —
    /// optional under Volume 6 Profiles Annex A, not a defect (`[C18]`).
    /// RES §5.3.2.2, p. 320: *"If PID_DOWNLOAD_COUNTER is not available for
    /// the part to be downloaded, then the MaC shall not perform a Partial
    /// Download."*
    Absent,
}

impl DownloadCounterCheck {
    /// Whether this outcome is one of CP §3.12.5's or RES §5.3.2.2's two
    /// refusals: [`Self::Changed`] and [`Self::Absent`] both stop
    /// `partial_download` before its first write, for their own clause and
    /// page each — see this type's own doc comment for why they stay two
    /// variants rather than one.
    pub fn refuses_partial_download(self) -> bool {
        matches!(self, Self::Changed { .. } | Self::Absent)
    }
}

impl fmt::Display for DownloadCounterCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadCounterCheck::NotChecked => f.write_str("not checked"),
            DownloadCounterCheck::NoStoredCounter(current) => write!(
                f,
                "no stored Download Counter to compare against; the device answers {current}"
            ),
            DownloadCounterCheck::Unchanged(value) => {
                write!(f, "unchanged at {value}")
            }
            DownloadCounterCheck::Changed { stored, current } => write!(
                f,
                "changed from {stored} to {current} since the preceding configuration"
            ),
            DownloadCounterCheck::Absent => f.write_str("not implemented on this device"),
        }
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
    /// (pp. 51-52, 54, 56). `[C18]` found the clause reconciling the two:
    /// RES §4.2.13.1.3, p. 34, defers per-object applicability to *"the
    /// Configuration Procedures in \[10\]"* — RES's reference list has
    /// `[10]` as Chapter 3/5/3, this project's own CP — *"and …
    /// \[17\]"* — Volume 6 Profiles — for "the mandatory - or optional
    /// access rights to Program Version", and Volume 6 Annex A makes the
    /// property optional rather than forbidden on the three tables, not
    /// mandatory the way CP §3.5.3 alone would suggest. Both readings leave
    /// the part `Loaded`.
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
    /// How many chunks the payload went out in (design spec §6.4).
    pub chunks: usize,
    /// The state the Load State Machine settled in.
    pub state: LoadState,
    /// What the CRC comparison found, if this procedure performs one.
    pub crc: CrcComparison,
    /// What happened to the `PID_PROGRAM_VERSION` write (`[C1]`).
    pub version: VersionOutcome,
    /// The Memory Control Block read after the load, whose CRC the next
    /// partial download will compare (design spec §7.2 step 7).
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
    /// What CP §3.12.4/3.12.5's and RES §4.2.30.3's Download Counter check
    /// found before this procedure ran (`[C13]`). [`DownloadCounterCheck::NotChecked`]
    /// for every procedure except `partial_download`, which is the only one
    /// CP §3.12.5's refusal applies to.
    pub download_counter: DownloadCounterCheck,
}

impl DownloadReport {
    fn new(kind: ProcedureKind) -> Self {
        Self {
            kind,
            steps: Vec::new(),
            parts: Vec::new(),
            error_codes: Vec::new(),
            download_counter: DownloadCounterCheck::NotChecked,
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
    /// the one it profiles is not applicable (design spec §7.3 design rules 1 to 3).
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
    /// The confirmation step of design spec §9.1 found a part that is still not
    /// `Loaded`. Reported, not retried.
    NotRecovered {
        /// Which part.
        object_index: ObjectIndex,
        /// What it reads instead.
        state: LoadState,
    },
    /// CP §3.5.2 Nr. 11 / CP §3.5.3 AP2 Nr. 13 (C10): the plan declared one
    /// or more access-key assignments, but `A_Key_Write` has no encoder
    /// (`cemi.rs`'s `key_write_has_an_apci_but_no_encoder`, design spec §10.7).
    /// Everything up to this step has already been written; only the key
    /// modification itself is refused, so the device is left on its
    /// current key rather than the plan's silently.
    AccessKeysNotSupported {
        /// How many levels the plan wanted (re)keyed.
        declared: usize,
    },
    /// `PID_DOWNLOAD_COUNTER` disagrees with the value this plan stored after
    /// the preceding configuration (`[C13]`). Nothing has been written yet —
    /// `partial_download` checks this before its unload step — so recovering
    /// is a plain `complete_download` call, not a `recover()`.
    ///
    /// CP §3.12.5, p. 100: *"If the read value of the Download Counter
    /// differs from the value that the MaC stored after the preceding
    /// configuration, then the MaC shall not continue with a partial
    /// download, but instead perform a complete download as specified in
    /// 3.12.4."*
    DownloadCounterChanged {
        /// What this MaC had on record.
        stored: u16,
        /// What the device answers now.
        current: u16,
    },
    /// `PID_DOWNLOAD_COUNTER` is not implemented on this device at all.
    /// Nothing has been written yet, for the same reason as
    /// [`Self::DownloadCounterChanged`].
    ///
    /// RES §5.3.2.2, p. 320: *"If PID_DOWNLOAD_COUNTER is not available for
    /// the part to be downloaded, then the MaC shall not perform a Partial
    /// Download."*
    DownloadCounterUnavailable,
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
                 failed recovery and is not retried in a loop (design spec §9.1 step 6)"
            ),
            DownloadError::AccessKeysNotSupported { declared } => write!(
                f,
                "the plan declares {declared} access-key assignment(s) at CP §3.5.2 Nr. 11 / \
                 CP §3.5.3 AP2 Nr. 13, but `A_Key_Write` has no encoder yet (design spec §10.7); the \
                 device is left on its current key rather than have this step reported done \
                 when it was not"
            ),
            DownloadError::DownloadCounterChanged { stored, current } => write!(
                f,
                "the download counter changed from {stored} to {current} since the preceding \
                 configuration, so the MaC shall not continue with a partial download, but \
                 instead perform a complete download as specified in 3.12.4 (CP §3.12.5, p. 100)"
            ),
            DownloadError::DownloadCounterUnavailable => write!(
                f,
                "PID_DOWNLOAD_COUNTER is not available for the part to be downloaded, so the \
                 MaC shall not perform a Partial Download (RES §5.3.2.2, p. 320)"
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
    /// with no valid configuration at all (design spec §7.1, §9.3). That window is
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

        // CP §3.5.2 Nr. 11, p. 44: *"Set access keys as required"*. The plan
        // says what "as required" means (C10); a plan declaring none is
        // reported as empty and a plan declaring some is refused here,
        // because `A_Key_Write` has no encoder (design spec §10.7).
        modify_access_keys(&mut report, kind, 11, &self.plan.access_keys)?;
        record(&mut report, kind, 12, "disconnect");
        self.session.disconnect().await;
        Ok(report)
    }

    /// CP §3.5.3's five partial-download variants (`[C11]`
    /// `partial_download_variant.rs`, transcribed pp. 44-56): unload one
    /// part, reload one part, and — for four of the five — escalate to the
    /// following segments if its allocation fails.
    ///
    /// `[C12]` The step numbers this method records come from the target
    /// part's own [`PartialDownloadVariant`], not from one literal numbering
    /// shared by all five: CP §3.5.3 numbers each of the five variants from
    /// 01 independently (`partial_download_variant.rs`'s own header), so the
    /// Association Table variant's tail sits at Nr. 07/08 while Application
    /// Program 2's sits at Nr. 13/14, and citing the wrong one is citing a
    /// step number that means something else in that clause.
    ///
    /// The escalation, where a variant has one, is the clause's own: *"⇒
    /// Continue at Nr. 07"*, which is *"unload all the following segments"*
    /// and reload them in ascending order. A partial download can therefore
    /// turn into something very close to a full one while it is running, and
    /// the report says so ([`DownloadReport::escalated_from`]) rather than
    /// reporting a tidier story than what happened. The Association Table
    /// variant is the one exception: CP §3.5.3, p. 56 gives its Nr. 06 no
    /// *"⇒ Continue"* and no escalation branch at all — *"if [Base Address]
    /// is zero then allocation was not successful. This causes an error
    /// message of the MaC to the Installer."* — so a failed allocation there
    /// is reported as a plain procedure failure, not retried.
    pub async fn partial_download(
        &mut self,
        object_index: ObjectIndex,
    ) -> Result<DownloadReport, DownloadError> {
        let kind = ProcedureKind::PartialDownload;
        let position = self
            .plan
            .position_of(object_index)
            .ok_or(DownloadError::UnknownPart { object_index })?;
        let variant = variant_for(self.plan.parts[position].kind());
        let procedure = variant.procedure();
        let mut report = DownloadReport::new(kind);
        let (mask, limit) = open(self.session, &self.plan, kind, &mut report).await?;
        check_download_counter(self.session, &self.plan, &mut report).await?;

        // Indices into `procedure.steps` below are fixed across all five
        // variants: the opening four (indices 0-3) plus unload/allocate
        // (4-5) are identical in shape everywhere, and only the Association
        // Table variant lacks indices 6-7 (the escalation announcement and
        // the target's own reload) — which is exactly the branch this
        // method never takes for it (`escalation_targets().is_empty()`
        // below).
        let unload_step = &procedure.steps[4];
        record(&mut report, kind, unload_step.number, unload_step.title);
        self.session
            .write_load_event(object_index, LoadEvent::Unload)
            .await?;

        let allocate_step = &procedure.steps[5];
        record(&mut report, kind, allocate_step.number, allocate_step.title);
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
            Err(DownloadError::Session(SessionError::AllocationFailed { .. }))
                if !variant.escalation_targets().is_empty() =>
            {
                let escalate_step = &procedure.steps[6];
                record(&mut report, kind, escalate_step.number, escalate_step.title);
                report.escalated_from = Some(object_index);
                // *"Unload all the following segments"* — CP §3.5.3 Nr. 07's
                // own escalation instruction, not a re-allocation retry: a
                // re-allocation also frees the previous memory and would
                // keep this part at its original base address (CP §3.5.1.2,
                // §7.6), but Nr. 07 is what the clause prescribes once
                // allocation has failed, so this part is unloaded along
                // with the ones after it rather than merely re-allocated.
                for part in &self.plan.parts[position..] {
                    self.session
                        .write_load_event(part.object_index, LoadEvent::Unload)
                        .await?;
                }
                // CP §3.5.3 AP2 variant Nr. 08, p. 46, is the reload of the
                // *target* part after escalation, and it carries the same
                // full "Compare CRC checksum" block as Nr. 06, p. 46 — Nr.
                // 08's text is Nr. 06's, word for word, down to the
                // `PID_MCB` read. This is the only reload in the escalation
                // that compares.
                let reload_target_step = &procedure.steps[7];
                record(
                    &mut report,
                    kind,
                    reload_target_step.number,
                    reload_target_step.title,
                );
                let outcome = load_one_part(
                    self.session,
                    &self.plan.parts[position],
                    mask,
                    self.plan.allocation_mode,
                    limit,
                    true,
                    &mut report,
                )
                .await?;
                report.parts.push(outcome);

                // `[C12]` Nr. 09-12, p. 47, reload the segments that merely
                // followed the target, one numbered outer step each — the
                // defect this fixes is exactly these steps going missing
                // from the trace (this module's own rule, stated twice:
                // `[C11]`'s header and Nr. 07's comment above). Each
                // follower's step number is looked up by its position in
                // the fixed download order relative to the target
                // (`PartKind`'s declaration order, `[C8]`), not by counting
                // this plan's own (possibly shorter) part list — a plan that
                // skips a kind still cites the clause's real number for the
                // kind it does carry, never a renumbered one. They carry no
                // CRC comparison block (`compare_crc = false`): they only
                // "Read and save CRC checksum" at the end, which this module
                // already records as [`CrcComparison::NotCompared`].
                let target_order = self.plan.parts[position].kind() as usize;
                for part in &self.plan.parts[position + 1..] {
                    let steps_after_target = part.kind() as usize - target_order - 1;
                    let reload_step = &procedure.steps[8 + steps_after_target];
                    record(&mut report, kind, reload_step.number, reload_step.title);
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

        // A successful Nr. 06 (or, on the escalation path, the reloads
        // above) continues at the second-to-last step of the variant's own
        // list, whichever number that is — CP §3.5.3 AP2 Nr. 13, p. 47, is
        // the same *"Set access keys as required"* text as CP §3.5.2
        // Nr. 11, so the same declaration and the same refusal apply (C10):
        // [`modify_access_keys`] is what makes both call sites report it
        // identically. This is deliberately not
        // [`PartialDownloadVariant::jump_target`]: that accessor is `None`
        // for the Association Table (CP §3.5.3, p. 56, has no escalation
        // branch for it to skip), but every variant, that one included,
        // still has an access-keys step and needs its number here — the
        // four variants where `jump_target()` is `Some` happen to agree
        // with this arithmetic, they do not supply it.
        let total_steps = procedure.steps.len();
        let access_keys_step = &procedure.steps[total_steps - 2];
        modify_access_keys(
            &mut report,
            kind,
            access_keys_step.number,
            &self.plan.access_keys,
        )?;
        let disconnect_step = &procedure.steps[total_steps - 1];
        record(
            &mut report,
            kind,
            disconnect_step.number,
            disconnect_step.title,
        );
        self.session.disconnect().await;
        Ok(report)
    }

    /// CP §3.5.4 steps 01 to 06: unload every part and disconnect.
    ///
    /// Step 07 of that clause —
    /// `SerialNumber_IndividualAddress_Write(FFFFh)` by broadcast — is not
    /// implemented and not reachable from here. It makes a device
    /// unaddressable by design (design spec §7.5, §13), and this phase builds the six
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

    /// KNXBench design spec §9.1, recovery after an interrupted download —
    /// no Standard equivalent. The Standard's own position is MP §3.1, p. 68:
    /// *"if an error is detected, the download shall be interrupted and an
    /// error-message shall be raised"* — no recovery procedure of its own.
    ///
    /// The order of the first two reads is load-bearing: `PID_ERROR_CODE` is
    /// read for every part *before* anything is unloaded, because `[D]`
    /// RES §4.2.28 clears it when the state leaves `Error` and an unload would
    /// therefore destroy the only evidence of what went wrong.
    ///
    /// Step 5 re-runs the loading half of §7.1 for **every** part, including
    /// parts that read `Loaded`. That is this design spec's own wording — not
    /// the Standard's — *"Re-run the complete download of §7.1 from step 06"*
    /// — and it is also the safer reading: after an interrupted download the
    /// parts that survived may be the *old* project's parts, and a device
    /// holding some old tables and some new ones is consistent with nothing.
    /// The cost is real and is stated in §9.3: a part that reads `Loaded` is
    /// invalidated on the way.
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

/// Which of CP §3.5.3's five partial-download variants (`[C11]`) a part's
/// `PartKind` corresponds to, so [`Downloader::partial_download`] can look up
/// that variant's own step numbering instead of hard-coding one variant's
/// numbers for every part. The two enums declare their five cases in the
/// same order for the same reason (`PartKind`'s own doc comment, `[C8]`), so
/// this is a name correspondence, not a second copy of any step data.
const fn variant_for(kind: PartKind) -> PartialDownloadVariant {
    match kind {
        PartKind::ApplicationProgram2 => PartialDownloadVariant::ApplicationProgram2,
        PartKind::ApplicationProgram1 => PartialDownloadVariant::ApplicationProgram1,
        PartKind::GroupObjectTable => PartialDownloadVariant::GroupObjectTable,
        PartKind::GroupAddressTable => PartialDownloadVariant::GroupAddressTable,
        PartKind::AssociationTable => PartialDownloadVariant::AssociationTable,
    }
}

/// CP §3.5.2 Nr. 11, p. 44, and CP §3.5.3 AP2 Nr. 13, p. 47 (C10): the
/// shared body of both "modify access keys" call sites, so that a complete
/// and a partial download report the same declaration in the same words
/// rather than two texts that could quietly drift apart.
///
/// A plan declaring [`AccessKeyDeclaration::NoneRequired`] is recorded and
/// nothing more happens, distinct from the old text this replaces, which
/// read identically whether the plan needed nothing or needed something
/// that got silently skipped. A plan declaring
/// [`AccessKeyDeclaration::Required`] is recorded as refused and the
/// procedure stops there: `A_Key_Write` has no encoder yet (design spec §10.7,
/// `cemi.rs`'s `key_write_has_an_apci_but_no_encoder`), so this project
/// cannot carry out what the plan is asking for, and does not pretend to.
fn modify_access_keys(
    report: &mut DownloadReport,
    kind: ProcedureKind,
    number: u8,
    declaration: &AccessKeyDeclaration,
) -> Result<(), DownloadError> {
    match declaration {
        AccessKeyDeclaration::NoneRequired => {
            record(
                report,
                kind,
                number,
                "modify access keys (declared: none required)",
            );
            Ok(())
        }
        AccessKeyDeclaration::Required(assignments) => {
            record(
                report,
                kind,
                number,
                "modify access keys (declared: refused, not implemented)",
            );
            Err(DownloadError::AccessKeysNotSupported {
                declared: assignments.len(),
            })
        }
    }
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
    // (design spec §10.3), so Nr. 03 happens here rather than after Nr. 02.
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

/// CP §3.12.4/3.12.5's and RES §5.3.2.2's Download Counter check (`[C13]`),
/// run once by [`Downloader::partial_download`], after [`open`] and before
/// its first write. Reads `PID_DOWNLOAD_COUNTER` from the Device Object,
/// never from the part being loaded — see [`DownloadCounterCheck`]'s own
/// doc comment for which half of that simplification is backed by a clause
/// and which is this module's own.
///
/// A `PropertyRefused` read is [`DownloadCounterCheck::Absent`], nothing
/// else is: a timeout or a disconnect during the read is not a conformant
/// device without the property, and stays whatever `SessionError` it was
/// via `?`. The decision to refuse is routed through
/// [`DownloadCounterCheck::refuses_partial_download`] rather than matched
/// again here, so the two stay in agreement by construction.
///
/// `report.download_counter` is set only for the two outcomes that let the
/// caller continue ([`DownloadCounterCheck::Unchanged`] and
/// [`DownloadCounterCheck::NoStoredCounter`]): `partial_download` never
/// returns `report` on `Err`, so a refusal is reported through the
/// returned [`DownloadError`] alone, and a value written here for either
/// refusal would be a write nobody can ever read.
async fn check_download_counter<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    plan: &DownloadPlan,
    report: &mut DownloadReport,
) -> Result<(), DownloadError> {
    let check = match session.read_download_counter().await {
        Ok(current) => match plan.stored_download_counter {
            None => DownloadCounterCheck::NoStoredCounter(current),
            Some(stored) if stored == current => DownloadCounterCheck::Unchanged(current),
            Some(stored) => DownloadCounterCheck::Changed { stored, current },
        },
        Err(SessionError::PropertyRefused { .. }) => DownloadCounterCheck::Absent,
        Err(err) => return Err(err.into()),
    };
    if check.refuses_partial_download() {
        return Err(match check {
            DownloadCounterCheck::Changed { stored, current } => {
                DownloadError::DownloadCounterChanged { stored, current }
            }
            DownloadCounterCheck::Absent => DownloadError::DownloadCounterUnavailable,
            // `refuses_partial_download` returns `true` for exactly these
            // two variants; see its own doc comment.
            _ => unreachable!("refuses_partial_download() only allows Changed and Absent here"),
        });
    }
    report.download_counter = check;
    Ok(())
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
/// either knowledge base (design spec §7.4, §12).
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
    use knx_core::commissioning::authorisation::{
        AccessKey, AccessKeyAssignment, AccessKeyDeclaration, AccessLevel, AuthorisationPlan,
    };
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
            restart_basic_t1: Duration::from_millis(1),
            restart_responsive_again: Duration::from_millis(5),
            post_restart_disconnect_wait: Duration::from_millis(60),
            programming_mode_broadcast_timeout: Duration::from_millis(20),
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

    /// A test fixture for C10: one assignment, so `Required` is never
    /// empty here.
    fn one_access_key_assignment() -> AccessKeyAssignment {
        AccessKeyAssignment {
            level: AccessLevel::from_octet(2),
            key: AccessKey::new(0x1122_3344).unwrap(),
        }
    }

    /// C10, direct: [`modify_access_keys`] is the one function both call
    /// sites share, so this is where the defect the task describes — "the
    /// report reads identically whether none were needed or some were
    /// silently skipped" — is tested at its source, without a simulator in
    /// the way. `NoneRequired` records a step and returns `Ok`; `Required`
    /// records a *different* step and returns the named error, and the two
    /// recorded titles must not read the same.
    #[test]
    fn modify_access_keys_reports_declared_none_and_refused_differently() {
        let mut none_report = DownloadReport::new(ProcedureKind::CompleteDownload);
        modify_access_keys(
            &mut none_report,
            ProcedureKind::CompleteDownload,
            11,
            &AccessKeyDeclaration::NoneRequired,
        )
        .expect("declaring none required does not refuse the step");
        let none_title = none_report.steps.last().expect("step 11 recorded").title;

        let mut required_report = DownloadReport::new(ProcedureKind::CompleteDownload);
        let declaration =
            AccessKeyDeclaration::required(vec![one_access_key_assignment()]).unwrap();
        let error = modify_access_keys(
            &mut required_report,
            ProcedureKind::CompleteDownload,
            11,
            &declaration,
        )
        .expect_err("declaring a key that cannot be written must refuse, not succeed");
        assert!(
            matches!(error, DownloadError::AccessKeysNotSupported { declared: 1 }),
            "got {error}"
        );
        let required_title = required_report
            .steps
            .last()
            .expect("step 11 recorded even though it is refused")
            .title;

        assert_ne!(
            none_title, required_title,
            "a plan needing nothing and a plan needing an unimplemented write must not \
             read the same"
        );
    }

    /// C10: the default declaration is `NoneRequired`, and step 11 says so
    /// rather than reading the same as it would if a key had been silently
    /// skipped (CP §3.5.2 Nr. 11, p. 44).
    #[tokio::test]
    async fn a_plan_declaring_no_access_keys_reports_step_11_as_declared_none() {
        let device = ap2_device();
        let mut session = writer(&device, WriteScope::Download);
        let report = Downloader::new(&mut session, two_parts())
            .complete_download()
            .await
            .expect("a plan declaring nothing at step 11 completes");

        let step_11 = report
            .steps
            .iter()
            .find(|step| step.number == 11 && step.kind == ProcedureKind::CompleteDownload)
            .expect("step 11 is recorded even though it does nothing");
        assert!(
            step_11.title.contains("none required"),
            "got {:?}",
            step_11.title
        );
    }

    /// C10's acceptance criterion: a plan declaring keys is refused, not
    /// silently completed. `A_Key_Write` has no encoder
    /// (`cemi.rs`'s `key_write_has_an_apci_but_no_encoder`, design spec §10.7).
    #[tokio::test]
    async fn a_plan_declaring_access_keys_is_refused_at_step_11() {
        let device = ap2_device();
        let mut session = writer(&device, WriteScope::Download);
        let declaring_plan = two_parts().with_access_keys(
            AccessKeyDeclaration::required(vec![one_access_key_assignment()]).unwrap(),
        );
        let error = Downloader::new(&mut session, declaring_plan)
            .complete_download()
            .await
            .expect_err("a plan declaring access keys must not silently complete");

        assert!(
            matches!(error, DownloadError::AccessKeysNotSupported { declared: 1 }),
            "got {error}"
        );
        // Step 12, the disconnect, never runs: the refusal stops the
        // procedure at step 11, one clause number before it.
        assert!(
            !error.to_string().is_empty(),
            "the refusal names why, for a report a human reads"
        );
    }

    /// The two call sites (CP §3.5.2 Nr. 11 and CP §3.5.3 AP2 Nr. 13) report
    /// the same declaration in the same words: [`modify_access_keys`] is
    /// the one place either procedure calls, so there is nowhere for the
    /// wording to drift apart between them.
    #[tokio::test]
    async fn both_call_sites_report_the_same_declaration_for_the_same_plan() {
        let complete_device = ap2_device();
        let mut complete_session = writer(&complete_device, WriteScope::Download);
        let complete_report = Downloader::new(&mut complete_session, two_parts())
            .complete_download()
            .await
            .expect("a plan declaring nothing completes");
        let complete_title = complete_report
            .steps
            .iter()
            .find(|step| step.number == 11)
            .expect("step 11 exists")
            .title;

        let partial_device = ap2_device_with(SimulatorConfig {
            download_counter: Some(1),
            ..SimulatorConfig::default()
        });
        let mut partial_session = writer(&partial_device, WriteScope::Download);
        // A first complete download so the partial download below has
        // something stored to escalate from is not needed here: the happy
        // path (no allocation failure) reaches Nr. 13 directly.
        let partial_report = Downloader::new(
            &mut partial_session,
            two_parts().with_stored_download_counter(1),
        )
        .partial_download(ObjectIndex::new(3))
        .await
        .expect("a plan declaring nothing completes");
        let partial_title = partial_report
            .steps
            .iter()
            .find(|step| step.number == 13)
            .expect("step 13 exists")
            .title;

        assert_eq!(complete_title, partial_title, "the wording must not drift");
    }

    /// C13 fix round 1, finding 5: `Display` is what a report actually
    /// shows a user, and nothing pinned its wording to each variant before
    /// this — deleting an arm's distinct text was as invisible as deleting
    /// `refuses_partial_download`'s.
    #[test]
    fn download_counter_check_display_text_is_specific_to_each_outcome() {
        assert_eq!(DownloadCounterCheck::NotChecked.to_string(), "not checked");
        assert_eq!(
            DownloadCounterCheck::NoStoredCounter(9).to_string(),
            "no stored Download Counter to compare against; the device answers 9"
        );
        assert_eq!(
            DownloadCounterCheck::Unchanged(4).to_string(),
            "unchanged at 4"
        );
        assert_eq!(
            DownloadCounterCheck::Changed {
                stored: 4,
                current: 9
            }
            .to_string(),
            "changed from 4 to 9 since the preceding configuration"
        );
        assert_eq!(
            DownloadCounterCheck::Absent.to_string(),
            "not implemented on this device"
        );
    }

    /// RES §4.2.30.3, p. 42: unchanged is the case where the partial
    /// download proceeds, and `[C13]`'s [`DownloadCounterCheck::Unchanged`]
    /// is what the report says so.
    #[tokio::test]
    async fn a_partial_download_proceeds_when_the_download_counter_is_unchanged() {
        let device = ap2_device_with(SimulatorConfig {
            download_counter: Some(7),
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        let report = Downloader::new(&mut session, two_parts().with_stored_download_counter(7))
            .partial_download(ObjectIndex::new(3))
            .await
            .expect("an unchanged download counter does not refuse the download");

        assert_eq!(report.download_counter, DownloadCounterCheck::Unchanged(7));
    }

    /// A plan with nothing stored is a gap in this application's own
    /// bookkeeping, not one of RES/CP's three outcomes — see
    /// [`DownloadCounterCheck::NoStoredCounter`]'s own doc comment for why
    /// this proceeds rather than refuses.
    #[tokio::test]
    async fn a_partial_download_proceeds_when_no_download_counter_was_stored() {
        let device = ap2_device_with(SimulatorConfig {
            download_counter: Some(3),
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        let report = Downloader::new(&mut session, two_parts())
            .partial_download(ObjectIndex::new(3))
            .await
            .expect("a plan with nothing stored is not the same as a plan finding a mismatch");

        assert_eq!(
            report.download_counter,
            DownloadCounterCheck::NoStoredCounter(3)
        );
    }

    /// CP §3.12.5, p. 100: a changed Download Counter refuses the partial
    /// download outright — before its first write, matching CP §3.5.2 step
    /// 04's own "nothing written yet" guard shape (`[C10]`'s
    /// `a_read_only_session_cannot_be_sequenced_into_writing` above).
    #[tokio::test]
    async fn a_partial_download_refuses_when_the_download_counter_changed() {
        let device = ap2_device_with(SimulatorConfig {
            download_counter: Some(9),
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        let error = Downloader::new(&mut session, two_parts().with_stored_download_counter(4))
            .partial_download(ObjectIndex::new(3))
            .await
            .expect_err("a changed download counter must refuse a partial download");

        assert!(
            matches!(
                error,
                DownloadError::DownloadCounterChanged {
                    stored: 4,
                    current: 9,
                }
            ),
            "got {error}"
        );
        assert!(!device.memory_was_written());
        assert!(load_state_writes(&device).is_empty());
    }

    /// RES §5.3.2.2, p. 320: an absent Download Counter refuses the partial
    /// download the same way a changed one does, even though the device is
    /// fully conformant (Volume 6 Profiles Annex A, `[C18]`) — absence is
    /// not a malformed read.
    #[tokio::test]
    async fn a_partial_download_refuses_when_the_download_counter_is_absent() {
        let device = ap2_device(); // SimulatorConfig::download_counter defaults to None.
        let mut session = writer(&device, WriteScope::Download);
        let error = Downloader::new(&mut session, two_parts().with_stored_download_counter(1))
            .partial_download(ObjectIndex::new(3))
            .await
            .expect_err("a device with no download counter must refuse a partial download");

        assert!(
            matches!(error, DownloadError::DownloadCounterUnavailable),
            "got {error}"
        );
        assert!(!device.memory_was_written());
        assert!(load_state_writes(&device).is_empty());
    }

    /// C13 fix round 1, finding 2: RES §4.2.30.2, p. 41, has the device
    /// increment the counter on every modification, so a difference of
    /// exactly one is the ordinary "changed" case, not the boundary of a
    /// tolerance. Nothing here treats `stored + 1` as close enough.
    #[tokio::test]
    async fn a_partial_download_refuses_when_the_download_counter_differs_by_exactly_one() {
        let device = ap2_device_with(SimulatorConfig {
            download_counter: Some(5),
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        let error = Downloader::new(&mut session, two_parts().with_stored_download_counter(4))
            .partial_download(ObjectIndex::new(3))
            .await
            .expect_err("a counter one higher than stored is still a change, not a match");

        assert!(
            matches!(
                error,
                DownloadError::DownloadCounterChanged {
                    stored: 4,
                    current: 5,
                }
            ),
            "got {error}"
        );
        assert!(!device.memory_was_written());
        assert!(load_state_writes(&device).is_empty());
    }

    /// C13 fix round 1, finding 3: only `SessionError::PropertyRefused`
    /// means "this device has no Download Counter". A disconnect while
    /// reading it is a comms fault, not a conformant device, and must not
    /// be folded into [`DownloadCounterCheck::Absent`] /
    /// [`DownloadError::DownloadCounterUnavailable`].
    #[tokio::test]
    async fn a_download_counter_read_that_loses_the_connection_is_not_reported_as_absent() {
        let device = ap2_device_with(SimulatorConfig {
            download_counter: Some(5),
            drop_connection_on_download_counter_read: true,
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        let error = Downloader::new(&mut session, two_parts().with_stored_download_counter(4))
            .partial_download(ObjectIndex::new(3))
            .await
            .expect_err("a lost connection is not a successful read of anything");

        assert!(
            matches!(
                error,
                DownloadError::Session(SessionError::ConnectionLost { .. })
            ),
            "got {error}, not the underlying comms fault"
        );
        // C13 re-review, invented mutation: the error above says only that
        // *a* read lost the connection. `open()` reads the mask version and
        // `PID_MANUFACTURER_ID` before `check_download_counter` runs, so a
        // simulator guard that dropped on any property read would produce
        // the same error from a much earlier frame. A dropped frame is never
        // recorded, so the manufacturer-ID read appearing in the log is the
        // proof that the connection was still alive when the download
        // counter was asked for.
        let seen = device.seen();
        assert!(
            seen.iter().any(|entry| matches!(
                entry,
                Seen::PropertyRead {
                    property_id: knx_core::commissioning::properties::PID_MANUFACTURER_ID,
                    ..
                }
            )),
            "the drop fired before the download-counter read: {seen:?}"
        );
        assert!(!device.memory_was_written());
        assert!(load_state_writes(&device).is_empty());
    }

    /// [`DownloadCounterCheck::NotChecked`] is the default a fresh
    /// [`DownloadReport`] starts at, and `complete_download` never replaces
    /// it: CP §3.12.5's refusal is specific to a *partial* download.
    #[tokio::test]
    async fn a_complete_download_never_checks_the_download_counter() {
        let device = ap2_device_with(SimulatorConfig {
            download_counter: Some(5),
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        let report = Downloader::new(&mut session, two_parts().with_stored_download_counter(1))
            .complete_download()
            .await
            .expect("a complete download does not consult the download counter at all");

        assert_eq!(report.download_counter, DownloadCounterCheck::NotChecked);
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
            download_counter: Some(1),
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
        ])
        .with_stored_download_counter(1);
        let mut session = writer(&device, WriteScope::Download);
        let report = Downloader::new(&mut session, parts)
            .partial_download(ObjectIndex::new(3))
            .await
            .expect("the escalation completes the download it turned into");

        assert_eq!(report.escalated_from, Some(ObjectIndex::new(3)));
        // `[C12]` This plan skips Application Program 1 and the Group
        // Object Table (a partial download need not carry all five,
        // `[C8]`), so the escalation only reloads the Group Address Table
        // and the Association Table — but their step numbers are still the
        // AP2 variant's own Nr. 11 and Nr. 12 (`partial_download_variant.rs`,
        // pp. 46-47), not a renumbering down to 09 and 10 for the two that
        // happen to be present here.
        assert_eq!(
            report.outer_step_numbers(),
            vec![1, 3, 2, 4, 5, 6, 7, 8, 11, 12, 13, 14],
            "steps 08, 11 and 12 are the escalation's own reloads, now recorded as \
             numbered outer steps instead of vanishing into ProcedureKind::LoadOnePart: {:?}",
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

    /// `[C12]` acceptance: a full plan carrying all five parts, escalating
    /// from the first, must show every one of the escalation's reloads
    /// (Nr. 08-12, CP §3.5.3 AP2, pp. 46-47) as its own numbered outer step
    /// — none of them silently missing, the rule this module states twice
    /// (`partial_download_variant.rs`'s header and the Nr. 07 comment in
    /// `Downloader::partial_download`).
    #[tokio::test]
    async fn escalated_reloads_09_to_12_appear_as_numbered_outer_steps() {
        const AP1_OBJECT: u8 = 4;
        let device = SimulatedDevice::with_config(SimulatorConfig {
            application_program_objects: [AP2_OBJECT, AP1_OBJECT].into_iter().collect(),
            allocation_fails_once_for: Some(AP2_OBJECT),
            download_counter: Some(1),
            ..SimulatorConfig::default()
        });
        let parts = plan(vec![
            part(
                AP2_OBJECT,
                "Application Program 2",
                8,
                PartKind::ApplicationProgram2,
            ),
            part(
                AP1_OBJECT,
                "Application Program 1",
                8,
                PartKind::ApplicationProgram1,
            ),
            table_part_with_no_version(5, "Group Object Table", 8, PartKind::GroupObjectTable),
            table_part_with_no_version(1, "Group Address Table", 8, PartKind::GroupAddressTable),
            table_part_with_no_version(2, "Association Table", 8, PartKind::AssociationTable),
        ])
        .with_stored_download_counter(1);
        let mut session = writer(&device, WriteScope::Download);
        let report = Downloader::new(&mut session, parts)
            .partial_download(ObjectIndex::new(AP2_OBJECT))
            .await
            .expect("the escalation completes the download it turned into");

        assert_eq!(report.escalated_from, Some(ObjectIndex::new(AP2_OBJECT)));
        assert_eq!(
            report.outer_step_numbers(),
            vec![1, 3, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
            "steps 09-12 are Nr. 07's own escalated reloads (Application Program 1, the \
             Group Object Table, the Group Address Table, the Association Table, in that \
             order) — a gap here is indistinguishable from a step that never ran: {:?}",
            report.steps
        );
        assert_eq!(
            report.parts.len(),
            5,
            "the target plus the four escalated reloads"
        );
    }

    /// `[C12]` acceptance: each of the five variants cites its own step
    /// numbers on the happy path (no escalation) — not one numbering
    /// borrowed from Application Program 2 for all five. Every case here
    /// comes straight from `PartialDownloadVariant`'s own table
    /// (`partial_download_variant.rs`), transcribed from CP §3.5.3, pp.
    /// 44-56.
    #[tokio::test]
    async fn each_variant_cites_its_own_outer_step_numbers() {
        struct Case {
            kind: PartKind,
            object: u8,
            expected: &'static [u8],
        }
        let cases = [
            Case {
                kind: PartKind::ApplicationProgram2,
                object: 3,
                expected: &[1, 3, 2, 4, 5, 6, 13, 14],
            },
            Case {
                kind: PartKind::ApplicationProgram1,
                object: 3,
                expected: &[1, 3, 2, 4, 5, 6, 12, 13],
            },
            Case {
                kind: PartKind::GroupObjectTable,
                object: 5,
                expected: &[1, 3, 2, 4, 5, 6, 11, 12],
            },
            Case {
                kind: PartKind::GroupAddressTable,
                object: 1,
                expected: &[1, 3, 2, 4, 5, 6, 10, 11],
            },
            Case {
                kind: PartKind::AssociationTable,
                object: 2,
                expected: &[1, 3, 2, 4, 5, 6, 7, 8],
            },
        ];

        for case in cases {
            let is_application_program = matches!(
                case.kind,
                PartKind::ApplicationProgram1 | PartKind::ApplicationProgram2
            );
            let device = if is_application_program {
                SimulatedDevice::with_config(SimulatorConfig {
                    application_program_objects: [case.object].into_iter().collect(),
                    download_counter: Some(1),
                    ..SimulatorConfig::default()
                })
            } else {
                SimulatedDevice::with_config(SimulatorConfig {
                    download_counter: Some(1),
                    ..SimulatorConfig::default()
                })
            };
            let one_part = if is_application_program {
                part(case.object, "the part under test", 8, case.kind)
            } else {
                table_part_with_no_version(case.object, "the part under test", 8, case.kind)
            };
            let parts = plan(vec![one_part]).with_stored_download_counter(1);
            let mut session = writer(&device, WriteScope::Download);
            let report = Downloader::new(&mut session, parts)
                .partial_download(ObjectIndex::new(case.object))
                .await
                .unwrap_or_else(|err| panic!("{:?} partial download failed: {err}", case.kind));
            assert_eq!(
                report.outer_step_numbers(),
                case.expected,
                "{:?} cited the wrong outer step numbers",
                case.kind
            );
        }
    }

    /// `[C12]` acceptance: CP §3.5.3, Association Table variant, p. 56, gives
    /// its Nr. 06 no *"⇒ Continue"* and no escalation branch — *"if [Base
    /// Address] is zero then allocation was not successful. This causes an
    /// error message of the MaC to the Installer."* A failed allocation here
    /// must be reported as a plain procedure failure, not retried.
    ///
    /// [`SimulatorConfig::allocation_fails_once_for`] fails only the
    /// *first* allocation attempt for an object; a second attempt for the
    /// same object succeeds. So a downloader that (wrongly) unloads and
    /// retries here would get a *successful* second attempt and return
    /// `Ok` with `escalated_from` set on a variant CP §3.5.3 gives no
    /// escalation to. Seeing exactly one allocation attempt and exactly one
    /// unload of this object is what proves that never happened — the
    /// `DownloadReport` this run would have produced, had it succeeded,
    /// could only ever have `escalated_from == None`.
    #[tokio::test]
    async fn the_last_segments_allocation_failure_is_terminal_not_escalated() {
        const ASSOCIATION_TABLE_OBJECT: u8 = 2;
        let device = SimulatedDevice::with_config(SimulatorConfig {
            allocation_fails_once_for: Some(ASSOCIATION_TABLE_OBJECT),
            download_counter: Some(1),
            ..SimulatorConfig::default()
        });
        let parts = plan(vec![table_part_with_no_version(
            ASSOCIATION_TABLE_OBJECT,
            "Association Table",
            6,
            PartKind::AssociationTable,
        )])
        .with_stored_download_counter(1);
        let mut session = writer(&device, WriteScope::Download);
        let error = Downloader::new(&mut session, parts)
            .partial_download(ObjectIndex::new(ASSOCIATION_TABLE_OBJECT))
            .await
            .expect_err(
                "the Association Table variant has no Nr. 07 escalation (CP §3.5.3, p. 56); \
                 a retry that happened to succeed on a device that only fails once would hide \
                 that this defect ever existed",
            );
        assert!(
            matches!(
                error,
                DownloadError::Session(SessionError::AllocationFailed { .. })
            ),
            "got {error}"
        );

        let writes = load_state_writes(&device);
        let allocation_attempts = writes
            .iter()
            .filter(|(object_index, event)| {
                *object_index == ASSOCIATION_TABLE_OBJECT
                    && *event == LoadEvent::AdditionalLoadControls.octet()
            })
            .count();
        assert_eq!(
            allocation_attempts, 1,
            "no retry: CP §3.5.3, p. 56 gives this variant nothing to retry through"
        );
        let unloads = writes
            .iter()
            .filter(|(object_index, event)| {
                *object_index == ASSOCIATION_TABLE_OBJECT && *event == LoadEvent::Unload.octet()
            })
            .count();
        assert_eq!(
            unloads, 1,
            "the escalation branch's own unload (CP §3.5.3 Nr. 07) never runs for a \
             variant with no Nr. 07"
        );
    }

    /// C12 fix round 1, finding 1: distinguishes the escalation guard's two
    /// plausible readings — "this variant has an escalation branch"
    /// (`variant.escalation_targets().is_empty()`, the correct one) from
    /// "this plan has a follower part to escalate onto"
    /// (`self.plan.parts[position + 1..].is_empty()`, a mutant that happens
    /// to agree with the correct guard everywhere the rest of this suite
    /// looks, because the one variant with no escalation — the Association
    /// Table — is also always last in a valid download order and so never
    /// has a follower). A single-part [`PartKind::GroupAddressTable`] plan
    /// separates the two: the variant *does* have an escalation branch (CP
    /// §3.5.3, p. 55, `⇒ Continue at Nr. 7`), even though this particular
    /// plan carries no follower for it to reload. The correct guard
    /// escalates and retries the target itself, succeeding on the second
    /// (unfailing) attempt; the mutant guard sees no follower and skips
    /// straight to `Err`.
    #[tokio::test]
    async fn an_eligible_variant_with_no_follower_still_escalates_and_retries() {
        const GROUP_ADDRESS_TABLE_OBJECT: u8 = 1;
        let device = SimulatedDevice::with_config(SimulatorConfig {
            allocation_fails_once_for: Some(GROUP_ADDRESS_TABLE_OBJECT),
            download_counter: Some(1),
            ..SimulatorConfig::default()
        });
        let parts = plan(vec![table_part_with_no_version(
            GROUP_ADDRESS_TABLE_OBJECT,
            "Group Address Table",
            6,
            PartKind::GroupAddressTable,
        )])
        .with_stored_download_counter(1);
        let mut session = writer(&device, WriteScope::Download);
        let report = Downloader::new(&mut session, parts)
            .partial_download(ObjectIndex::new(GROUP_ADDRESS_TABLE_OBJECT))
            .await
            .expect(
                "the Group Address Table variant does have a Nr. 07 escalation (CP §3.5.3, \
                 p. 55); a single-part plan with no follower to reload must still retry the \
                 target itself, not fail outright",
            );
        assert_eq!(
            report.escalated_from,
            Some(ObjectIndex::new(GROUP_ADDRESS_TABLE_OBJECT)),
            "the escalation did happen — this is CP §3.5.3's own documented limitation \
             (§113): a shortened plan with no follower only reloads the target"
        );
        assert_eq!(
            report.parts.len(),
            1,
            "one part attempted, its own retry — no follower existed to reload"
        );

        let writes = load_state_writes(&device);
        let allocation_attempts = writes
            .iter()
            .filter(|(object_index, event)| {
                *object_index == GROUP_ADDRESS_TABLE_OBJECT
                    && *event == LoadEvent::AdditionalLoadControls.octet()
            })
            .count();
        assert_eq!(
            allocation_attempts, 2,
            "the failed first attempt and the escalation's retry, CP §3.5.3 Nr. 06 and Nr. 08"
        );
    }

    /// `[C9]` acceptance: CP §3.5.3 AP2 variant Nr. 08, p. 46, is the target
    /// part's own reload after escalation, and it carries the same full
    /// "Compare CRC checksum" block as Nr. 06, p. 46 — the two are worded
    /// identically, `PID_MCB` read and all. Nr. 09-12, p. 47, reload the
    /// segments that only followed the target and end with "Read and save
    /// CRC checksum" — storing, not comparing; no such block. A version of
    /// this fix that passed `compare_crc = false` for the whole
    /// `parts[position..]` slice reported `NotCompared` for the target too,
    /// which is what this test pins down.
    #[tokio::test]
    async fn an_escalated_reload_compares_the_crc_for_the_target_part_only() {
        let device = ap2_device_with(SimulatorConfig {
            allocation_fails_once_for: Some(3),
            download_counter: Some(1),
            ..SimulatorConfig::default()
        });
        let stored = device.mcb(ObjectIndex::new(3));
        let parts = plan(vec![
            part(
                3,
                "Application Program 2",
                16,
                PartKind::ApplicationProgram2,
            )
            .with_stored_mcb(stored),
            part(1, "Address Table", 8, PartKind::GroupAddressTable),
            part(2, "Association Table", 6, PartKind::AssociationTable),
        ])
        .with_stored_download_counter(1);
        let mut session = writer(&device, WriteScope::Download);
        let report = Downloader::new(&mut session, parts)
            .partial_download(ObjectIndex::new(3))
            .await
            .expect("the escalation completes the download it turned into");

        assert_eq!(report.escalated_from, Some(ObjectIndex::new(3)));
        assert_eq!(report.parts.len(), 3);
        assert_eq!(
            report.parts[0].object_index,
            ObjectIndex::new(3),
            "the target is reloaded first, ascending order (CP §3.5.3 Nr. 07, p. 46)"
        );
        assert_eq!(
            report.parts[0].crc,
            CrcComparison::Matched,
            "Nr. 08's reload of the target part must compare for real, not read \
             NotCompared for the very part whose failed CRC comparison escalated this \
             download in the first place"
        );
        for outcome in &report.parts[1..] {
            assert_eq!(
                outcome.crc,
                CrcComparison::NotCompared,
                "Nr. 09-12 carry no CRC comparison block for the segments that only \
                 followed the target"
            );
        }
    }

    /// A CRC that matches is reported and is *not* used to skip the write:
    /// step 05 has already unloaded the part, so `[D]` RES Table 93 has
    /// declared the data undefined, and the algorithm CP §3.5.3 names for
    /// this case is not specified anywhere (design spec §7.4, §12).
    #[tokio::test]
    async fn a_matching_crc_is_reported_and_the_data_is_written_anyway() {
        let device = ap2_device_with(SimulatorConfig {
            download_counter: Some(1),
            ..SimulatorConfig::default()
        });
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
        ])
        .with_stored_download_counter(1);
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
        let device = ap2_device_with(SimulatorConfig {
            download_counter: Some(1),
            ..SimulatorConfig::default()
        });
        let parts = plan(vec![part(
            3,
            "Application Program 2",
            6,
            PartKind::ApplicationProgram2,
        )
        .with_stored_mcb(vec![0xDE, 0xAD])])
        .with_stored_download_counter(1);
        let mut session = writer(&device, WriteScope::Download);
        let differed = Downloader::new(&mut session, parts)
            .partial_download(ObjectIndex::new(3))
            .await
            .expect("a partial download of one part");
        assert_eq!(differed.parts[0].crc, CrcComparison::Differed);

        let device = ap2_device_with(SimulatorConfig {
            download_counter: Some(1),
            ..SimulatorConfig::default()
        });
        let mut session = writer(&device, WriteScope::Download);
        let unknown = Downloader::new(
            &mut session,
            plan(vec![part(
                3,
                "Application Program 2",
                6,
                PartKind::ApplicationProgram2,
            )])
            .with_stored_download_counter(1),
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
        let device = ap2_device_with(SimulatorConfig {
            download_counter: Some(1),
            ..SimulatorConfig::default()
        });
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
        .with_stored_mcb(stored.to_vec())])
        .with_stored_download_counter(1);
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
        let device = ap2_device_with(SimulatorConfig {
            download_counter: Some(1),
            ..SimulatorConfig::default()
        });
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
        .with_stored_mcb(stored.to_vec())])
        .with_stored_download_counter(1);
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

    /// The "regardless" half of bit 0 outranking the CRC: this pairs bit 0
    /// set with CRC octets that differ, which `compare_mcb_bit_0_outranks_a_
    /// matching_crc` above does not cover. A comparison that checked the
    /// CRC first, and only inspected bit 0 once the CRC already matched,
    /// would report `Differed` here instead — this guards against exactly
    /// that ordering bug.
    #[test]
    fn compare_mcb_bit_0_outranks_a_differing_crc() {
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
            crc: 0x5678,
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

    /// `[C8]` Acceptance: all five kinds, in the order CP §3.5.2 Nr. 06-10
    /// and CP §3.5.3 AP2 Nr. 08-12 both give them, build without complaint.
    #[test]
    fn the_normative_download_order_is_accepted() {
        let parts = vec![
            part(1, "AP2", 4, PartKind::ApplicationProgram2),
            part(2, "AP1", 4, PartKind::ApplicationProgram1),
            part(3, "GOT", 4, PartKind::GroupObjectTable),
            part(4, "Address Table", 4, PartKind::GroupAddressTable),
            part(5, "Association Table", 4, PartKind::AssociationTable),
        ];
        assert!(DownloadPlan::new(SIMULATED_MANUFACTURER, parts).is_ok());
    }

    /// A partial plan may skip kinds — a plan of just two of the five is
    /// ordinary — as long as the ones it keeps stay in relative order.
    #[test]
    fn a_gapped_plan_that_keeps_relative_order_is_accepted() {
        let parts = vec![
            part(1, "AP2", 4, PartKind::ApplicationProgram2),
            part(2, "Address Table", 4, PartKind::GroupAddressTable),
        ];
        assert!(DownloadPlan::new(SIMULATED_MANUFACTURER, parts).is_ok());
    }

    /// One adjacent pair swapped: Application Program 1 loaded before
    /// Application Program 2, which CP §3.5.2 Nr. 06-10 places the other
    /// way round by row position.
    #[test]
    fn an_adjacent_swap_is_rejected() {
        let parts = vec![
            part(1, "AP1", 4, PartKind::ApplicationProgram1),
            part(2, "AP2", 4, PartKind::ApplicationProgram2),
        ];
        let error = DownloadPlan::new(SIMULATED_MANUFACTURER, parts)
            .expect_err("Application Program 1 before Application Program 2 is out of order");
        assert!(
            matches!(
                error,
                PlanError::OutOfOrder {
                    kind: PartKind::ApplicationProgram2,
                    preceding_kind: PartKind::ApplicationProgram1,
                    ..
                }
            ),
            "got {error:?}"
        );
    }

    /// The full reverse of the normative order. Every adjacent pair
    /// violates it; the constructor reports the first one it finds rather
    /// than trying to describe all of them at once.
    #[test]
    fn a_fully_reversed_plan_is_rejected() {
        let parts = vec![
            part(1, "Association Table", 4, PartKind::AssociationTable),
            part(2, "Address Table", 4, PartKind::GroupAddressTable),
            part(3, "GOT", 4, PartKind::GroupObjectTable),
            part(4, "AP1", 4, PartKind::ApplicationProgram1),
            part(5, "AP2", 4, PartKind::ApplicationProgram2),
        ];
        let error = DownloadPlan::new(SIMULATED_MANUFACTURER, parts)
            .expect_err("a fully reversed plan is out of order at its very first pair");
        assert!(
            matches!(
                error,
                PlanError::OutOfOrder {
                    kind: PartKind::GroupAddressTable,
                    preceding_kind: PartKind::AssociationTable,
                    ..
                }
            ),
            "got {error:?}"
        );
    }

    /// Two parts of the same kind (different objects; same-object duplicates
    /// are their own error, tested separately). Neither download-order table
    /// lists a second row for a kind it already lists once, so there is no
    /// order to place them in, and the constructor refuses rather than
    /// picking one.
    #[test]
    fn two_parts_of_the_same_kind_are_rejected() {
        let parts = vec![
            part(1, "Association Table one", 4, PartKind::AssociationTable),
            part(2, "Association Table two", 4, PartKind::AssociationTable),
        ];
        let error = DownloadPlan::new(SIMULATED_MANUFACTURER, parts)
            .expect_err("no download order distinguishes two parts of the same kind");
        assert!(
            matches!(
                error,
                PlanError::OutOfOrder {
                    kind: PartKind::AssociationTable,
                    preceding_kind: PartKind::AssociationTable,
                    ..
                }
            ),
            "got {error:?}"
        );
    }

    /// `[C8]` The escalation slice (`parts[position..]` in
    /// [`Downloader::partial_download`]) takes its target set entirely from
    /// the plan's order. This proves the actual danger the order check
    /// exists for: build a plan with Application Program 1 ahead of
    /// Application Program 2 and confirm the constructor never hands it to
    /// a procedure to escalate from — not just that the error variant looks
    /// right, but that a caller cannot get a `DownloadPlan` out of a
    /// wrongly-ordered `Vec` at all.
    #[test]
    fn a_wrongly_ordered_plan_never_becomes_a_downloadable_plan() {
        let parts = vec![
            part(1, "AP1", 4, PartKind::ApplicationProgram1),
            part(2, "AP2", 4, PartKind::ApplicationProgram2),
            part(3, "Association Table", 4, PartKind::AssociationTable),
        ];
        assert!(
            DownloadPlan::new(SIMULATED_MANUFACTURER, parts).is_err(),
            "a caller-supplied order that violates CP §3.5.2 Nr. 06-10 must never reach \
             partial_download's escalation slice"
        );
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
        let device = SimulatedDevice::with_config(SimulatorConfig {
            download_counter: Some(1),
            ..SimulatorConfig::default()
        });
        let parts = plan(vec![part(
            2,
            "Association Table",
            6,
            PartKind::AssociationTable,
        )])
        .with_stored_download_counter(1);
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
        let device = SimulatedDevice::with_config(SimulatorConfig {
            download_counter: Some(1),
            ..SimulatorConfig::default()
        });
        let parts = plan(vec![part(
            3,
            "Application Program 2",
            4,
            PartKind::ApplicationProgram2,
        )])
        .with_stored_download_counter(1);
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
