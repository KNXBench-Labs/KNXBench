//! Retained supplied-domain project preparation and bounded offline reproduction.
use super::{invalid, PreparedCandidate};
use crate::property_validation::Binding;
use crate::property_validation_bundle::{digest, PlanRecord, PreparedBundle};
use knx_core::{ContactableAddress, DeviceId, Project};
use knx_productdb::image::{
    build_project_experimental_system2_image, ExperimentalParameterCoverage,
    ExperimentalParameterDisposition, ExperimentalSystem2ImagePolicy,
    ExperimentalSystem2SourceBinding, ImageRequest,
};
use knx_productdb::system2_procedure::{plan_system2_candidate, System2ProcedurePolicy};
use knx_store::project_history::NativeSnapshot;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Read, Write},
};

const CONFIGURATION: &str = "project-parameters-links-flags-v1";
const PROCEDURE: &str = "source-capacity-preserving-independent-data-v1";
const SCOPE: &str = "complete-supplied-domain-project-without-infrastructure-owned-data-v1";
const LIMIT: usize = 16 * 1024 * 1024;

mod profile;
mod retained_bytes;
mod source;
pub use profile::{ProjectProductSelector, ProjectValidationProfile};
pub use source::{
    describe_project_source_closure, ProjectProductInput, SourceClosureDescriptor,
    SourceMemberDescriptor, SourceMemberRole, SourceOwnerDescriptor,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestRecord {
    pub program_id: String,
    pub individual_address: u16,
    pub values: BTreeMap<String, String>,
    pub links: Vec<LinkRecord>,
    pub flag_overrides: BTreeMap<String, FlagsRecord>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkRecord {
    pub object: u8,
    pub group_address: u16,
    pub sending: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlagsRecord {
    pub read: Option<bool>,
    pub write: Option<bool>,
    pub transmit: Option<bool>,
    pub update: Option<bool>,
    pub communication: Option<bool>,
    pub read_on_init: Option<bool>,
}
impl RequestRecord {
    pub fn from_request(request: &ImageRequest) -> Self {
        Self {
            program_id: request.program_id.clone(),
            individual_address: request.individual_address.raw(),
            values: request.values.clone(),
            links: request
                .links
                .iter()
                .map(|l| LinkRecord {
                    object: l.object,
                    group_address: l.group_address.raw(),
                    sending: l.sending,
                })
                .collect(),
            flag_overrides: request
                .flag_overrides
                .iter()
                .map(|(id, f)| {
                    (
                        id.clone(),
                        FlagsRecord {
                            read: f.read,
                            write: f.write,
                            transmit: f.transmit,
                            update: f.update,
                            communication: f.communication,
                            read_on_init: f.read_on_init,
                        },
                    )
                })
                .collect(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageRecord {
    pub parameter_id: String,
    pub references: Vec<String>,
    pub source_default: String,
    pub segment: Option<String>,
    pub field: Option<[u32; 3]>,
    pub legacy_patch_always: bool,
    pub default_union_parameter: bool,
    pub active_reference: Option<String>,
    pub disposition: String,
}
impl From<&ExperimentalParameterCoverage> for CoverageRecord {
    fn from(c: &ExperimentalParameterCoverage) -> Self {
        Self {
            parameter_id: c.parameter_id.clone(),
            references: c.references.clone(),
            source_default: c.source_default.clone(),
            segment: c.segment.clone(),
            field: c
                .field
                .map(|f| [f.offset, u32::from(f.bit_offset), f.size_in_bit]),
            legacy_patch_always: c.legacy_patch_always,
            default_union_parameter: c.default_union_parameter,
            active_reference: c.active_reference.clone(),
            disposition: match c.disposition {
                ExperimentalParameterDisposition::Virtual => "virtual",
                ExperimentalParameterDisposition::RetainedSourceTemplate => {
                    "retained-source-template"
                }
                ExperimentalParameterDisposition::BackgroundDefault => "background-default",
                ExperimentalParameterDisposition::LegacyPatchDefaultReference => {
                    "legacy-patch-default-reference"
                }
                ExperimentalParameterDisposition::DefaultUnionBase => "default-union-base",
                ExperimentalParameterDisposition::InactiveUnionAlternative => {
                    "inactive-union-alternative"
                }
                ExperimentalParameterDisposition::ActiveOverlay => "active-overlay",
            }
            .into(),
        }
    }
}
/// Supplied Project plus winning-source/package reproduction, never observed hardware state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectPreparationRecord {
    pub revision: u16,
    pub scope: String,
    pub device_id: u32,
    pub device_source_path: String,
    pub device_source_id: String,
    #[serde(with = "retained_bytes::bytes")]
    pub project_snapshot: Vec<u8>,
    pub snapshot_sha256: String,
    pub project_semantic_sha256: String,
    pub request: RequestRecord,
    #[serde(with = "retained_bytes::bytes")]
    pub profile_bytes: Vec<u8>,
    pub profile_sha256: String,
    pub schema_namespace: String,
    pub source_sha256: String,
    pub product_input: ProjectProductInput,
    pub configuration_policy: String,
    pub plan_sha256: String,
    pub coverage: Vec<CoverageRecord>,
}
impl ProjectPreparationRecord {
    pub fn sha256(&self) -> io::Result<String> {
        #[derive(Serialize)]
        struct Identity<'a> {
            domain: &'static str,
            revision: u16,
            scope: &'a str,
            project_semantic_sha256: &'a str,
            snapshot_sha256: &'a str,
            device_id: u32,
            device_source_path: &'a str,
            device_source_id: &'a str,
            request: &'a RequestRecord,
            profile_sha256: &'a str,
            schema_namespace: &'a str,
            source_sha256: &'a str,
            product_input: source::ProductInputIdentity<'a>,
            configuration_policy: &'a str,
            plan_sha256: &'a str,
            coverage: &'a [CoverageRecord],
        }
        let identity = Identity {
            domain: "knxbench-project-system2-preparation-v2-explicit-product-input",
            revision: self.revision,
            scope: &self.scope,
            project_semantic_sha256: &self.project_semantic_sha256,
            snapshot_sha256: &self.snapshot_sha256,
            device_id: self.device_id,
            device_source_path: &self.device_source_path,
            device_source_id: &self.device_source_id,
            request: &self.request,
            profile_sha256: &self.profile_sha256,
            schema_namespace: &self.schema_namespace,
            source_sha256: &self.source_sha256,
            product_input: self.product_input.identity(),
            configuration_policy: &self.configuration_policy,
            plan_sha256: &self.plan_sha256,
            coverage: &self.coverage,
        };
        Ok(digest(
            &serde_json::to_vec(&identity).map_err(io::Error::other)?,
        ))
    }
    pub fn decoded_project(&self) -> io::Result<Project> {
        let snapshot = NativeSnapshot::decode(&self.project_snapshot, &self.snapshot_sha256)
            .map_err(io::Error::other)?;
        if !snapshot.opaque.is_empty()
            || !snapshot.manufacturer_refs.is_empty()
            || snapshot.semantic_hash().map_err(io::Error::other)? != self.project_semantic_sha256
        {
            return Err(invalid("retained supplied-domain project identity differs"));
        }
        Ok(snapshot.project)
    }
    pub(crate) fn validate(&self, binding: &Binding, plan: &PlanRecord) -> io::Result<()> {
        if self.revision != 2
            || self.scope != SCOPE
            || self.configuration_policy != CONFIGURATION
            || !binding.simulated
            || binding.serial != [0x53, 0x49, 0x4d, 0, 0, 1]
            || binding.project_preparation_sha256.as_ref() != Some(&self.sha256()?)
            || self.profile_sha256 != digest(&self.profile_bytes)
            || self.plan_sha256 != plan.sha256()?
            || self.plan_sha256 != binding.plan_sha256
            || self.profile_sha256 != binding.profile_sha256
            || self.source_sha256 != binding.source_sha256
        {
            return Err(invalid(
                "project preparation record or retained digest differs",
            ));
        }
        let profile = ProjectValidationProfile::from_json(&self.profile_bytes)?;
        if profile.program_id != self.request.program_id
            || profile.schema_namespace != self.schema_namespace
            || profile.source_sha256 != self.source_sha256
            || profile.id != binding.profile_id
            || profile.version != binding.profile_version
            || profile.mask != binding.mask
            || profile.manufacturer != binding.manufacturer
            || profile.hardware != binding.hardware
            || profile.application != binding.application
            || knx_core::IndividualAddress::from_raw(self.request.individual_address).to_string()
                != binding.target
        {
            return Err(invalid(
                "project profile/request/source/device identity differs",
            ));
        }
        let project = self.decoded_project()?;
        let device = project
            .devices
            .get(DeviceId(self.device_id))
            .ok_or_else(|| invalid("retained project device missing"))?;
        if device.source.path != self.device_source_path
            || device.source.ets_id != self.device_source_id
        {
            return Err(invalid("retained project device source differs"));
        }
        let rebuilt = reconstruct(
            &self.product_input,
            &project,
            DeviceId(self.device_id),
            &profile,
        )?;
        if rebuilt.request != self.request
            || rebuilt.normalized != *plan
            || rebuilt.coverage != self.coverage
        {
            return Err(invalid(
                "retained source reconstruction request/plan/coverage differs",
            ));
        }
        Ok(())
    }
}
fn reject_duplicate_overrides(
    project: &Project,
    device: DeviceId,
    program: &str,
) -> io::Result<()> {
    use knx_core::provenance::Override;
    let instance = project
        .devices
        .get(device)
        .ok_or_else(|| invalid("unknown project device"))?;
    let mut identities = BTreeSet::new();
    for id in &instance.com_objects {
        let object = project
            .devices
            .com_object(*id)
            .ok_or_else(|| invalid("missing project object"))?;
        let f = &object.flags;
        let stated = [
            &f.read,
            &f.write,
            &f.transmit,
            &f.update,
            &f.communication,
            &f.read_on_init,
        ]
        .into_iter()
        .any(|f| match f {
            Override::Value(v) => v.layer.is_exported(),
            Override::Absent => false,
            _ => true,
        });
        if stated
            && !identities.insert(knx_productdb::enrich::com_object_lookup_id(
                program,
                &object.source.ets_id,
                object.module_instance.is_some(),
            ))
        {
            return Err(invalid("duplicate normalized flag-override reference"));
        }
    }
    Ok(())
}
fn retained_blob(conn: &knx_productdb::Connection, table: &str, sha: &str) -> io::Result<Vec<u8>> {
    // Only internal constant table names reach this helper; check before materializing.
    let length: i64 = conn
        .query_row(
            &format!("SELECT length(bytes) FROM {table} WHERE sha256=?1"),
            [sha],
            |r| r.get(0),
        )
        .map_err(io::Error::other)?;
    if length < 0 || length > LIMIT as i64 {
        return Err(invalid("retained input exceeds bundle budget"));
    }
    conn.query_row(
        &format!("SELECT bytes FROM {table} WHERE sha256=?1"),
        [sha],
        |r| r.get(0),
    )
    .map_err(io::Error::other)
}
struct Reconstruction {
    request: RequestRecord,
    normalized: PlanRecord,
    coverage: Vec<CoverageRecord>,
    source_positions: usize,
    allocation_octets: usize,
    written_octets: usize,
    inferences: Vec<String>,
}
fn build(
    conn: &knx_productdb::Connection,
    project: &Project,
    device: DeviceId,
    profile: &ProjectValidationProfile,
) -> io::Result<Reconstruction> {
    reject_duplicate_overrides(project, device, &profile.program_id)?;
    let request = knx_productdb::image_request::image_request_for_device(conn, project, device)
        .map_err(io::Error::other)?;
    if request.program_id != profile.program_id {
        return Err(invalid("resolved project program differs from profile"));
    }
    ContactableAddress::new(request.individual_address)
        .map_err(|_| invalid("excluded candidate target"))?;
    let built = build_project_experimental_system2_image(conn, &request, &profile.policy())
        .map_err(|e| invalid(e.to_string()))?;
    let plan = plan_system2_candidate(
        &built.image,
        request.individual_address,
        &System2ProcedurePolicy::source_capacity_preserving_independent_data(),
    )
    .map_err(io::Error::other)?;
    profile.check_identity(&plan)?;
    let written_octets = plan
        .steps()
        .iter()
        .filter_map(|s| match &s.operation {
            knx_core::commissioning::property_download::PropertyOperation::WriteMemory {
                data,
                ..
            } => Some(data.len()),
            _ => None,
        })
        .sum();
    Ok(Reconstruction {
        request: RequestRecord::from_request(&request),
        normalized: PlanRecord::from_plan(&plan),
        coverage: built.coverage.iter().map(CoverageRecord::from).collect(),
        source_positions: plan.source_positions(),
        allocation_octets: built.image.segments.iter().map(|s| s.octets.len()).sum(),
        written_octets,
        inferences: built
            .image
            .inferences
            .iter()
            .map(|i| format!("{}: {}", i.rule, i.detail))
            .collect(),
    })
}
// Test-local, synchronous scopes only: count entry at the actual reconstruction
// seam, not plan accessors or a predicted journal/telegram count. No production
// counter/cache, and no scope or thread-local borrow crosses an await.
#[cfg(test)]
thread_local! {
    static RECONSTRUCTIONS: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}
#[cfg(test)]
pub(crate) fn count_reconstructions<R>(f: impl FnOnce() -> R) -> (R, usize) {
    struct Reset(Option<usize>);
    impl Drop for Reset {
        fn drop(&mut self) {
            RECONSTRUCTIONS.set(self.0);
        }
    }
    let _reset = Reset(RECONSTRUCTIONS.replace(Some(0)));
    let result = f();
    (
        result,
        RECONSTRUCTIONS.get().expect("active reconstruction count"),
    )
}

/// Nonrecursive authority for initial preparation, record admission and explicit replay.
fn reconstruct(
    input: &ProjectProductInput,
    project: &Project,
    device: DeviceId,
    profile: &ProjectValidationProfile,
) -> io::Result<Reconstruction> {
    #[cfg(test)]
    RECONSTRUCTIONS.set(RECONSTRUCTIONS.get().map(|count| count + 1));
    let scratch = tempfile::tempdir()?;
    let conn = knx_productdb::open_and_migrate(&scratch.path().join("products.sqlite"))
        .map_err(io::Error::other)?;
    input.install(&conn, project, device, profile)?;
    build(&conn, project, device, profile)
}
fn candidate(bundle: PreparedBundle, built: Reconstruction) -> io::Result<PreparedCandidate> {
    Ok(PreparedCandidate {
        bundle,
        source_positions: built.source_positions,
        allocation_octets: built.allocation_octets,
        written_octets: built.written_octets,
        preserved_octets: built
            .allocation_octets
            .checked_sub(built.written_octets)
            .ok_or_else(|| invalid("candidate coverage overflows"))?,
        inferences: built.inferences,
    })
}
/// Offline only. Pins live winners, evidence and originals in one read snapshot.
pub fn prepare_project_simulated_candidate(
    conn: &knx_productdb::Connection,
    project: &Project,
    device: DeviceId,
    profile_bytes: &[u8],
) -> io::Result<PreparedCandidate> {
    let profile = ProjectValidationProfile::from_json(profile_bytes)?;
    let tx = conn.unchecked_transaction().map_err(io::Error::other)?;
    let input = match &profile.product_input {
        ProjectProductSelector::StandalonePackage { package_sha256 } => {
            let source_bytes = retained_blob(&tx, "source_file", &profile.source_sha256)?;
            let package_bytes = retained_blob(&tx, "package", package_sha256)?;
            let source_member = tx
                .query_row(
                    "SELECT path FROM package_member WHERE package_sha256=?1 AND source_sha256=?2",
                    [package_sha256, &profile.source_sha256],
                    |r| r.get(0),
                )
                .map_err(io::Error::other)?;
            ProjectProductInput::StandalonePackage {
                source_member,
                source_bytes,
                package_bytes,
                package_sha256: package_sha256.clone(),
            }
        }
        ProjectProductSelector::RetainedSourceMembers { closure_sha256 } => {
            let (descriptor, member_bytes) =
                source::collect(&tx, project, device, &profile.schema_namespace)?;
            if descriptor.sha256()? != *closure_sha256 {
                return Err(invalid(
                    "caller-pinned source closure differs from winning-source snapshot",
                ));
            }
            ProjectProductInput::RetainedSourceMembers {
                descriptor: Box::new(descriptor),
                member_bytes,
                closure_sha256: closure_sha256.clone(),
            }
        }
    };
    input.matches(&profile)?;
    let built = build(&tx, project, device, &profile)?;
    // Reproduction cannot borrow any live rows absent from the retained input.
    let replay = reconstruct(&input, project, device, &profile)?;
    if replay.request != built.request
        || replay.normalized != built.normalized
        || replay.coverage != built.coverage
    {
        return Err(invalid(
            "retained product input did not reproduce live preparation",
        ));
    }
    let snapshot = NativeSnapshot {
        project: project.clone(),
        opaque: vec![],
        manufacturer_refs: vec![],
    };
    let project_snapshot = snapshot.encode().map_err(io::Error::other)?;
    let instance = project
        .devices
        .get(device)
        .ok_or_else(|| invalid("unknown project device"))?;
    let record = ProjectPreparationRecord {
        revision: 2,
        scope: SCOPE.into(),
        device_id: device.0,
        device_source_path: instance.source.path.clone(),
        device_source_id: instance.source.ets_id.clone(),
        snapshot_sha256: digest(&project_snapshot),
        project_semantic_sha256: snapshot.semantic_hash().map_err(io::Error::other)?,
        project_snapshot,
        request: built.request.clone(),
        profile_bytes: profile_bytes.to_vec(),
        profile_sha256: digest(profile_bytes),
        schema_namespace: profile.schema_namespace.clone(),
        source_sha256: profile.source_sha256.clone(),
        product_input: input,
        configuration_policy: CONFIGURATION.into(),
        plan_sha256: built.normalized.sha256()?,
        coverage: built.coverage.clone(),
    };
    let binding = Binding {
        target: knx_core::IndividualAddress::from_raw(built.request.individual_address).to_string(),
        serial: [0x53, 0x49, 0x4d, 0, 0, 1],
        mask: profile.mask,
        manufacturer: profile.manufacturer,
        hardware: profile.hardware,
        application: profile.application,
        profile_id: profile.id,
        profile_version: profile.version,
        profile_sha256: record.profile_sha256.clone(),
        source_sha256: record.source_sha256.clone(),
        plan_sha256: record.plan_sha256.clone(),
        simulated: true,
        project_preparation_sha256: Some(record.sha256()?),
    };
    let bundle = PreparedBundle::new_project(binding, built.normalized.clone(), record)?;
    candidate(bundle, built)
}
/// Rebuild only from retained originals and supplied-domain snapshot, without a live DB.
pub fn replay_project_simulated_candidate(
    bundle: &PreparedBundle,
) -> io::Result<PreparedCandidate> {
    bundle.plan()?;
    let retained = bundle
        .project_preparation
        .as_ref()
        .ok_or_else(|| invalid("not a project bundle"))?;
    let project = retained.decoded_project()?;
    let profile = ProjectValidationProfile::from_json(&retained.profile_bytes)?;
    let reproduced = reconstruct(
        &retained.product_input,
        &project,
        DeviceId(retained.device_id),
        &profile,
    )?;
    if reproduced.request != retained.request
        || reproduced.normalized != bundle.normalized_plan
        || reproduced.coverage != retained.coverage
    {
        return Err(invalid(
            "retained project preparation did not reproduce exactly",
        ));
    }
    candidate(bundle.clone(), reproduced)
}
/// Count encoded evidence without an unbounded serialization allocation. Reserve a
/// conservative recovery envelope, including all possible original byte octets.
pub(crate) fn check_budget(bundle: &PreparedBundle) -> io::Result<()> {
    struct Counter(usize);
    impl Write for Counter {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.0 = self
                .0
                .checked_add(b.len())
                .filter(|n| *n <= LIMIT)
                .ok_or_else(|| invalid("project bundle exceeds recovery envelope budget"))?;
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let plan = bundle.normalized_plan.to_plan()?;
    // Absolute allocations encode their lengths in octets 4..6. Sum all,
    // including unwritten masked/capacity tails; original recovery retains both.
    let mut memory = 0usize;
    for step in plan.steps() {
        match &step.operation {
            knx_core::commissioning::property_download::PropertyOperation::WriteMemory {
                data,
                ..
            } => memory = memory.saturating_add(data.len()),
            knx_core::commissioning::property_download::PropertyOperation::LoadControl {
                payload,
                ..
            } if payload.octets()[0] == 3 && payload.octets()[1] <= 1 => {
                memory = memory.saturating_add(usize::from(u16::from_be_bytes([
                    payload.octets()[4],
                    payload.octets()[5],
                ])))
            }
            _ => {}
        }
    }
    // Count the retained bundle at its real three-level nesting depth in the
    // protocol's PRETTY RecoveryAcquired envelope (not compact bundle JSON).
    #[derive(Serialize)]
    struct EnvelopeBudget<'a> {
        checkpoint: CheckpointBudget<'a>,
    }
    #[derive(Serialize)]
    struct CheckpointBudget<'a> {
        recovery: RecoveryBudget<'a>,
    }
    #[derive(Serialize)]
    struct RecoveryBudget<'a> {
        original_bundle: &'a PreparedBundle,
    }
    // 1 MiB fixed slack plus 1 KiB/operation covers envelope/resource metadata.
    // Each original byte costs <64 characters even at the protocol's deepest
    // pretty indentation, including commas/newlines and a three-digit octet.
    let overhead = 1024usize
        .saturating_mul(1024)
        .saturating_add(plan.steps().len().saturating_mul(1024))
        .saturating_add(memory.saturating_mul(64));
    let mut count = Counter(overhead);
    let envelope = EnvelopeBudget {
        checkpoint: CheckpointBudget {
            recovery: RecoveryBudget {
                original_bundle: bundle,
            },
        },
    };
    serde_json::to_writer_pretty(&mut count, &envelope).map_err(io::Error::other)?;
    if count.0 > LIMIT {
        return Err(invalid("project bundle exceeds recovery envelope budget"));
    }
    Ok(())
}
