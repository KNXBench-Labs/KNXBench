//! Shareable, payload-free record of the read-only check after a regular memory download.
//!
//! One vocabulary for the CLI, the tester's report, the server's download
//! status and the durable activity history. It is built only from
//! [`knx_net::commissioning::post_download`]'s result: [`PostDownloadCheck::from_observation`]
//! takes reach and the load-state verdict from that module's own
//! `reach()` and `load_verdict()`; nothing here classifies a reading anew.
//!
//! **Separate claims.** The transfer (`written`) and the closing restart
//! stay in their own fields elsewhere; this record says only what one fresh,
//! read-only attempt after them found: whether the target was reached, its
//! identity and every load state the plan settles, each expected next to
//! observed. A matching identity is not the same physical exemplar (no
//! serial number is read), `loaded` is a load state, not a run state, and
//! [`RuntimeState`] is always `unknown`.
//!
//! **Payload-free.** No address, serial number, raw identity octets, key or
//! free-text transport error: a mismatch names the part that differed, not
//! what the device answered.
//!
//! **Absent means not checked.** A record without this field (an older
//! server, a download that failed, a run that ended before the check) is
//! [`Observation::NotChecked`], never a match, `loaded` or running.

use std::fmt;

use knx_core::commissioning::load_control_memory::MemoryLoadStateMachine;
use knx_core::commissioning::load_state::LoadState;
use knx_core::commissioning::properties::{ObjectIndex, PID_HARDWARE_TYPE};
use knx_net::commissioning::post_download::{
    IdentityMismatch, IdentityReading, LoadStateObservation, LoadStateVerdict, NotPerformed,
    ObservationOutcome, PostDownloadObservation, Reach,
};
use serde::{Deserialize, Serialize};

/// What the read-only check after a download found, and what it never says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PostDownloadCheck {
    /// How far the check got.
    pub observation: Observation,
    /// Whether the application runs: never observed.
    pub runtime: RuntimeState,
}

/// Whether the application runs. No verified run-state reader exists, so
/// the only value is `unknown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeState {
    Unknown,
}

/// How far the check got. Each level nests the next, so a record cannot say
/// "not reached" and carry an identity at the same time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Observation {
    /// No check result exists: the transfer failed or did not end, the run
    /// predates the check, or the record was written by an older version.
    NotChecked {},
    /// Nothing was sent: the plan has no verified identity to compare.
    Unsupported { reason: UnsupportedReason },
    /// Nothing was sent: no read-only session could be built.
    NotPerformed {},
    /// The check started and did not settle (cancelled or shut down).
    /// Nothing about the device is known from it.
    Interrupted {},
    /// The one read-only attempt ran.
    Attempted { reach: ReachCheck },
}

/// Why nothing was sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UnsupportedReason {
    /// The plan's mask has no verified application-identity layout.
    UnsupportedMask,
    /// The plan has no single verified five-octet application identity.
    ApplicationUnknown,
}

/// Whether the attempt reached the target, on its own evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "result",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ReachCheck {
    /// The connection did not open, or no transmission of the first read
    /// was acknowledged. Nothing about the download follows from it.
    NotReached {},
    /// The first read failed without evidence either way.
    Unknown {},
    /// The target answered or acknowledged the first read.
    Reached { identity: IdentityCheck },
}

/// The identity the reached target showed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "result",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum IdentityCheck {
    /// A read failed; identity and load states are unknown.
    Unavailable { part: IdentityPart },
    /// The device answered something other than the plan's identity. No
    /// load state was read.
    Mismatch {
        part: IdentityPart,
        kind: MismatchKind,
    },
    /// Mask, manufacturer, every declared comparison and the exact
    /// application identity matched; then the load states were read.
    Match(MatchedLoadStates),
}

/// Which identity read failed or differed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum IdentityPart {
    Mask {},
    Manufacturer {},
    /// A comparison the plan declares (`ObjIdx`, `PropId`: protocol
    /// constants, not device data).
    Property {
        object_index: u8,
        property_id: u8,
    },
    /// The application identity (`PID_PROGRAM_VERSION`).
    Application {},
}

/// How an identity answer differed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MismatchKind {
    /// A complete answer with another value.
    Differs,
    /// An answer of the wrong width.
    Malformed,
}

/// Load states after a matching identity, and the verdict on them.
///
/// Deserialisation refuses a stored verdict that its own machines
/// contradict, and a machine read after a failed one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "MatchedWire")]
pub struct MatchedLoadStates {
    /// Always `notVerified`: no serial number is read.
    exemplar: ExemplarCheck,
    /// One entry per machine the plan loads, unloads or requires, in machine order.
    load_states: Vec<MachineReading>,
    /// The verdict on the load states against the plan, and only on those.
    plan_load_verdict: PlanLoadVerdict,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatchedWire {
    exemplar: ExemplarCheck,
    load_states: Vec<MachineReading>,
    plan_load_verdict: PlanLoadVerdict,
}

/// Whether the device is the same physical exemplar the project means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExemplarCheck {
    NotVerified,
}

/// The verdict on the load states against the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlanLoadVerdict {
    /// Every machine was read in the state the plan leaves it in (which may
    /// be `unloaded`). Not a claim that anything runs.
    MatchesPlan,
    /// At least one machine was read in another state.
    DiffersFromPlan,
    /// Not enough was read or expected to say either.
    Unknown,
}

/// One machine: what the plan leaves it in and what was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MachineReading {
    pub machine: MachineCode,
    /// `None` when the plan's last control does not settle the machine.
    pub expected: Option<LoadStateCode>,
    pub observed: MachineObservation,
}

/// A load-state read's result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "result",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum MachineObservation {
    Observed {
        load_state: LoadStateCode,
    },
    /// The read failed: unknown.
    Unavailable {},
    /// Not read, because an earlier read failed.
    NotRead {},
}

/// The memory load-state machines, as stable codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MachineCode {
    AddressTable,
    AssociationTable,
    ApplicationProgram,
    PeiProgram,
}

/// Load states (MP §3.31), as stable codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoadStateCode {
    Unloaded,
    Loaded,
    Loading,
    Error,
    Unloading,
    LoadCompleting,
}

impl From<MemoryLoadStateMachine> for MachineCode {
    fn from(machine: MemoryLoadStateMachine) -> Self {
        match machine {
            MemoryLoadStateMachine::AddressTable => Self::AddressTable,
            MemoryLoadStateMachine::AssociationTable => Self::AssociationTable,
            MemoryLoadStateMachine::ApplicationProgram => Self::ApplicationProgram,
            MemoryLoadStateMachine::PeiProgram => Self::PeiProgram,
        }
    }
}

impl From<LoadState> for LoadStateCode {
    fn from(state: LoadState) -> Self {
        match state {
            LoadState::Unloaded => Self::Unloaded,
            LoadState::Loaded => Self::Loaded,
            LoadState::Loading => Self::Loading,
            LoadState::Error => Self::Error,
            LoadState::Unloading => Self::Unloading,
            LoadState::LoadCompleting => Self::LoadCompleting,
        }
    }
}

impl From<LoadStateVerdict> for PlanLoadVerdict {
    fn from(verdict: LoadStateVerdict) -> Self {
        match verdict {
            LoadStateVerdict::MatchesPlan => Self::MatchesPlan,
            LoadStateVerdict::DiffersFromPlan => Self::DiffersFromPlan,
            LoadStateVerdict::Unknown => Self::Unknown,
        }
    }
}

impl From<IdentityReading> for IdentityPart {
    fn from(reading: IdentityReading) -> Self {
        match reading {
            IdentityReading::Mask => Self::Mask {},
            IdentityReading::Manufacturer => Self::Manufacturer {},
            IdentityReading::Property {
                object_index,
                property_id,
            } => Self::Property {
                object_index,
                property_id,
            },
            IdentityReading::Application => Self::Application {},
        }
    }
}

/// `PID_HARDWARE_TYPE` of the Device Object, for the malformed-width mismatch.
fn hardware_type() -> IdentityPart {
    IdentityPart::Property {
        object_index: ObjectIndex::DEVICE.octet(),
        property_id: PID_HARDWARE_TYPE,
    }
}

fn mismatch(mismatch: &IdentityMismatch) -> IdentityCheck {
    let (part, kind) = match mismatch {
        IdentityMismatch::Mask { .. } => (IdentityPart::Mask {}, MismatchKind::Differs),
        IdentityMismatch::Manufacturer { .. } => {
            (IdentityPart::Manufacturer {}, MismatchKind::Differs)
        }
        IdentityMismatch::Property {
            object_index,
            property_id,
            ..
        } => (
            IdentityPart::Property {
                object_index: *object_index,
                property_id: *property_id,
            },
            MismatchKind::Differs,
        ),
        IdentityMismatch::HardwareTypeMalformed { .. } => {
            (hardware_type(), MismatchKind::Malformed)
        }
        IdentityMismatch::ApplicationMalformed { .. } => {
            (IdentityPart::Application {}, MismatchKind::Malformed)
        }
        IdentityMismatch::Application { .. } => {
            (IdentityPart::Application {}, MismatchKind::Differs)
        }
    };
    IdentityCheck::Mismatch { part, kind }
}

impl PostDownloadCheck {
    fn new(observation: Observation) -> Self {
        Self {
            observation,
            runtime: RuntimeState::Unknown,
        }
    }

    /// No check result: what an absent field means.
    pub fn not_checked() -> Self {
        Self::new(Observation::NotChecked {})
    }

    /// The check started and did not settle.
    pub fn interrupted() -> Self {
        Self::new(Observation::Interrupted {})
    }

    /// The sanitized form of `observation`. Reach and verdict are the
    /// observation's own; error texts and device answers are dropped.
    pub fn from_observation(observation: &PostDownloadObservation) -> Self {
        let reached = |identity| Observation::Attempted {
            reach: ReachCheck::Reached { identity },
        };
        Self::new(match (observation.reach(), &observation.outcome) {
            (_, ObservationOutcome::NotPerformed(why)) => match why {
                NotPerformed::UnsupportedMask(_) => Observation::Unsupported {
                    reason: UnsupportedReason::UnsupportedMask,
                },
                NotPerformed::ApplicationUnknown => Observation::Unsupported {
                    reason: UnsupportedReason::ApplicationUnknown,
                },
                NotPerformed::Session(_) => Observation::NotPerformed {},
            },
            (Reach::NotReached, _) => Observation::Attempted {
                reach: ReachCheck::NotReached {},
            },
            (Reach::Unknown, _) => Observation::Attempted {
                reach: ReachCheck::Unknown {},
            },
            (_, ObservationOutcome::IdentityUnavailable { reading, .. }) => {
                reached(IdentityCheck::Unavailable {
                    part: (*reading).into(),
                })
            }
            (_, ObservationOutcome::IdentityMismatch(found)) => reached(mismatch(found)),
            (_, ObservationOutcome::Observed { load_states, .. }) => {
                reached(IdentityCheck::Match(MatchedLoadStates {
                    exemplar: ExemplarCheck::NotVerified,
                    load_states: load_states
                        .iter()
                        .map(|reading| MachineReading {
                            machine: reading.machine.into(),
                            expected: reading.expected.map(Into::into),
                            observed: match reading.state {
                                LoadStateObservation::Observed(state) => {
                                    MachineObservation::Observed {
                                        load_state: state.into(),
                                    }
                                }
                                LoadStateObservation::Unavailable(_) => {
                                    MachineObservation::Unavailable {}
                                }
                                LoadStateObservation::NotRead => MachineObservation::NotRead {},
                            },
                        })
                        .collect(),
                    plan_load_verdict: observation.load_verdict().into(),
                }))
            }
            // `reach()` says reached exactly for the three outcomes above;
            // anything else is not a reading this record may invent.
            (
                _,
                ObservationOutcome::Unreachable(_) | ObservationOutcome::FirstReadInconclusive(_),
            ) => Observation::Attempted {
                reach: ReachCheck::Unknown {},
            },
        })
    }

    /// Whether a check result exists (anything but [`Observation::NotChecked`]).
    pub fn is_checked(&self) -> bool {
        !matches!(self.observation, Observation::NotChecked {})
    }

    /// The identity check, when the target was reached.
    pub fn identity(&self) -> Option<&IdentityCheck> {
        match &self.observation {
            Observation::Attempted {
                reach: ReachCheck::Reached { identity },
            } => Some(identity),
            _ => None,
        }
    }

    /// One line: what a report summary and the CLI's heading show.
    pub fn line(&self) -> String {
        let reach = match &self.observation {
            Observation::NotChecked {} => return "not checked".into(),
            Observation::Unsupported { reason } => {
                return format!("not performed ({reason})");
            }
            Observation::NotPerformed {} => {
                return "not performed (no read-only session could be built)".into()
            }
            Observation::Interrupted {} => return "interrupted; nothing is known from it".into(),
            Observation::Attempted { reach } => reach,
        };
        match reach {
            ReachCheck::NotReached {} => "not reached in one read-only attempt".into(),
            ReachCheck::Unknown {} => {
                "reach unknown (the first read failed without evidence)".into()
            }
            ReachCheck::Reached { identity } => match identity {
                IdentityCheck::Unavailable { part } => {
                    format!("reached; identity unavailable ({part} could not be read)")
                }
                IdentityCheck::Mismatch { part, kind } => {
                    format!("reached; identity MISMATCH ({part} {kind})")
                }
                IdentityCheck::Match(matched) => format!(
                    "reached; identity matches the plan; load states {}",
                    matched.plan_load_verdict
                ),
            },
        }
    }

    /// The lines the CLI prints after the transfer's own summary.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![format!("reached / identity: {}", self.line())];
        if let Some(IdentityCheck::Match(matched)) = self.identity() {
            lines.push(
                "  identity: mask, manufacturer, declared comparisons and application match; \
                 no serial number read, so not proven the same physical device"
                    .into(),
            );
            for reading in &matched.load_states {
                lines.push(format!(
                    "  {}: expected {}, observed {}",
                    reading.machine,
                    reading
                        .expected
                        .map_or_else(|| "unresolved".to_string(), |state| state.to_string()),
                    reading.observed
                ));
            }
        }
        lines.push("application running: unknown (not observed)".into());
        lines
    }
}

impl MatchedLoadStates {
    /// Every machine read, in machine order.
    pub fn load_states(&self) -> &[MachineReading] {
        &self.load_states
    }

    /// The observation's verdict on them.
    pub fn plan_load_verdict(&self) -> PlanLoadVerdict {
        self.plan_load_verdict
    }
}

impl TryFrom<MatchedWire> for MatchedLoadStates {
    type Error = &'static str;

    /// A consistency check of a stored record, not a classifier: live
    /// verdicts come from `knx_net`'s `load_verdict()` alone.
    fn try_from(wire: MatchedWire) -> Result<Self, Self::Error> {
        let mut failed = false;
        for reading in &wire.load_states {
            match reading.observed {
                MachineObservation::NotRead {} if !failed => {
                    return Err("a machine is unread without an earlier failure")
                }
                MachineObservation::Observed { .. } if failed => {
                    return Err("a machine was read after a failed read")
                }
                MachineObservation::Unavailable {} if failed => {
                    return Err("only the first failed read is unavailable")
                }
                MachineObservation::Unavailable {} => failed = true,
                _ => {}
            }
        }
        if !wire
            .load_states
            .windows(2)
            .all(|pair| pair[0].machine < pair[1].machine)
        {
            return Err("machines are not in machine order");
        }
        let compared = |reading: &MachineReading| match (reading.expected, reading.observed) {
            (Some(expected), MachineObservation::Observed { load_state }) => {
                Some(expected == load_state)
            }
            _ => None,
        };
        let consistent = match wire.plan_load_verdict {
            PlanLoadVerdict::DiffersFromPlan => {
                wire.load_states.iter().any(|r| compared(r) == Some(false))
            }
            PlanLoadVerdict::MatchesPlan => {
                !wire.load_states.is_empty()
                    && wire.load_states.iter().all(|r| compared(r) == Some(true))
            }
            PlanLoadVerdict::Unknown => {
                !wire.load_states.iter().any(|r| compared(r) == Some(false))
                    && (wire.load_states.is_empty()
                        || wire.load_states.iter().any(|r| compared(r).is_none()))
            }
        };
        if !consistent {
            return Err("the load-state verdict contradicts its machines");
        }
        Ok(Self {
            exemplar: wire.exemplar,
            load_states: wire.load_states,
            plan_load_verdict: wire.plan_load_verdict,
        })
    }
}

impl fmt::Display for UnsupportedReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnsupportedMask => "no verified application identity layout for the plan's mask",
            Self::ApplicationUnknown => "the plan has no verified application identity",
        })
    }
}

impl fmt::Display for IdentityPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mask {} => f.write_str("mask version"),
            Self::Manufacturer {} => f.write_str("manufacturer"),
            Self::Property {
                object_index,
                property_id,
            } => write!(f, "property {object_index}/{property_id}"),
            Self::Application {} => f.write_str("application identity"),
        }
    }
}

impl fmt::Display for MismatchKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Differs => "differs",
            Self::Malformed => "has the wrong width",
        })
    }
}

impl fmt::Display for PlanLoadVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MatchesPlan => "match the plan",
            Self::DiffersFromPlan => "DIFFER from the plan",
            Self::Unknown => "unknown against the plan",
        })
    }
}

impl fmt::Display for MachineCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AddressTable => "address table",
            Self::AssociationTable => "association table",
            Self::ApplicationProgram => "application program",
            Self::PeiProgram => "PEI program",
        })
    }
}

impl fmt::Display for LoadStateCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unloaded => "unloaded",
            Self::Loaded => "loaded",
            Self::Loading => "loading",
            Self::Error => "error",
            Self::Unloading => "unloading",
            Self::LoadCompleting => "load completing",
        })
    }
}

impl fmt::Display for MachineObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Observed { load_state } => write!(f, "{load_state}"),
            Self::Unavailable {} => f.write_str("unavailable (the read failed)"),
            Self::NotRead {} => f.write_str("not read (an earlier read failed)"),
        }
    }
}

#[cfg(test)]
#[path = "post_download_check_tests.rs"]
mod tests;
