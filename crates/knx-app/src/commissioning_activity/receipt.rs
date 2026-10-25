//! Run-scoped, payload-free preparation and settlement receipt of one regular download.
//!
//! Both objects are display evidence about one run: what was planned and how
//! the project belief was settled. They are not a per-device baseline, an
//! observed device state or a statement about where a flag came from. Each
//! carries its own `format: 1`; the stored page stays on the outer format 2,
//! and a reader that does not know them refuses the page rather than drop them.
use chrono::{DateTime, SecondsFormat, Utc};
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_core::commissioning::pending::ProgrammingParts;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::Written;
use crate::device_download::PreparedDownload;
use crate::programming_state::{written_parts, Settlement};

/// Longest accepted program id; one byte per character, so escaping never grows it.
pub const MAX_PROGRAM_ID: usize = 128;
/// Longest accepted application version.
pub const MAX_APPLICATION_VERSION: usize = 32;
const SHA256_HEX: usize = 64;
/// `YYYY-MM-DDTHH:MM:SS.mmmZ`
const TIME_LENGTH: usize = 24;

/// The nested version marker: exactly the integer 1, nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format1;

impl Serialize for Format1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(1)
    }
}

impl<'de> Deserialize<'de> for Format1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match u64::deserialize(deserializer)? {
            1 => Ok(Self),
            _ => Err(D::Error::custom("unknown receipt format")),
        }
    }
}

/// Which parts the run was planned to write. Known even when nothing else is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    #[serde(rename = "complete")]
    Complete,
    #[serde(rename = "partial-parameters")]
    PartialParameters,
    #[serde(rename = "partial-group-addresses")]
    PartialGroupAddresses,
    #[serde(rename = "partial-both")]
    PartialBoth,
}

impl Scope {
    fn partial(self) -> Option<PartialDownloadParts> {
        let (parameters, group_addresses) = match self {
            Self::Complete => return None,
            Self::PartialParameters => (true, false),
            Self::PartialGroupAddresses => (false, true),
            Self::PartialBoth => (true, true),
        };
        Some(PartialDownloadParts {
            parameters,
            group_addresses,
        })
    }

    /// The parts a run of this scope writes into device memory.
    pub fn written_parts(self) -> ProgrammingParts {
        written_parts(self.partial())
    }

    /// `None` for a partial scope that selects nothing, which no plan has.
    pub fn of(partial: Option<PartialDownloadParts>) -> Option<Self> {
        Some(match partial {
            None => Self::Complete,
            Some(PartialDownloadParts {
                parameters: true,
                group_addresses: false,
            }) => Self::PartialParameters,
            Some(PartialDownloadParts {
                parameters: false,
                group_addresses: true,
            }) => Self::PartialGroupAddresses,
            Some(PartialDownloadParts {
                parameters: true,
                group_addresses: true,
            }) => Self::PartialBoth,
            Some(_) => return None,
        })
    }
}

/// An optional value that must be written out: a missing key is not `null`.
fn required_option<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(deserializer)
}

fn graphic(text: &str, longest: usize) -> bool {
    !text.is_empty()
        && text.len() <= longest
        && text
            .bytes()
            .all(|b| b.is_ascii_graphic() && b != b'"' && b != b'\\')
}

fn lower_hex_sha256(text: &str) -> bool {
    text.len() == SHA256_HEX && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn hex4(text: &str) -> bool {
    text.len() == 4 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'A'..=b'F'))
}

/// The product program the plan was made from, as the catalogue named it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase", deny_unknown_fields)]
pub enum ProgramReceipt {
    #[serde(rename_all = "camelCase")]
    Identified {
        id: String,
        #[serde(deserialize_with = "required_option")]
        application_version: Option<String>,
        source_sha256: String,
    },
    Unavailable,
}

impl ProgramReceipt {
    fn valid(&self) -> bool {
        match self {
            Self::Identified {
                id,
                application_version,
                source_sha256,
            } => {
                graphic(id, MAX_PROGRAM_ID)
                    && application_version
                        .as_deref()
                        .is_none_or(|v| graphic(v, MAX_APPLICATION_VERSION))
                    && lower_hex_sha256(source_sha256)
            }
            Self::Unavailable => true,
        }
    }
}

/// What the run was planned to do. Planned values are not observed ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase", deny_unknown_fields)]
pub enum DownloadPreparation {
    #[serde(rename_all = "camelCase")]
    Available {
        format: Format1,
        scope: Scope,
        program: ProgramReceipt,
        planned_mask: String,
        planned_manufacturer: String,
        steps: u32,
        data_octets: u32,
    },
    /// Something needed for the plan summary was out of range; the scope stays.
    Unavailable { format: Format1, scope: Scope },
}

impl DownloadPreparation {
    /// The summary of `prepared`, with the program as read in the same
    /// catalogue lease. `None` for a plan that has no scope.
    pub fn from_prepared(prepared: &PreparedDownload) -> Option<Self> {
        use knx_core::commissioning::memory_download::MemoryDownloadStep;
        let scope = Scope::of(prepared.partial.as_ref().map(|(parts, _)| *parts))?;
        let writes = prepared.plan.steps.iter().filter_map(|step| match step {
            MemoryDownloadStep::WriteMemory { octets, .. } => Some(octets.len()),
            _ => None,
        });
        let Some((steps, data_octets)) = plan_counts(prepared.plan.steps.len(), writes) else {
            return Some(Self::Unavailable {
                format: Format1,
                scope,
            });
        };
        let program = program_receipt(&prepared.request.program_id, prepared.program.as_ref());
        Some(Self::Available {
            format: Format1,
            scope,
            program,
            planned_mask: format!("{:04X}", prepared.plan.mask.0),
            planned_manufacturer: format!("{:04X}", prepared.plan.manufacturer),
            steps,
            data_octets,
        })
    }

    /// The scope the run was planned for, whatever else is unavailable.
    pub fn scope(&self) -> Scope {
        match self {
            Self::Available { scope, .. } | Self::Unavailable { scope, .. } => *scope,
        }
    }

    fn valid(&self) -> bool {
        match self {
            Self::Available {
                program,
                planned_mask,
                planned_manufacturer,
                ..
            } => program.valid() && hex4(planned_mask) && hex4(planned_manufacturer),
            Self::Unavailable { .. } => true,
        }
    }
}

/// The identity as stored: only for the program the request named and only
/// within the bounds; anything else is unavailable, never refused or rewritten.
fn program_receipt(
    request_program_id: &str,
    identity: Option<&knx_productdb::query::ProgramIdentity>,
) -> ProgramReceipt {
    let Some(identity) = identity else {
        return ProgramReceipt::Unavailable;
    };
    let candidate = ProgramReceipt::Identified {
        id: identity.id.clone(),
        application_version: identity.application_version.clone(),
        source_sha256: identity.source_sha256.clone(),
    };
    if identity.id == request_program_id && candidate.valid() {
        candidate
    } else {
        ProgramReceipt::Unavailable
    }
}

/// The step and data-octet counts, or `None` where either leaves `u32`.
fn plan_counts(steps: usize, mut writes: impl Iterator<Item = usize>) -> Option<(u32, u32)> {
    let octets = writes.try_fold(0_u32, |sum, n| sum.checked_add(u32::try_from(n).ok()?))?;
    Some((u32::try_from(steps).ok()?, octets))
}

/// Four exact booleans, one per programming part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PartBits {
    pub address: bool,
    pub application: bool,
    pub parameters: bool,
    pub group_addresses: bool,
}

impl From<ProgrammingParts> for PartBits {
    fn from(parts: ProgrammingParts) -> Self {
        Self {
            address: parts.address,
            application: parts.application,
            parameters: parts.parameters,
            group_addresses: parts.group_addresses,
        }
    }
}

impl From<PartBits> for ProgrammingParts {
    fn from(bits: PartBits) -> Self {
        Self {
            address: bits.address,
            application: bits.application,
            parameters: bits.parameters,
            group_addresses: bits.group_addresses,
        }
    }
}

/// The settlement's three part sets. `programmed` and `pending` are disjoint
/// and together make `touched`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettledParts {
    pub programmed: PartBits,
    pub pending: PartBits,
    pub touched: PartBits,
}

impl From<&Settlement> for SettledParts {
    fn from(settlement: &Settlement) -> Self {
        Self {
            programmed: settlement.programmed.into(),
            pending: settlement.pending.into(),
            touched: settlement.touched.into(),
        }
    }
}

impl SettledParts {
    fn consistent(&self) -> bool {
        let (programmed, pending, touched) = (
            ProgrammingParts::from(self.programmed),
            ProgrammingParts::from(self.pending),
            ProgrammingParts::from(self.touched),
        );
        programmed.union(pending) == touched && programmed.intersection(pending).is_empty()
    }
}

/// Whether the product source still planned the plan that ran. This compares
/// plans, never source files: a replaced file can plan the same bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceVerdict {
    Unchanged,
    ChangedOrUnknown,
    NotChecked,
}

/// Whether the project still yielded a request for the device at the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CurrentRequest {
    Readable,
    Unavailable,
}

/// How the project belief was settled when the run ended. The project is the
/// one open at the start; its incarnation is not stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "project", rename_all = "camelCase", deny_unknown_fields)]
pub enum DownloadSettlement {
    /// The run wrote nothing, so no settlement was attempted.
    NotAttempted {
        format: Format1,
    },
    NoProject {
        format: Format1,
    },
    OtherProject {
        format: Format1,
    },
    DeviceMissing {
        format: Format1,
    },
    /// The settler could not run (the server state was gone).
    Unavailable {
        format: Format1,
    },
    /// Applied and written to the project file.
    #[serde(rename_all = "camelCase")]
    Persisted {
        format: Format1,
        source: SourceVerdict,
        current_request: CurrentRequest,
        applied: SettledParts,
        settled_at: String,
    },
    /// Applied in memory; the project has no file to write.
    #[serde(rename_all = "camelCase")]
    SessionOnly {
        format: Format1,
        source: SourceVerdict,
        current_request: CurrentRequest,
        applied: SettledParts,
        settled_at: String,
    },
    /// Computed, not applied: the project file could not be written.
    #[serde(rename_all = "camelCase")]
    PersistenceFailed {
        format: Format1,
        source: SourceVerdict,
        current_request: CurrentRequest,
        computed: SettledParts,
    },
}

/// How the project took the settlement, as the domain reported it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectOutcome {
    Persisted,
    SessionOnly,
    PersistenceFailed,
}

/// The canonical settlement instant: 24 bytes, milliseconds, `Z`.
pub fn canonical_time(at: DateTime<Utc>) -> Option<String> {
    let text = at.to_rfc3339_opts(SecondsFormat::Millis, true);
    canonical_time_text(&text).then_some(text)
}

fn canonical_time_text(text: &str) -> bool {
    // A leap second has no millisecond-UTC form the web reader would admit.
    text.len() == TIME_LENGTH
        && text.get(17..19) != Some("60")
        && DateTime::parse_from_rfc3339(text)
            .is_ok_and(|at| at.to_utc().to_rfc3339_opts(SecondsFormat::Millis, true) == text)
}

impl DownloadSettlement {
    /// A computed settlement and what the project did with it. A time that
    /// has no canonical form is an unavailable observation, not a made-up one.
    pub fn computed(
        outcome: ProjectOutcome,
        source: SourceVerdict,
        current_request: CurrentRequest,
        settlement: &Settlement,
        at: DateTime<Utc>,
    ) -> Self {
        let parts = SettledParts::from(settlement);
        match outcome {
            ProjectOutcome::PersistenceFailed => Self::PersistenceFailed {
                format: Format1,
                source,
                current_request,
                computed: parts,
            },
            ProjectOutcome::Persisted | ProjectOutcome::SessionOnly => {
                let Some(settled_at) = canonical_time(at) else {
                    return Self::Unavailable { format: Format1 };
                };
                if outcome == ProjectOutcome::Persisted {
                    Self::Persisted {
                        format: Format1,
                        source,
                        current_request,
                        applied: parts,
                        settled_at,
                    }
                } else {
                    Self::SessionOnly {
                        format: Format1,
                        source,
                        current_request,
                        applied: parts,
                        settled_at,
                    }
                }
            }
        }
    }

    /// Whether this settlement is consistent with the row it would sit in.
    /// `written` is the row's own witnessed value and `completed` whether the
    /// row is a finished, fully written transfer.
    fn valid(&self, written: Written, completed: bool, scope: Scope) -> bool {
        let (source, current_request, parts, time) = match self {
            Self::NotAttempted { .. } => return written == Written::No,
            Self::NoProject { .. }
            | Self::OtherProject { .. }
            | Self::DeviceMissing { .. }
            | Self::Unavailable { .. } => return written != Written::No,
            Self::Persisted {
                source,
                current_request,
                applied,
                settled_at,
                ..
            }
            | Self::SessionOnly {
                source,
                current_request,
                applied,
                settled_at,
                ..
            } => (*source, *current_request, applied, Some(settled_at)),
            Self::PersistenceFailed {
                source,
                current_request,
                computed,
                ..
            } => (*source, *current_request, computed, None),
        };
        if written == Written::No
            || time.is_some_and(|text| !canonical_time_text(text))
            || !parts.consistent()
        {
            return false;
        }
        let programmed = ProgrammingParts::from(parts.programmed);
        let touched = ProgrammingParts::from(parts.touched);
        let memory = scope.written_parts();
        if completed {
            let all = if scope == Scope::Complete {
                ProgrammingParts::ALL
            } else {
                memory
            };
            source != SourceVerdict::NotChecked
                && touched == all
                && (source == SourceVerdict::Unchanged
                    || programmed
                        .intersection(ProgrammingParts {
                            address: false,
                            ..ProgrammingParts::ALL
                        })
                        .is_empty())
                && (current_request == CurrentRequest::Readable || programmed.is_empty())
        } else {
            source == SourceVerdict::NotChecked
                && programmed.is_empty()
                && touched.intersection(memory) == touched
        }
    }
}

/// Whether a row's receipts agree with the row. Absence of either is valid
/// except that a settlement needs the preparation it settles.
pub(super) fn valid_receipts(
    state: &str,
    written: Option<Written>,
    preparation: Option<&DownloadPreparation>,
    settlement: Option<&DownloadSettlement>,
) -> bool {
    if preparation.is_some_and(|p| !p.valid()) {
        return false;
    }
    let Some(settlement) = settlement else {
        return true;
    };
    let (Some(preparation), Some(written)) = (preparation, written) else {
        return false;
    };
    let completed = state == "finished" && written == Written::Yes;
    matches!(state, "finished" | "failed")
        && settlement.valid(written, completed, preparation.scope())
}

#[cfg(test)]
mod tests;
