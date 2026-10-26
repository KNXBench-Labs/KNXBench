//! Immutable normalized bundles and exact operation-specific recovery serialization.
//!
//! A bundle is input, never physical admission: hashes detect accidental drift,
//! not provenance or malicious rewriting. Source/profile translation stays upstream.
use crate::property_validation::Binding;
use knx_core::commissioning::{
    load_control::{self, LoadControlSubtype},
    load_state::{LoadEvent, LoadState, MaskVersion},
    property_download::{PropertyDownloadPlan, PropertyDownloadStep, PropertyOperation},
};
use knx_core::{ContactableAddress, IndividualAddress};
use knx_net::commissioning::pointer_width::{PointerWidthAdmission, PropertyPointerValue};
use knx_net::commissioning::property_recovery::{
    ExperimentalPropertyPreconditions, PropertyPointerObservation, PropertyRecoveryRegion,
    PropertyRecoveryResource, PropertyRecoverySnapshot,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io;

pub(crate) fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanRecord {
    pub target: String,
    pub mask: u16,
    pub manufacturer: u16,
    pub steps: Vec<StepRecord>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepRecord {
    pub source_index: usize,
    pub operation: OperationRecord,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationRecord {
    Connect,
    HardwareCheck {
        expected: [u8; 6],
    },
    LoadControl {
        object: u8,
        octets: [u8; 10],
    },
    WriteMemory {
        object: u8,
        address: u16,
        data: Vec<u8>,
    },
    ToolMetadata {
        table_address: u16,
    },
    Disconnect,
}
impl PlanRecord {
    pub fn from_plan(plan: &PropertyDownloadPlan) -> Self {
        Self {
            target: plan.target().to_string(),
            mask: plan.mask().0,
            manufacturer: plan.manufacturer(),
            steps: plan
                .steps()
                .iter()
                .map(|s| StepRecord {
                    source_index: s.source_index,
                    operation: match &s.operation {
                        PropertyOperation::Connect => OperationRecord::Connect,
                        PropertyOperation::HardwareCheck { expected } => {
                            OperationRecord::HardwareCheck {
                                expected: *expected,
                            }
                        }
                        PropertyOperation::LoadControl { object, payload } => {
                            OperationRecord::LoadControl {
                                object: *object,
                                octets: *payload.octets(),
                            }
                        }
                        PropertyOperation::WriteMemory {
                            object,
                            address,
                            data,
                        } => OperationRecord::WriteMemory {
                            object: *object,
                            address: *address,
                            data: data.clone(),
                        },
                        PropertyOperation::ToolMetadata { table_address } => {
                            OperationRecord::ToolMetadata {
                                table_address: *table_address,
                            }
                        }
                        PropertyOperation::Disconnect => OperationRecord::Disconnect,
                    },
                })
                .collect(),
        }
    }
    pub fn sha256(&self) -> io::Result<String> {
        Ok(digest(&serde_json::to_vec(self).map_err(io::Error::other)?))
    }
    pub fn to_plan(&self) -> io::Result<PropertyDownloadPlan> {
        let target = self
            .target
            .parse::<IndividualAddress>()
            .map_err(|_| invalid("invalid normalized target"))?;
        ContactableAddress::new(target).map_err(|_| invalid("excluded normalized target"))?;
        let steps = self
            .steps
            .iter()
            .map(|s| {
                Ok(PropertyDownloadStep {
                    source_index: s.source_index,
                    operation: match &s.operation {
                        OperationRecord::Connect => PropertyOperation::Connect,
                        OperationRecord::Disconnect => PropertyOperation::Disconnect,
                        OperationRecord::HardwareCheck { expected } => {
                            PropertyOperation::HardwareCheck {
                                expected: *expected,
                            }
                        }
                        OperationRecord::WriteMemory {
                            object,
                            address,
                            data,
                        } => PropertyOperation::WriteMemory {
                            object: *object,
                            address: *address,
                            data: data.clone(),
                        },
                        OperationRecord::ToolMetadata { table_address } => {
                            PropertyOperation::ToolMetadata {
                                table_address: *table_address,
                            }
                        }
                        OperationRecord::LoadControl { object, octets } => {
                            // Rebuild only Core's represented payload classes, then validate
                            // exact byte equality and the complete immutable plan below.
                            let payload = match octets[0] {
                                1 => load_control::event_payload(LoadEvent::StartLoading),
                                2 => load_control::event_payload(LoadEvent::LoadCompleted),
                                4 => load_control::event_payload(LoadEvent::Unload),
                                3 => {
                                    let subtype = match octets[1] {
                                        0 => LoadControlSubtype::AllocAbsDataSeg,
                                        1 => LoadControlSubtype::AllocAbsStackSeg,
                                        2 => LoadControlSubtype::AllocAbsTaskSeg,
                                        3 => LoadControlSubtype::TaskPtr,
                                        4 => LoadControlSubtype::TaskCtrl1,
                                        5 => LoadControlSubtype::TaskCtrl2,
                                        _ => {
                                            return Err(invalid(
                                                "unsupported stored control subtype",
                                            ))
                                        }
                                    };
                                    load_control::additional_load_control(
                                        subtype,
                                        octets[2..]
                                            .try_into()
                                            .map_err(|_| invalid("stored control width"))?,
                                    )
                                }
                                _ => return Err(invalid("unsupported stored load event")),
                            };
                            if payload.octets() != octets {
                                return Err(invalid("stored control is not exact"));
                            }
                            PropertyOperation::LoadControl {
                                object: *object,
                                payload,
                            }
                        }
                    },
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        PropertyDownloadPlan::new(target, MaskVersion(self.mask), self.manufacturer, steps)
            .map_err(|_| invalid("stored property plan is invalid"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedBundle {
    pub format: u16,
    pub binding: Binding,
    pub normalized_plan: PlanRecord,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_preparation:
        Option<Box<crate::system2_validation_preparation::ProjectPreparationRecord>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardware_preparation:
        Option<Box<crate::system2_validation_preparation::ProjectHardwareRecord>>,
}
impl PreparedBundle {
    pub fn new(binding: Binding, normalized_plan: PlanRecord) -> io::Result<Self> {
        let bundle = Self {
            format: 1,
            binding,
            normalized_plan,
            project_preparation: None,
            hardware_preparation: None,
        };
        bundle.plan()?;
        Ok(bundle)
    }
    pub(crate) fn new_project(
        binding: Binding,
        normalized_plan: PlanRecord,
        record: crate::system2_validation_preparation::ProjectPreparationRecord,
    ) -> io::Result<Self> {
        let bundle = Self {
            format: 2,
            binding,
            normalized_plan,
            project_preparation: Some(Box::new(record)),
            hardware_preparation: None,
        };
        bundle.plan()?;
        Ok(bundle)
    }
    pub fn plan(&self) -> io::Result<PropertyDownloadPlan> {
        self.binding.validate()?;
        match (
            self.format,
            &self.project_preparation,
            &self.binding.project_preparation_sha256,
            &self.hardware_preparation,
        ) {
            (1, None, None, None) => {
                // Defaults profile v1 (superseded) or v2; ValidationProfile binds each policy.
                let defaults = match self.binding.profile_version {
                    1 => true,
                    2 => self
                        .binding
                        .profile_id
                        .ends_with(crate::system2_validation_preparation::DEFAULTS_V2_ID_SUFFIX),
                    _ => false,
                };
                if !defaults {
                    return Err(invalid("unsupported legacy validation profile version"));
                }
            }
            (2, Some(record), Some(_), None) => {
                crate::system2_validation_preparation::check_budget(self)?;
                record.validate(&self.binding, &self.normalized_plan)?;
            }
            (3, Some(record), Some(_), Some(hardware)) => {
                crate::system2_validation_preparation::check_budget(self)?;
                hardware.validate(&self.binding, &self.normalized_plan, record)?;
            }
            _ => return Err(invalid("bundle format/project preparation mixing")),
        }
        if self.normalized_plan.sha256()? != self.binding.plan_sha256
            || self.normalized_plan.target != self.binding.target
            || self.normalized_plan.mask != self.binding.mask
            || self.normalized_plan.manufacturer != self.binding.manufacturer
        {
            return Err(invalid("bundle format or exact plan binding differs"));
        }
        let plan = self.normalized_plan.to_plan()?;
        if !plan.steps().iter().any(|s| matches!(&s.operation, PropertyOperation::HardwareCheck { expected } if *expected == self.binding.hardware)) {
            return Err(invalid("bundle hardware identity differs from plan"));
        }
        Ok(plan)
    }
    /// The bound recovery premise, including the PID 7 width rule this bundle's hardware
    /// record admits. Format 1/2 and revision-1 hardware records stay four-octet only.
    pub fn preconditions(&self) -> io::Result<ExperimentalPropertyPreconditions> {
        self.preconditions_for(&self.plan()?)
    }
    /// Same as `preconditions` for a plan the caller already reconstructed and validated,
    /// so a fresh execution context still pays for one retained-source reconstruction.
    pub(crate) fn preconditions_for(
        &self,
        plan: &PropertyDownloadPlan,
    ) -> io::Result<ExperimentalPropertyPreconditions> {
        let premise = ExperimentalPropertyPreconditions::same_application_reinstallation(
            plan,
            self.binding.serial,
            self.binding.application,
        )
        .map_err(|_| invalid("same-application recovery premise cannot represent this plan"))?;
        let Some(policy_objects) = self
            .hardware_preparation
            .as_ref()
            .and_then(|h| h.pointer_width_objects())
        else {
            return Ok(premise);
        };
        let touched = premise
            .source_pointers
            .iter()
            .map(|p| p.object)
            .filter(|object| policy_objects.contains(object))
            .collect();
        let admission = PointerWidthAdmission::one_element_two_or_four(touched)
            .map_err(|_| invalid("pointer width policy covers no object of this plan"))?;
        premise
            .with_pointer_width_admission(admission)
            .map_err(|_| invalid("pointer width policy does not bind this plan"))
    }
    /// Rebind a simulated-prepared candidate to a physical device identity.
    ///
    /// The serial must be read from the real device beforehand (never guessed).
    /// The result is marked `simulated: false` and revalidated; the plan,
    /// profile and source hashes are unchanged — only the device binding and
    /// the recovery premise serial differ. See ADR-0152.
    pub fn rebind_hardware(&self, serial: [u8; 6]) -> io::Result<PreparedBundle> {
        if self.format != 1
            || self.project_preparation.is_some()
            || self.binding.project_preparation_sha256.is_some()
        {
            return Err(invalid(
                "project preparation is intrinsically simulator-only",
            ));
        }
        let mut bundle = self.clone();
        bundle.binding.serial = serial;
        bundle.binding.simulated = false;
        bundle.plan()?;
        bundle.preconditions()?;
        Ok(bundle)
    }
    pub fn consent_phrase(&self) -> String {
        if let Some(hardware) = &self.hardware_preparation {
            return format!(
                "I confirm configured System-2 {} to {} preparation {}",
                hardware.operation, self.binding.target, hardware.source_preparation_sha256
            );
        }
        format!(
            "I confirm property validation to {} plan {}",
            self.binding.target, self.binding.plan_sha256
        )
    }
    pub fn acceptance_phrase(&self) -> String {
        format!(
            "I accept STRONG HYPOTHESIS to {} plan {}",
            self.binding.target, self.binding.plan_sha256
        )
    }
    pub fn restore_phrase(&self) -> String {
        if let Some(hardware) = &self.hardware_preparation {
            return format!(
                "I confirm configured System-2 recovery {} to {} preparation {}",
                hardware.operation,
                self.binding.target,
                self.binding
                    .project_preparation_sha256
                    .as_deref()
                    .unwrap_or("INVALID")
            );
        }
        format!(
            "I confirm property recovery to {} plan {}",
            self.binding.target, self.binding.plan_sha256
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceRecord {
    pub object: u8,
    pub state: u8,
    pub pointer: PointerRecord,
    /// The device refused the PID_TABLE_REFERENCE read; `pointer` is then zero.
    /// Absent (Alpha 16 grammar) when a pointer was read.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pointer_refused: bool,
}
/// A raw PID 7 reply as stored: a four-element array is today's grammar, a two-element
/// array is a two-octet reply that only a width-admitting hardware record may carry.
/// Any other length matches neither variant and is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PointerRecord {
    Four([u8; 4]),
    Two([u8; 2]),
}
impl From<[u8; 4]> for PointerRecord {
    fn from(octets: [u8; 4]) -> Self {
        Self::Four(octets)
    }
}
impl From<PropertyPointerValue> for PointerRecord {
    fn from(value: PropertyPointerValue) -> Self {
        match value {
            PropertyPointerValue::Four(octets) => Self::Four(octets),
            PropertyPointerValue::Two(octets) => Self::Two(octets),
        }
    }
}
impl From<PointerRecord> for PropertyPointerValue {
    fn from(record: PointerRecord) -> Self {
        match record {
            PointerRecord::Four(octets) => Self::Four(octets),
            PointerRecord::Two(octets) => Self::Two(octets),
        }
    }
}
impl PointerRecord {
    pub fn octets(&self) -> &[u8] {
        match self {
            Self::Four(octets) => octets,
            Self::Two(octets) => octets,
        }
    }
    /// Refuse a two-octet reply on an object the admission does not cover.
    fn check_admitted(&self, object: u8, admission: &PointerWidthAdmission) -> io::Result<()> {
        if matches!(self, Self::Two(_)) && !admission.applies_to(object) {
            return Err(invalid(
                "two-octet pointer is not admitted by this bundle's width policy",
            ));
        }
        Ok(())
    }
}

/// Readable per-object pointer observation: source expectation versus device read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PointerObservationRecord {
    pub object: u8,
    pub expected: [u8; 4],
    /// `None` when the device refused the read.
    pub observed: Option<PointerRecord>,
    pub state: u8,
}
impl PointerObservationRecord {
    pub fn from_observation(observation: &PropertyPointerObservation) -> Self {
        Self {
            object: observation.object,
            expected: observation.expected,
            observed: observation.observed.map(PointerRecord::from),
            state: observation.state.octet(),
        }
    }
    /// Refuses a two-octet read the bundle's width admission does not cover.
    pub fn to_observation(
        &self,
        admission: &PointerWidthAdmission,
    ) -> io::Result<PropertyPointerObservation> {
        if let Some(observed) = &self.observed {
            observed.check_admitted(self.object, admission)?;
        }
        Ok(PropertyPointerObservation {
            object: self.object,
            expected: self.expected,
            observed: self.observed.map(PropertyPointerValue::from),
            state: LoadState::from_octet(self.state)
                .map_err(|_| invalid("pointer observation load state is unknown"))?,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionRecord {
    pub operation_index: usize,
    pub source_index: usize,
    pub object: u8,
    pub address: u16,
    pub data: Vec<u8>,
}
/// Stores the *original* source procedure including every allocation/task control;
/// region originals replace only its exact data payloads during reinstallation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryRecord {
    pub version: u16,
    pub original_bundle: PreparedBundle,
    pub old_device_control: u8,
    /// Observed prestate per touched object (equal to the source unless it differed).
    pub resources: Vec<ResourceRecord>,
    pub allocations: Vec<RegionRecord>,
    pub regions: Vec<RegionRecord>,
    /// Redundant readable copy of every resource that differed from the source
    /// shape; absent (Alpha 16 grammar) when none did. Revalidated on load.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prestate_observations: Vec<PointerObservationRecord>,
    /// Device serial read at acquisition when it differed from the binding's;
    /// absent (Alpha 16 grammar) when equal. Recovery replay is bound to it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_serial: Option<[u8; 6]>,
    /// Object 3 PID_PROGRAM_VERSION read at acquisition when it differed from
    /// the plan's application (empty: refused); absent when equal (Alpha 16 grammar).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_program_version: Option<Vec<u8>>,
    /// Allocation indices whose prestate read was refused or short; their data
    /// are zero placeholders. Absent (Alpha 16 grammar) when every read succeeded.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prestate_unreadable_allocations: Vec<usize>,
}
impl RecoveryRecord {
    pub fn from_snapshot(
        snapshot: &PropertyRecoverySnapshot,
        bundle: &PreparedBundle,
    ) -> io::Result<Self> {
        if snapshot.original_plan != bundle.plan()?
            || snapshot.preconditions != bundle.preconditions()?
        {
            return Err(invalid(
                "acquired recovery does not bind the exact prepared bundle",
            ));
        }
        let record = Self {
            version: snapshot.version,
            original_bundle: bundle.clone(),
            old_device_control: snapshot.old_device_control,
            resources: snapshot
                .resources
                .iter()
                .map(|r| ResourceRecord {
                    object: r.object,
                    state: r.state.octet(),
                    pointer: r.pointer.map_or(PointerRecord::Four([0; 4]), Into::into),
                    pointer_refused: r.pointer.is_none(),
                })
                .collect(),
            allocations: snapshot
                .allocations
                .iter()
                .map(|r| RegionRecord {
                    operation_index: r.operation_index,
                    source_index: r.source_index,
                    object: r.object,
                    address: r.address,
                    data: r.data.clone(),
                })
                .collect(),
            regions: snapshot
                .regions
                .iter()
                .map(|r| RegionRecord {
                    operation_index: r.operation_index,
                    source_index: r.source_index,
                    object: r.object,
                    address: r.address,
                    data: r.data.clone(),
                })
                .collect(),
            prestate_observations: snapshot
                .prestate_observations()
                .iter()
                .map(PointerObservationRecord::from_observation)
                .collect(),
            observed_serial: snapshot.serial_observation().map(|o| o.observed),
            observed_program_version: snapshot.program_version_observation().map(|o| o.observed),
            prestate_unreadable_allocations: snapshot.unreadable_allocations.clone(),
        };
        record.to_snapshot(bundle)?;
        Ok(record)
    }
    pub fn to_snapshot(&self, bundle: &PreparedBundle) -> io::Result<PropertyRecoverySnapshot> {
        if self.version != 1 || self.original_bundle != *bundle {
            return Err(invalid(
                "recovery source/application/profile/plan binding differs",
            ));
        }
        let plan = bundle.plan()?;
        let preconditions = bundle.preconditions()?;
        if self.resources.len() != preconditions.source_pointers.len()
            || self
                .resources
                .iter()
                .zip(&preconditions.source_pointers)
                .any(|(a, b)| {
                    a.object != b.object
                        || LoadState::from_octet(a.state).is_err()
                        || (a.pointer_refused && a.pointer != PointerRecord::Four([0; 4]))
                })
        {
            return Err(invalid(
                "recovery resource/control coverage differs from source",
            ));
        }
        let writes: Vec<_> = plan
            .steps()
            .iter()
            .enumerate()
            .filter_map(|(i, s)| match &s.operation {
                PropertyOperation::WriteMemory {
                    object,
                    address,
                    data,
                } => Some((i, s.source_index, *object, *address, data.len())),
                _ => None,
            })
            .collect();
        if self.regions.len() != writes.len()
            || self.regions.iter().zip(writes).any(|(r, (i, s, o, a, n))| {
                (
                    r.operation_index,
                    r.source_index,
                    r.object,
                    r.address,
                    r.data.len(),
                ) != (i, s, o, a, n)
            })
        {
            return Err(invalid(
                "recovery memory coverage is incomplete or reordered",
            ));
        }
        for resource in &self.resources {
            resource
                .pointer
                .check_admitted(resource.object, &preconditions.pointer_widths)?;
        }
        if self.observed_serial == Some(preconditions.serial)
            || self
                .observed_program_version
                .as_ref()
                .is_some_and(|v| v.as_slice() == preconditions.program_version || v.len() > 64)
        {
            return Err(invalid(
                "observed serial/program version is recorded only when it differs from the binding",
            ));
        }
        let snapshot = PropertyRecoverySnapshot {
            version: self.version,
            original_plan: plan.clone(),
            observed_serial: self.observed_serial.unwrap_or(preconditions.serial),
            observed_program_version: self
                .observed_program_version
                .clone()
                .unwrap_or_else(|| preconditions.program_version.to_vec()),
            preconditions,
            old_device_control: self.old_device_control,
            unreadable_allocations: self.prestate_unreadable_allocations.clone(),
            resources: self
                .resources
                .iter()
                .map(|r| PropertyRecoveryResource {
                    object: r.object,
                    state: LoadState::from_octet(r.state).expect("load state octet checked above"),
                    pointer: (!r.pointer_refused).then_some(r.pointer.into()),
                })
                .collect(),
            allocations: self
                .allocations
                .iter()
                .map(|r| PropertyRecoveryRegion {
                    operation_index: r.operation_index,
                    source_index: r.source_index,
                    object: r.object,
                    address: r.address,
                    data: r.data.clone(),
                })
                .collect(),
            regions: self
                .regions
                .iter()
                .map(|r| PropertyRecoveryRegion {
                    operation_index: r.operation_index,
                    source_index: r.source_index,
                    object: r.object,
                    address: r.address,
                    data: r.data.clone(),
                })
                .collect(),
        };
        // Evidence of a differing prestate loads; recovery_plan() refuses it later.
        snapshot
            .validate_evidence(&plan)
            .map_err(|_| invalid("recovery allocation coverage or original bytes differ"))?;
        let observations = self
            .prestate_observations
            .iter()
            .map(|record| record.to_observation(&snapshot.preconditions.pointer_widths))
            .collect::<io::Result<Vec<_>>>()?;
        if observations != snapshot.prestate_observations() {
            return Err(invalid(
                "recorded prestate observations differ from captured resources",
            ));
        }
        Ok(snapshot)
    }
}
