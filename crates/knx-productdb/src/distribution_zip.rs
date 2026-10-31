//! Inspects checked ZIP envelopes with collision-checked logical paths and immutable raw names.

use super::provenance::append_counted;
use super::*;
use crate::operation::Cancellation;
use crate::package::{open_distribution_archive, read_checked_member, PackageError, PackageLimits};

fn archive_error(e: PackageError) -> DistributionInspectionError {
    match e {
        PackageError::SizeLimit { path } => limit(&format!("archive: {path}")),
        other => DistributionInspectionError::Archive(other.to_string()),
    }
}

pub(super) fn inspect(
    bytes: &[u8],
    limits: &DistributionLimits,
    report: &mut DistributionInspection,
) -> Result<()> {
    let mut checked = open_distribution_archive(
        bytes,
        PackageLimits {
            max_member_size: limits.max_member_bytes,
            max_expanded_size: limits.max_expanded_bytes,
        },
        Cancellation::Quiet,
    )
    .map_err(archive_error)?;
    report.source_kind = DistributionSourceKind::DistributionZip;
    if checked.has_master {
        report.diagnostics.push(DistributionDiagnostic { code:"masterMemberPresentNotInspected".into(),source:None,detail:"A member is named knx_master.xml; master data and authenticity are not verified or borrowed".into() });
        report
            .limitations
            .retain(|l| *l != DistributionLimitation::MissingMaster);
        report
            .limitations
            .push(DistributionLimitation::MasterNotInspected);
    }
    let mut recognized = false;
    let mut records = super::readers::output_count(report);
    // Exact running size: re-serializing the whole report per member would be quadratic.
    let cap = limits.max_report_bytes;
    let mut bytes = super::provenance::json_size(report, cap)?;
    for index in 0..checked.archive.len() {
        let file = checked
            .archive
            .by_index_raw(index)
            .map_err(|e| DistributionInspectionError::Archive(e.to_string()))?;
        let decoded_name = file.name().to_string();
        let raw_name = file.name_raw().to_vec();
        let size = file.size();
        let compressed_size = file.compressed_size();
        let logical_path = decoded_name.replace('\\', "/");
        let directory = logical_path.ends_with('/');
        drop(file);
        let (_, data) = read_checked_member(
            &mut checked.archive,
            index,
            limits.max_member_bytes,
            Cancellation::Quiet,
        )
        .map_err(archive_error)?;
        let data = data.unwrap_or_default();
        if directory && !data.is_empty() {
            return Err(DistributionInspectionError::Archive(
                "directory member carries data".into(),
            ));
        }
        let mut member = DistributionMember {
            ordinal: index as u32,
            raw_name,
            decoded_name,
            logical_path,
            compressed_size,
            size,
            sha256: crate::sha256_hex(&data),
            state: DistributionPassState::Preserved,
            error: None,
        };
        let sniff = data.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&data);
        let xml_candidate = !directory
            && (member.logical_path.to_ascii_lowercase().ends_with(".xml")
                || sniff.iter().copied().find(|b| !b.is_ascii_whitespace()) == Some(b'<')
                || data.starts_with(&[0xff, 0xfe])
                || data.starts_with(&[0xfe, 0xff]));
        if xml_candidate {
            // A failed optional member never leaves a partly appended logical report.
            let mut part = DistributionInspection::empty(
                &member.logical_path,
                &data,
                DistributionSourceKind::ManufacturerXml,
            );
            match super::xml::inspect(&data, Some(index as u32), limits, &mut part) {
                Ok(_) => {
                    records = records
                        .checked_add(super::readers::output_count(&part))
                        .ok_or_else(|| limit("output records"))?;
                    recognized = true;
                    member.state = DistributionPassState::Parsed;
                    let b = &mut bytes;
                    append_counted(&mut report.sections, part.sections, b, cap)?;
                    append_counted(&mut report.documents, part.documents, b, cap)?;
                    append_counted(&mut report.catalog_items, part.catalog_items, b, cap)?;
                    append_counted(&mut report.hardware, part.hardware, b, cap)?;
                    append_counted(&mut report.applications, part.applications, b, cap)?;
                    append_counted(&mut report.reader_passes, part.reader_passes, b, cap)?;
                    append_counted(&mut report.unknowns, part.unknowns, b, cap)?;
                    append_counted(&mut report.references, part.references, b, cap)?;
                    append_counted(&mut report.identities, part.identities, b, cap)?;
                    append_counted(&mut report.diagnostics, part.diagnostics, b, cap)?;
                    append_counted(
                        &mut report.declared_metadata,
                        part.declared_metadata,
                        b,
                        cap,
                    )?;
                    append_counted(
                        &mut report.preserved_content,
                        part.preserved_content,
                        b,
                        cap,
                    )?;
                    append_counted(
                        &mut report.dependency_evidence,
                        part.dependency_evidence,
                        b,
                        cap,
                    )?;
                }
                Err(e @ DistributionInspectionError::ResourceLimit(_)) => return Err(e),
                Err(e) => {
                    member.error = Some(e);
                }
            }
        }
        append_counted(&mut report.members, vec![member], &mut bytes, cap)?;
        records += 1;
        if records > limits.max_output_records {
            return Err(limit("output records"));
        }
    }
    if !recognized {
        return Err(DistributionInspectionError::UnrecognizedSource);
    }
    super::links::resolve_archive(report, limits)?;
    Ok(())
}
