//! Source/profile-bound experimental preparation produces simulator bundles without device access.

mod project;
pub(crate) use project::check_budget;
pub use project::{
    describe_project_source_closure, prepare_project_simulated_candidate,
    replay_project_simulated_candidate, CoverageRecord, FlagsRecord, LinkRecord,
    ProjectPreparationRecord, ProjectProductInput, ProjectProductSelector,
    ProjectValidationProfile, RequestRecord, SourceClosureDescriptor, SourceMemberDescriptor,
    SourceMemberRole, SourceOwnerDescriptor,
};

use std::collections::BTreeMap;
use std::io;

use knx_core::commissioning::property_download::PropertyOperation;
use knx_core::{ContactableAddress, IndividualAddress};
use knx_productdb::image::{
    build_experimental_system2_image, ExperimentalSystem2ImagePolicy,
    ExperimentalSystem2SourceBinding, ImageRequest,
};
use knx_productdb::system2_procedure::{plan_system2_candidate, System2ProcedurePolicy};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::property_validation::Binding;
use crate::property_validation_bundle::{PlanRecord, PreparedBundle};

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// A versioned data profile, separate from manufacturer-independent implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationProfile {
    pub format: u16,
    pub id: String,
    pub version: u32,
    pub classification: String,
    pub configuration: String,
    pub program_id: String,
    pub schema_namespace: String,
    pub source_sha256: String,
    pub package_sha256: String,
    pub mask: u16,
    pub manufacturer: u16,
    pub hardware: [u8; 6],
    pub application: [u8; 5],
    pub numeric_order: String,
    pub bit_union_offsets: String,
    pub legacy_patch_always: String,
    pub invisible_parameters: String,
    pub binary_restrictions: String,
    pub reference_activation: String,
    pub procedure_policy: String,
}

impl ValidationProfile {
    /// Parse only the named complete-defaults experimental profile grammar.
    pub fn from_json(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() > 16 * 1024 {
            return Err(invalid("profile exceeds bounded size"));
        }
        let result: Self = serde_json::from_slice(bytes).map_err(io::Error::other)?;
        let hash = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if result.format != 1
            || result.version != 1
            || result.id.is_empty()
            || result.id.len() > 128
            || result.classification != "strong-hypothesis-simulator-validation-only"
            || result.configuration != "complete-defaults-no-links-v1"
            || result.procedure_policy != "declared-counts-zero-flags-full-table-identity-v1"
            || result.mask != 0x0025
            || !hash(&result.source_sha256)
            || !hash(&result.package_sha256)
            || result.application[..2] != result.manufacturer.to_be_bytes()
        {
            return Err(invalid("unrecognized or inconsistent candidate profile"));
        }
        Ok(result)
    }
}

fn check_identity(
    plan: &knx_core::commissioning::property_download::PropertyDownloadPlan,
    profile: &ValidationProfile,
) -> io::Result<()> {
    if plan.manufacturer() != profile.manufacturer || plan.mask().0 != profile.mask
        || !plan.steps().iter().any(|s| matches!(s.operation, PropertyOperation::HardwareCheck {expected} if expected == profile.hardware)) {
        return Err(invalid("candidate profile identity differs from translated source"));
    }
    let task = plan.steps().iter().find_map(|s| match &s.operation {
        PropertyOperation::LoadControl { object: 3, payload }
            if payload.octets()[..2] == [3, 2] =>
        {
            Some(&payload.octets()[5..])
        }
        _ => None,
    });
    if task != Some(profile.application.as_slice()) {
        return Err(invalid(
            "candidate application identity differs from translated source",
        ));
    }
    Ok(())
}

/// Complete candidate preparation, with explicit software-only evidence dimensions.
#[derive(Debug)]
pub struct PreparedCandidate {
    pub bundle: PreparedBundle,
    pub source_positions: usize,
    pub allocation_octets: usize,
    pub written_octets: usize,
    pub preserved_octets: usize,
    pub inferences: Vec<String>,
}

/// Reuse the production resolver/image builder and complete source translator.
///
/// This deliberately supports complete defaults with no project links/overrides.
/// The synthetic serial is visibly synthetic and can never authorize physical
/// use. No source hash, profile declaration or simulator result grants admission
/// to an actual target. The returned plan preserves all source masks.
pub fn prepare_simulated_candidate(
    conn: &knx_productdb::Connection,
    profile_bytes: &[u8],
    target: IndividualAddress,
) -> io::Result<PreparedCandidate> {
    ContactableAddress::new(target).map_err(|_| invalid("excluded candidate target"))?;
    let profile = ValidationProfile::from_json(profile_bytes)?;
    let request = ImageRequest {
        program_id: profile.program_id.clone(),
        individual_address: target,
        values: BTreeMap::new(),
        links: Vec::new(),
        flag_overrides: BTreeMap::new(),
    };
    let policy = ExperimentalSystem2ImagePolicy {
        revision: 1,
        binding: ExperimentalSystem2SourceBinding {
            program_id: profile.program_id.clone(),
            schema_namespace: profile.schema_namespace.clone(),
            source_sha256: profile.source_sha256.clone(),
            package_sha256: Some(profile.package_sha256.clone()),
        },
        numeric_order: profile.numeric_order.clone(),
        bit_union_offsets: profile.bit_union_offsets.clone(),
        legacy_patch_always: profile.legacy_patch_always.clone(),
        invisible_parameters: profile.invisible_parameters.clone(),
        binary_restrictions: profile.binary_restrictions.clone(),
        reference_activation: profile.reference_activation.clone(),
    };
    let image = build_experimental_system2_image(conn, &request, &policy)
        .map_err(|e| invalid(e.to_string()))?
        .image;
    let plan = plan_system2_candidate(
        &image,
        target,
        &System2ProcedurePolicy::declared_counts_zero_flags_full_table_identity(),
    )
    .map_err(|e| invalid(e.to_string()))?;
    check_identity(&plan, &profile)?;
    let normalized = PlanRecord::from_plan(&plan);
    let binding = Binding {
        target: target.to_string(),
        serial: [0x53, 0x49, 0x4D, 0, 0, 1],
        mask: plan.mask().0,
        manufacturer: plan.manufacturer(),
        hardware: profile.hardware,
        application: profile.application,
        profile_id: profile.id,
        profile_version: profile.version,
        profile_sha256: format!("{:x}", Sha256::digest(profile_bytes)),
        source_sha256: profile.source_sha256,
        plan_sha256: normalized.sha256()?,
        simulated: true,
        project_preparation_sha256: None,
    };
    let allocation_octets: usize = image.segments.iter().map(|s| s.octets.len()).sum();
    let written_octets: usize = plan
        .steps()
        .iter()
        .filter_map(|s| match &s.operation {
            PropertyOperation::WriteMemory { data, .. } => Some(data.len()),
            _ => None,
        })
        .sum();
    let source_positions = plan.source_positions();
    let mut inferences: Vec<_> = image
        .inferences
        .iter()
        .map(|i| format!("{}: {}", i.rule, i.detail))
        .collect();
    inferences.push("STRONG HYPOTHESIS: declared allocation counts; zero flags map to disabled checksum; every task segment carries the full application identity; exact tool metadata retained without wire write. Same-application reconstruction and masked-byte survival are simulator assumptions, not physical recovery evidence.".into());
    Ok(PreparedCandidate {
        bundle: PreparedBundle::new(binding, normalized)?,
        source_positions,
        allocation_octets,
        written_octets,
        preserved_octets: allocation_octets
            .checked_sub(written_octets)
            .ok_or_else(|| invalid("candidate coverage overflows"))?,
        inferences,
    })
}
