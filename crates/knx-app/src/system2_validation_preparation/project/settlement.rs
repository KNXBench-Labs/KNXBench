//! Settlement-only caption proof retains original evidence and never constructs hardware authority.
use super::*;
use crate::system2_validation_preparation::hardware::prepare_project_hardware_source;
use knx_productdb::package::{settlement_inventory, PackageLimits};
use knx_productdb::settlement_caption::caption_value_only;

#[cfg(test)]
#[path = "settlement_tests.rs"]
mod tests;

pub(in crate::system2_validation_preparation) struct Comparison {
    pub profile: Vec<u8>,
    pub input: ProjectProductInput,
}

fn closure_from_original(
    source: &ProjectPreparationRecord,
    project: &Project,
    device: DeviceId,
    profile: &ProjectValidationProfile,
) -> io::Result<(SourceClosureDescriptor, [Vec<u8>; 2])> {
    match &source.product_input {
        ProjectProductInput::RetainedSourceMembers {
            descriptor,
            member_bytes,
            ..
        } => Ok((descriptor.as_ref().clone(), member_bytes.clone())),
        input @ ProjectProductInput::StandalonePackage { .. } => {
            let scratch = tempfile::tempdir()?;
            let conn = knx_productdb::open_and_migrate(&scratch.path().join("products.sqlite"))
                .map_err(io::Error::other)?;
            input.install(&conn, project, device, profile)?;
            source::collect(&conn, project, device, &profile.schema_namespace)
        }
    }
}

fn same_closure(old: &SourceClosureDescriptor, new: &SourceClosureDescriptor) -> io::Result<()> {
    // Compare every descriptor field. Only actual application byte identity and its
    // measured element digest can change. Hardware and every owner bridge stay exact.
    let mut comparison = new.clone();
    comparison.members[1].source_sha256 = old.members[1].source_sha256.clone();
    comparison.members[1].byte_len = old.members[1].byte_len;
    comparison.members[1].owner.element_digest = old.members[1].owner.element_digest.clone();
    if comparison != *old || new.members[1].source_sha256 == old.members[1].source_sha256 {
        return Err(invalid(
            "caption source closure identity differs or source unchanged",
        ));
    }
    Ok(())
}

fn package_input(
    conn: &knx_productdb::Connection,
    old: &ProjectProductInput,
    new_source: &[u8],
    source_sha: &str,
) -> io::Result<ProjectProductInput> {
    let ProjectProductInput::StandalonePackage {
        source_member,
        package_bytes: old_bytes,
        ..
    } = old
    else {
        return Err(invalid("selector family differs"));
    };
    // Count actual hashes, not aliases; do not filter missing/bad/path-mismatched candidates.
    // A second candidate is already ambiguity, regardless of its validity or contents.
    let mut query = conn
        .prepare(
            "SELECT DISTINCT package_sha256 FROM package_member WHERE source_sha256=?1 LIMIT 2",
        )
        .map_err(io::Error::other)?;
    let candidates = query
        .query_map([source_sha], |r| r.get::<_, String>(0))
        .map_err(io::Error::other)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(io::Error::other)?;
    let [sha] = candidates.as_slice() else {
        return Err(invalid("current standalone package missing or ambiguous"));
    };
    let mut query = conn
        .prepare(
            "SELECT path FROM package_member WHERE source_sha256=?1 AND package_sha256=?2 LIMIT 2",
        )
        .map_err(io::Error::other)?;
    let paths = query
        .query_map([source_sha, sha], |r| r.get::<_, String>(0))
        .map_err(io::Error::other)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(io::Error::other)?;
    if paths.len() != 1 || paths[0] != *source_member {
        return Err(invalid("current standalone source membership differs"));
    }
    let bytes = retained_blob(conn, "package", sha)?;
    if digest(&bytes) != *sha {
        return Err(invalid("current package digest differs"));
    }
    let limits = PackageLimits {
        max_member_size: LIMIT as u64,
        max_expanded_size: LIMIT as u64,
    };
    let old_inventory = settlement_inventory(old_bytes, limits).map_err(io::Error::other)?;
    let mut new_inventory = settlement_inventory(&bytes, limits).map_err(io::Error::other)?;
    if new_inventory.len() != old_inventory.len() {
        return Err(invalid("package entry inventory differs"));
    }
    let mut selected = 0;
    for (old, new) in old_inventory.iter().zip(&mut new_inventory) {
        if old.name == *source_member {
            if old.directory || new.bytes != new_source {
                return Err(invalid("selected package member differs"));
            }
            selected += 1;
            new.bytes = old.bytes.clone();
        }
        if old != new {
            return Err(invalid("package entry identity/order/content differs"));
        }
    }
    if selected != 1 {
        return Err(invalid("selected package member unavailable"));
    }
    Ok(ProjectProductInput::StandalonePackage {
        source_member: source_member.clone(),
        source_bytes: new_source.to_vec(),
        package_bytes: bytes,
        package_sha256: sha.clone(),
    })
}

/// Requires independently validated original format-3 evidence and a typed stale-source phase.
/// All reads/preparations stay inside the caller's project/command-stack/managed-owner lease.
pub(in crate::system2_validation_preparation) fn compare(
    conn: &knx_productdb::Connection,
    original: &Project,
    device: DeviceId,
    bundle: &PreparedBundle,
) -> io::Result<Comparison> {
    let source = bundle
        .project_preparation
        .as_ref()
        .ok_or_else(|| invalid("missing original source"))?;
    let hardware = bundle
        .hardware_preparation
        .as_ref()
        .ok_or_else(|| invalid("missing original hardware"))?;
    let profile = ProjectValidationProfile::from_json(&source.profile_bytes)?;
    let (old_descriptor, old_bytes) = closure_from_original(source, original, device, &profile)?;
    // Short transaction only; preparation later opens its own transaction, never a nested one.
    let (descriptor, member_bytes) = {
        let tx = conn.unchecked_transaction().map_err(io::Error::other)?;
        source::collect(&tx, original, device, &profile.schema_namespace)?
    };
    same_closure(&old_descriptor, &descriptor)?;
    if old_bytes[0] != member_bytes[0] {
        return Err(invalid("hardware source changed"));
    }
    source::validate_xml(&old_bytes[1], &profile.schema_namespace)?;
    source::validate_xml(&member_bytes[1], &profile.schema_namespace)?;
    caption_value_only(&old_bytes[1], &member_bytes[1], &profile.program_id)?;
    let input = match &source.product_input {
        old @ ProjectProductInput::StandalonePackage { .. } => package_input(
            conn,
            old,
            &member_bytes[1],
            &descriptor.members[1].source_sha256,
        )?,
        ProjectProductInput::RetainedSourceMembers { .. } => {
            ProjectProductInput::RetainedSourceMembers {
                closure_sha256: descriptor.sha256()?,
                descriptor: Box::new(descriptor.clone()),
                member_bytes,
            }
        }
    };
    let mut new_profile = profile.clone();
    new_profile.source_sha256 = descriptor.members[1].source_sha256.clone();
    new_profile.product_input = match &input {
        ProjectProductInput::StandalonePackage { package_sha256, .. } => {
            ProjectProductSelector::StandalonePackage {
                package_sha256: package_sha256.clone(),
            }
        }
        ProjectProductInput::RetainedSourceMembers { closure_sha256, .. } => {
            ProjectProductSelector::RetainedSourceMembers {
                closure_sha256: closure_sha256.clone(),
            }
        }
    };
    let bytes = serde_json::to_vec(&new_profile).map_err(io::Error::other)?;
    let mut checked_profile = ProjectValidationProfile::from_json(&bytes)?;
    checked_profile.source_sha256 = profile.source_sha256.clone();
    checked_profile.product_input = profile.product_input.clone();
    if checked_profile != profile {
        return Err(invalid("non-provenance profile differs"));
    }
    let fresh = prepare_project_hardware_source(
        conn,
        original,
        device,
        &bytes,
        &hardware.operation,
        hardware.recovery_track,
    )?;
    let new_bundle = fresh.preview();
    let record = new_bundle
        .project_preparation
        .as_ref()
        .ok_or_else(|| invalid("missing comparison preparation"))?;
    if record.product_input != input {
        return Err(invalid("new source recollection differs"));
    }
    compare_prepared_records(
        source,
        record,
        &bundle.normalized_plan,
        &new_bundle.normalized_plan,
    )?;
    if new_bundle.plan()? != bundle.plan()? {
        return Err(invalid("original request/coverage/policy/plan differs"));
    }
    let a = replay_project_simulated_candidate(bundle)?;
    let b = replay_project_simulated_candidate(new_bundle)?;
    if a.source_positions != b.source_positions
        || a.allocation_octets != b.allocation_octets
        || a.written_octets != b.written_octets
        || a.preserved_octets != b.preserved_octets
        || a.inferences != b.inferences
    {
        return Err(invalid("derived preparation fields differ"));
    }
    Ok(Comparison {
        profile: bytes,
        input,
    })
}

// Defense in depth AFTER full preparation and the literal-caption classifier. This private
// value comparison grants no source, execution, start or recovery authority on its own.
fn compare_prepared_records(
    original: &ProjectPreparationRecord,
    current: &ProjectPreparationRecord,
    old_plan: &PlanRecord,
    new_plan: &PlanRecord,
) -> io::Result<()> {
    let mut record = current.clone();
    // Full typed record equality after restoring ONLY enumerated provenance fields.
    record.source_sha256 = original.source_sha256.clone();
    record.product_input = original.product_input.clone();
    record.profile_bytes = original.profile_bytes.clone();
    record.profile_sha256 = original.profile_sha256.clone();
    if record != *original
        || new_plan != old_plan
        || serde_json::to_vec(new_plan).map_err(io::Error::other)?
            != serde_json::to_vec(old_plan).map_err(io::Error::other)?
    {
        return Err(invalid("original request/coverage/policy/plan differs"));
    }
    Ok(())
}
