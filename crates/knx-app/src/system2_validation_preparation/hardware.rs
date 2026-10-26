//! Separate configured physical preparation; source candidates grant no write authority.
use super::{
    invalid, prepare_project_simulated_candidate, ProjectPreparationRecord,
    ProjectValidationProfile,
};
use crate::property_validation::Binding;
use crate::property_validation_bundle::{digest, PlanRecord, PreparedBundle};
use knx_core::commissioning::{
    authorisation::AuthorisationPlan,
    mutation::TargetKind,
    properties::{ObjectIndex, PID_HARDWARE_TYPE, PID_PROGRAM_VERSION},
};
use knx_core::{DeviceId, Project};
use knx_net::{commissioning::SessionTiming, management::ManagementTransport, ManagementSession};
use serde::{Deserialize, Serialize};
use std::{
    io,
    sync::atomic::{AtomicBool, Ordering},
};

fn supported_profile() -> io::Result<super::ValidationProfile> {
    super::ValidationProfile::from_json(include_bytes!(
        "../../../../data/system2-validation/up258d12-920504-v1.json"
    ))
}
/// Append-only id of the PID 7 width rule of revision-2 records; never reinterpreted.
/// A source-bound experimental policy: object 1 two-octet width was seen on one device,
/// objects 2 and 3 rely on the public package declaration only (unverified).
pub(crate) const POINTER_WIDTH_POLICY_V1: &str =
    "up258-mv0025-pid7-objects123-one-element-two-or-four-octet-source-declared-v1";
/// Objects the policy names; a bundle admits the subset its plan touches.
const POINTER_WIDTH_POLICY_OBJECTS: [u8; 3] = [1, 2, 3];
const BASIS: &str = "experimental-same-application-source-shape-not-observed-allocator-history-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecoveryTrack {
    ExternalEts,
    KnxBenchBestEffort,
}

/// Source-only stage, produced freshly from a Project, never from an uploaded bundle.
/// May be prepared on a blocking worker; it contains no DB handle across an await.
#[derive(Clone)]
pub struct ProjectHardwareSource {
    bundle: PreparedBundle,
    operation: String,
    recovery: RecoveryTrack,
}

/// Identity observed through a read-only management session. Not a pre-write backup.
/// Mask, manufacturer and hardware type must match the source; serial and current
/// program version are observations only (owner rulings 2026-10-25).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HardwareObservation {
    pub target: String,
    pub serial: [u8; 6],
    pub mask: u16,
    pub manufacturer: u16,
    pub hardware: [u8; 6],
    /// The plan's application identity, which the download writes.
    pub application: [u8; 5],
    /// The device's current PID_PROGRAM_VERSION when it differed from
    /// `application` (empty: read refused); absent when equal (Alpha 16 grammar).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_application: Option<Vec<u8>>,
}

/// Separate versioned hardware admission evidence; v2 project candidates remain unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectHardwareRecord {
    pub revision: u16,
    pub operation: String,
    pub recovery_track: RecoveryTrack,
    pub recovery_basis: String,
    pub observation: HardwareObservation,
    pub source_preparation_sha256: String,
    /// Revision 2 only: the exact policy id. Absent on revision 1, whose bytes and
    /// digest stay as they were; `null` is refused rather than read as absent.
    #[serde(
        default,
        deserialize_with = "some_policy",
        skip_serializing_if = "Option::is_none"
    )]
    pub pointer_width_policy: Option<String>,
}
fn some_policy<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}
impl ProjectHardwareRecord {
    /// Objects whose PID 7 answers may be one element of two or four octets; `None` for
    /// the historical four-octet-only revision 1.
    pub(crate) fn pointer_width_objects(&self) -> Option<&'static [u8]> {
        (self.pointer_width_policy.as_deref() == Some(POINTER_WIDTH_POLICY_V1))
            .then_some(&POINTER_WIDTH_POLICY_OBJECTS[..])
    }
    pub fn sha256(&self) -> io::Result<String> {
        Ok(digest(
            &serde_json::to_vec(&("knxbench-configured-system2-hardware-v1", self))
                .map_err(io::Error::other)?,
        ))
    }
    pub(crate) fn validate(
        &self,
        binding: &Binding,
        plan: &PlanRecord,
        source: &ProjectPreparationRecord,
    ) -> io::Result<()> {
        valid_operation(&self.operation)?;
        exact_profile(&source.profile_bytes)?;
        let policy_grammar = match self.revision {
            1 => self.pointer_width_policy.is_none(),
            2 => self.pointer_width_policy.as_deref() == Some(POINTER_WIDTH_POLICY_V1),
            _ => false,
        };
        if !policy_grammar
            || self.recovery_basis != BASIS
            || binding.simulated
            || self.source_preparation_sha256 != source.sha256()?
            || binding.project_preparation_sha256.as_ref() != Some(&self.sha256()?)
            || self.observation
                != observation_for(binding, self.observation.observed_application.clone())
            || self
                .observation
                .observed_application
                .as_ref()
                .is_some_and(|observed| {
                    observed.as_slice() == binding.application || observed.len() > 64
                })
            // Only the synthetic marker guards against a simulator preparation
            // posing as real. Zero/all-FF serials are observations (amendment 3);
            // recovery is unavailable for them, not the download.
            || self.observation.serial == [0x53, 0x49, 0x4d, 0, 0, 1]
        {
            return Err(invalid(
                "configured hardware observation/operation/source binding differs",
            ));
        }
        // Validate the retained SOURCE using its unchanged simulator grammar.
        // This does not admit a simulator bundle: only format 3 carries this separate record.
        let mut source_binding = binding.clone();
        source_binding.simulated = true;
        source_binding.serial = [0x53, 0x49, 0x4d, 0, 0, 1];
        source_binding.project_preparation_sha256 = Some(source.sha256()?);
        source.validate(&source_binding, plan)
    }
}
fn observation_for(
    binding: &Binding,
    observed_application: Option<Vec<u8>>,
) -> HardwareObservation {
    HardwareObservation {
        target: binding.target.clone(),
        serial: binding.serial,
        mask: binding.mask,
        manufacturer: binding.manufacturer,
        hardware: binding.hardware,
        application: binding.application,
        observed_application,
    }
}
fn valid_operation(operation: &str) -> io::Result<()> {
    if operation.is_empty()
        || operation.len() > 128
        || !operation
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
        return Err(invalid(
            "operation id must be 1..128 ASCII letters/digits or -_.",
        ));
    }
    Ok(())
}
fn exact_profile(bytes: &[u8]) -> io::Result<()> {
    let p = ProjectValidationProfile::from_json(bytes)?;
    let supported = supported_profile()?;
    if p.program_id != supported.program_id
        || p.mask != supported.mask
        || p.manufacturer != supported.manufacturer
        || p.hardware != supported.hardware
        || p.application != supported.application
    {
        return Err(invalid(
            "configured hardware is limited to exact UP258 M-0001_A-9205-04-9270 identity",
        ));
    }
    Ok(())
}
/// UI/default configured entry: select an explicit retained-source closure, never a package/default fallback.
pub fn prepare_retained_project_hardware_source(
    conn: &knx_productdb::Connection,
    project: &Project,
    device: DeviceId,
    operation: &str,
    recovery: RecoveryTrack,
) -> io::Result<ProjectHardwareSource> {
    valid_operation(operation)?;
    let mut admitted = Vec::new();
    for namespace in [
        "http://knx.org/xml/project/20",
        "http://knx.org/xml/project/23",
    ] {
        if let Ok(descriptor) =
            super::describe_project_source_closure(conn, project, device, namespace)
        {
            admitted.push(descriptor);
        }
    }
    if admitted.len() != 1 {
        return Err(invalid("exact retained source closure unavailable, unsupported or ambiguous; no package/default fallback"));
    }
    let descriptor = admitted
        .pop()
        .ok_or_else(|| invalid("missing retained source descriptor"))?;
    let supported = supported_profile()?;
    if descriptor.program_id != supported.program_id {
        return Err(invalid(
            "retained hardware source is not the exact UP258 application",
        ));
    }
    let application = &descriptor.members[1];
    let profile = ProjectValidationProfile {
        format: 2,
        id: "up258d12-configured-v2".into(),
        version: 2,
        classification: "strong-hypothesis-simulator-validation-only".into(),
        configuration: "project-parameters-links-flags-v1".into(),
        program_id: supported.program_id,
        schema_namespace: application.schema_namespace.clone(),
        source_sha256: application.source_sha256.clone(),
        product_input: super::ProjectProductSelector::RetainedSourceMembers {
            closure_sha256: descriptor.sha256()?,
        },
        mask: supported.mask,
        manufacturer: supported.manufacturer,
        hardware: supported.hardware,
        application: supported.application,
        numeric_order: "high-octet-first-explicit-or-schema-default-v1".into(),
        bit_union_offsets: "msb0-byte-aligned-union-relative-v1".into(),
        legacy_patch_always: "inactive-default-ref-consensus-active-wins-v1".into(),
        invisible_parameters: "source-template-inactive-legacy-patch-active-overlay-v1".into(),
        binary_restrictions: "single-octet-enumeration-binary-payload-v1".into(),
        reference_activation: "schema10-no-dynamic-reference-is-active-v1".into(),
        procedure_policy: "source-capacity-preserving-independent-data-v1".into(),
    };
    prepare_project_hardware_source(
        conn,
        project,
        device,
        &serde_json::to_vec(&profile).map_err(io::Error::other)?,
        operation,
        recovery,
    )
}
/// Fresh Project → request → source image → ordered plan, including configured links/flags.
/// This stage is offline, read-only and grants no hardware authority.
pub fn prepare_project_hardware_source(
    conn: &knx_productdb::Connection,
    project: &Project,
    device: DeviceId,
    profile: &[u8],
    operation: &str,
    recovery: RecoveryTrack,
) -> io::Result<ProjectHardwareSource> {
    valid_operation(operation)?;
    exact_profile(profile)?;
    let bundle = prepare_project_simulated_candidate(conn, project, device, profile)?.bundle;
    Ok(ProjectHardwareSource {
        bundle,
        operation: operation.into(),
        recovery,
    })
}
impl ProjectHardwareSource {
    /// Offline preview only. Returning a shared reference cannot relabel the candidate.
    pub fn preview(&self) -> &PreparedBundle {
        &self.bundle
    }
    pub fn operation(&self) -> &str {
        &self.operation
    }
    /// Selecting a recovery track before observation grants no write authority.
    pub fn with_recovery_track(mut self, recovery: RecoveryTrack) -> Self {
        self.recovery = recovery;
        self
    }
    pub fn preview_summary(&self) -> io::Result<super::ConfiguredHardwarePreview> {
        super::configured_hardware_preview::project_hardware_preview(&self.bundle, &self.operation)
    }
    /// Check all operator phrases while still OFFLINE, before acquiring a tunnel.
    pub fn validate_confirmations(
        &self,
        confirmations: &HardwareConfirmations<'_>,
    ) -> io::Result<()> {
        let preview = self.preview_summary()?;
        if confirmations.operation != preview.confirmation_phrase
            || confirmations.hypothesis != preview.hypothesis_phrase
            || confirmations.download != preview.download_phrase
            || confirmations.risk != preview.release_phrase
        {
            return Err(invalid("exact configured preparation, hypothesis, download and risk confirmations required"));
        }
        Ok(())
    }
    /// Rebuild against current live winners before acquiring a gateway lease/tunnel.
    pub fn check_current(
        &self,
        conn: &knx_productdb::Connection,
        project: &Project,
        device: DeviceId,
    ) -> io::Result<()> {
        let source = self
            .bundle
            .project_preparation
            .as_ref()
            .ok_or_else(|| invalid("missing project source"))?;
        let fresh = prepare_project_hardware_source(
            conn,
            project,
            device,
            &source.profile_bytes,
            &self.operation,
            self.recovery,
        )?;
        if fresh.bundle.binding != self.bundle.binding {
            return Err(invalid("configured source preparation is stale"));
        }
        Ok(())
    }
}

/// Observe exact target identity, then construct a NEW operation-bound physical record.
/// No device-changing service is used. Caller owns the existing gateway lease/tunnel.
/// Hardware pre-state is acquired again by execution, not taken from this observation.
pub async fn observe_project_hardware<T: ManagementTransport>(
    transport: &T,
    source: ProjectHardwareSource,
    timing: SessionTiming,
    cancel: &AtomicBool,
) -> io::Result<PreparedBundle> {
    if transport.target_kind() != TargetKind::Hardware {
        return Err(invalid(
            "hardware observation requires a hardware-kind transport",
        ));
    }
    if cancel.load(Ordering::SeqCst) {
        return Err(invalid("hardware preparation cancelled before connection"));
    }
    let mut bundle = source.bundle;
    let target = bundle.plan()?.target();
    let mut session =
        ManagementSession::read_only(transport, target, AuthorisationPlan::Skip, timing)
            .map_err(io::Error::other)?;
    session.connect().await.map_err(io::Error::other)?;
    let observed = async {
        let serial = session
            .read_serial_number()
            .await
            .map_err(io::Error::other)?
            .octets();
        let mask = session
            .read_mask_version()
            .await
            .map_err(io::Error::other)?
            .0;
        let manufacturer = session
            .read_manufacturer_id()
            .await
            .map_err(io::Error::other)?;
        let hardware = session
            .read_property(ObjectIndex::DEVICE, PID_HARDWARE_TYPE, 1, 1)
            .await
            .map_err(io::Error::other)?;
        // An unloaded device may refuse the read; that is an observation too.
        let application = match session
            .read_property(ObjectIndex::APPLICATION_PROGRAM, PID_PROGRAM_VERSION, 1, 1)
            .await
        {
            Ok(application) => application,
            Err(knx_net::commissioning::SessionError::PropertyRefused { .. }) => Vec::new(),
            Err(error) => return Err(io::Error::other(error)),
        };
        if mask != bundle.binding.mask
            || manufacturer != bundle.binding.manufacturer
            || hardware.as_slice() != bundle.binding.hardware
        {
            return Err(invalid(
                "observed hardware/mask/manufacturer differs from configured source",
            ));
        }
        if cancel.load(Ordering::SeqCst) {
            return Err(invalid("hardware preparation cancelled"));
        }
        Ok((serial, application))
    }
    .await;
    session.disconnect().await;
    let (serial, application) = observed?;
    bundle.binding.serial = serial;
    if application.len() > 64 {
        return Err(invalid("program version response exceeds its bound"));
    }
    let observed_application =
        (application.as_slice() != bundle.binding.application).then_some(application);
    bundle.binding.simulated = false;
    let record = ProjectHardwareRecord {
        revision: 2,
        operation: source.operation,
        recovery_track: source.recovery,
        recovery_basis: BASIS.into(),
        observation: observation_for(&bundle.binding, observed_application),
        source_preparation_sha256: bundle
            .project_preparation
            .as_ref()
            .ok_or_else(|| invalid("missing project source"))?
            .sha256()?,
        pointer_width_policy: Some(POINTER_WIDTH_POLICY_V1.into()),
    };
    bundle.binding.project_preparation_sha256 = Some(record.sha256()?);
    bundle.hardware_preparation = Some(Box::new(record));
    bundle.format = 3;
    bundle.plan()?;
    bundle.preconditions()?;
    Ok(bundle)
}

/// Opaque freshness witness, built from the *current* supplied project and live product DB.
/// It owns validated immutable bytes and crosses blocking/async service boundaries.
pub struct CurrentProjectHardware {
    bundle: PreparedBundle,
}
impl CurrentProjectHardware {
    pub fn bundle(&self) -> &PreparedBundle {
        &self.bundle
    }
}

pub struct HardwareConfirmations<'a> {
    pub operation: &'a str,
    pub hypothesis: &'a str,
    pub download: &'a str,
    pub risk: &'a str,
}
pub struct HardwareRecoveryConfirmations<'a> {
    pub recovery: &'a str,
    pub download: &'a str,
    pub risk: &'a str,
}
pub async fn execute_project_hardware<T: ManagementTransport>(
    transport: &T,
    admission: &CurrentProjectHardware,
    store: &mut crate::property_validation::OperationStore,
    timing: SessionTiming,
    confirmations: &HardwareConfirmations<'_>,
    cancel: &AtomicBool,
) -> io::Result<()> {
    let execution = crate::property_validation_execution::ValidatedHardwareExecution::new(
        &admission.bundle,
        store,
    )?;
    execution.check_confirmations(confirmations)?;
    crate::property_validation_execution::execute_hardware_inner(
        transport,
        execution,
        timing,
        confirmations.download,
        confirmations.risk,
        cancel,
    )
    .await
}
pub async fn restore_project_hardware<T: ManagementTransport>(
    transport: &T,
    bundle: &PreparedBundle,
    store: &mut crate::property_validation::OperationStore,
    timing: SessionTiming,
    confirmations: &HardwareRecoveryConfirmations<'_>,
    cancel: &AtomicBool,
) -> io::Result<()> {
    validate_project_hardware_recovery(bundle, store, confirmations)?;
    crate::property_validation_execution::restore_hardware_inner(
        transport,
        bundle,
        store,
        timing,
        confirmations.recovery,
        confirmations.download,
        confirmations.risk,
        cancel,
    )
    .await
}
/// Pre-tunnel, read-only admission of the exact original backup and separate recovery consent.
pub fn validate_project_hardware_recovery(
    bundle: &PreparedBundle,
    store: &crate::property_validation::OperationStore,
    confirmations: &HardwareRecoveryConfirmations<'_>,
) -> io::Result<()> {
    let plan = bundle.plan()?;
    if bundle.format != 3
        || bundle.hardware_preparation.is_none()
        || store.binding() != &bundle.binding
    {
        return Err(invalid(
            "separate configured hardware preparation and matching operation required",
        ));
    }
    store.require_restore_available()?;
    let snapshot = store
        .recovery()
        .ok_or_else(|| invalid("original actual prestate unavailable"))?
        .to_snapshot(bundle)?;
    crate::property_validation_execution::refuse_unavailable_recovery(&snapshot, bundle)?;
    if confirmations.recovery != bundle.restore_phrase()
        || confirmations.download
            != knx_core::commissioning::mutation::required_confirmation_phrase(
                plan.target(),
                knx_core::commissioning::mutation::WriteScope::Download,
            )
        || confirmations.risk
            != knx_net::commissioning::hardware_release::required_release_phrase(plan.target())
    {
        return Err(invalid(
            "exact recovery, ordinary download and best-effort risk confirmations required",
        ));
    }
    Ok(())
}
impl crate::property_validation_execution::ValidatedHardwareExecution<'_> {
    pub(crate) fn check_confirmations(
        &self,
        confirmations: &HardwareConfirmations<'_>,
    ) -> io::Result<()> {
        let bundle = self.bundle();
        let plan = self.plan();
        if bundle.format != 3 || bundle.hardware_preparation.is_none() {
            return Err(invalid("separate configured hardware preparation required"));
        }
        if confirmations.operation != bundle.consent_phrase()
            || confirmations.hypothesis != bundle.acceptance_phrase()
            || confirmations.download
                != knx_core::commissioning::mutation::required_confirmation_phrase(
                    plan.target(),
                    knx_core::commissioning::mutation::WriteScope::Download,
                )
            || confirmations.risk
                != knx_net::commissioning::hardware_release::required_release_phrase(plan.target())
        {
            return Err(invalid("exact operation/preparation, ordinary download and best-effort risk confirmations required"));
        }
        Ok(())
    }
}

/// Rebuild with current winners immediately before start; never compare only plan hashes.
pub fn admit_current_project_hardware(
    conn: &knx_productdb::Connection,
    project: &Project,
    device: DeviceId,
    bundle: &PreparedBundle,
) -> io::Result<CurrentProjectHardware> {
    bundle.plan()?;
    let hardware = bundle
        .hardware_preparation
        .as_ref()
        .ok_or_else(|| invalid("separate configured hardware preparation required"))?;
    let source = bundle
        .project_preparation
        .as_ref()
        .ok_or_else(|| invalid("missing configured project source"))?;
    let fresh = prepare_project_hardware_source(
        conn,
        project,
        device,
        &source.profile_bytes,
        &hardware.operation,
        hardware.recovery_track,
    )?;
    if fresh
        .bundle
        .project_preparation
        .as_ref()
        .ok_or_else(|| invalid("missing fresh project source"))?
        .sha256()?
        != hardware.source_preparation_sha256
        || fresh.bundle.normalized_plan != bundle.normalized_plan
    {
        return Err(invalid(
            "configured hardware preparation is stale: project/selection/source/request changed",
        ));
    }
    Ok(CurrentProjectHardware {
        bundle: bundle.clone(),
    })
}
