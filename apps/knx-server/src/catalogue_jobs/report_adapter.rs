//! Complete borrowed-row extraction from the real controlled ProductDB reports.
// Macro field identifiers are also its inferred generic type identifiers.
#![allow(non_camel_case_types)]
use super::{dto::*, persistence::JobWorkspace, records::ArtifactWriter, strict::U64};
use knx_productdb::{managed_apply::ControlledReport, ProductFileInstall};
use serde::Serialize;

// Each record borrows its original strings. The writer counts/admits its bounded
// serialization before materialization; no cloned whole report or first-N view.
macro_rules! row {
    ($writer:ident, $ordinal:ident, $kind:literal, {$($field:ident: $value:expr),* $(,)?}) => {{
        #[derive(Serialize)]
        #[allow(non_camel_case_types)]
        #[serde(rename_all = "camelCase")]
        struct Row<$($field: Serialize),*> { ordinal: U64, kind: &'static str, $($field: $field),* }
        $writer.append(&Row { ordinal: U64($ordinal), kind: $kind, $($field: $value),* })?;
        $ordinal = $ordinal.checked_add(1).ok_or(FailureKind::CounterExhausted)?;
    }};
}
fn n(n: usize) -> U64 {
    U64(n as u64)
}
fn add(a: u64, b: u64) -> Result<u64, FailureKind> {
    a.checked_add(b).ok_or(FailureKind::ReportMismatch)
}

pub(super) fn summary(
    install: &ProductFileInstall,
    report: &ControlledReport,
    artifact: Artifact,
    consent_id: super::JobId,
) -> Result<Preview, FailureKind> {
    let conflicts = install
        .packages
        .iter()
        .try_fold(0, |a, p| add(a, p.report.conflicts.len() as u64))?;
    Ok(Preview {
        consent_id,
        artifact,
        package_already_installed: install.packages.iter().all(|p| p.report.skipped),
        wrapper: install.wrapper.is_some(),
        packages: n(install.packages.len()),
        programs: n(report.programs.len()),
        products_added: U64(report.products_added),
        hardware2_programs_added: U64(report.hardware2programs_added),
        id_conflicts: U64(conflicts),
        growth_cliff_possible: true,
    })
}

pub(super) fn write(
    workspace: &mut JobWorkspace,
    kind: ArtifactKind,
    install: &ProductFileInstall,
    report: &ControlledReport,
    bytes: u64,
) -> Result<Artifact, FailureKind> {
    // Validate parent identities and all measured cross-row counts before the
    // manifest can certify completeness. Unavailable history is explicit.
    if install.packages.is_empty()
        || (install.wrapper.is_none()
            && (install.packages.len() != 1 || install.packages[0].path.is_some()))
        || install
            .packages
            .iter()
            .any(|p| p.path.is_some() != install.wrapper.is_some())
        || report
            .programs
            .iter()
            .any(|p| p.package_key >= install.packages.len())
    {
        return Err(FailureKind::ReportMismatch);
    }
    for p in &install.packages {
        let r = &p.report;
        if let Some(f) = &r.facts {
            let occurrences = f
                .unknown_constructs
                .iter()
                .try_fold(0, |a, u| add(a, u64::from(u.occurrences)))?;
            if f.unknown_occurrences != occurrences {
                return Err(FailureKind::ReportMismatch);
            }
            for c in &f.counts {
                if c.category.as_str() == "unknown_construct"
                    && ((c.disposition.as_str() == "read" && c.count != occurrences)
                        || (c.disposition.as_str() == "stored"
                            && c.count != f.unknown_constructs.len() as u64))
                {
                    return Err(FailureKind::ReportMismatch);
                }
            }
        }
        if let Some(b) = &r.baggage {
            for payload in &b.payloads {
                let links = b
                    .declarations
                    .iter()
                    .filter(|d| {
                        d.resolution.as_str() == "resolved"
                            && d.member_path.as_ref() == Some(&payload.member_path)
                    })
                    .count();
                if payload.declarations != links as u64
                    || !r.members.iter().any(|m| {
                        m.path == payload.member_path
                            && m.sha256 == payload.sha256
                            && m.size == payload.size
                    })
                {
                    return Err(FailureKind::ReportMismatch);
                }
            }
            if b.declarations.iter().any(|d| {
                d.resolution.as_str() == "resolved"
                    && !b
                        .payloads
                        .iter()
                        .any(|p| Some(&p.member_path) == d.member_path.as_ref())
            }) {
                return Err(FailureKind::ReportMismatch);
            }
        }
    }
    let mut expected = 1u64;
    for p in &report.programs {
        expected = add(expected, add(1, p.program.installed_versions.len() as u64)?)?;
    }
    if let Some(w) = &install.wrapper {
        expected = add(
            expected,
            add(install.packages.len() as u64, w.other_members.len() as u64)?,
        )?;
    }
    for p in &install.packages {
        let r = &p.report;
        expected = add(expected, 2)?; // package report + baggage summary
        for size in [r.members.len(), r.conflicts.len(), r.source_names.len()] {
            expected = add(expected, size as u64)?;
        }
        if let Some(f) = &r.facts {
            for size in [
                f.counts.len(),
                f.unknown_constructs.len(),
                f.diagnostics.len(),
            ] {
                expected = add(expected, size as u64)?;
            }
        } else {
            expected = add(expected, 1)?;
        }
        if let Some(b) = &r.baggage {
            expected = add(
                expected,
                add(b.declarations.len() as u64, b.payloads.len() as u64)?,
            )?;
        } else {
            expected = add(expected, 1)?;
        }
    }
    let mut writer = ArtifactWriter::new(workspace, kind)?;
    let mut ordinal = 0u64;
    let conflicts = install
        .packages
        .iter()
        .try_fold(0, |a, p| add(a, p.report.conflicts.len() as u64))?;
    match kind {
        ArtifactKind::Preview => {
            row!(writer, ordinal, "previewSummary", {package_already_installed: install.packages.iter().all(|p|p.report.skipped), wrapper: install.wrapper.is_some(), packages: n(install.packages.len()), programs: n(report.programs.len()), products_added: U64(report.products_added), hardware2_programs_added: U64(report.hardware2programs_added), id_conflicts: U64(conflicts), growth_cliff_possible: true})
        }
        ArtifactKind::Result => {
            row!(writer, ordinal, "resultSummary", {packages: n(install.packages.len()), catalogue_page_bytes: U64(bytes), next_job_input_limit_exceeded: bytes > knx_productdb::managed_apply::CATALOGUE_INPUT_BYTE_LIMIT})
        }
    }
    for (key, p) in report.programs.iter().enumerate() {
        let r = &p.program;
        row!(writer, ordinal, "program", {program_key: n(key), package_key: n(p.package_key), id: &r.id, manufacturer_id: &r.manufacturer_id, name: &r.name, application_number: &r.application_number, application_version: &r.application_version, status: r.status});
        for v in &r.installed_versions {
            row!(writer, ordinal, "installedVersion", {program_key: n(key), id: &v.id, application_version: &v.application_version});
        }
    }
    if let Some(wrapper) = &install.wrapper {
        for (key, p) in install.packages.iter().enumerate() {
            row!(writer, ordinal, "wrapperPackage", {package_key: n(key), path: &p.path, sha256: &p.report.sha256, already_installed: p.report.skipped});
        }
        for m in &wrapper.other_members {
            row!(writer, ordinal, "wrapperOtherMember", {path: &m.path, size: U64(m.size), sha256: &m.sha256, member_kind: m.kind});
        }
    }
    let mut source_key = 0u64;
    for (key, p) in install.packages.iter().enumerate() {
        let r = &p.report;
        #[derive(Serialize)]
        struct Translations {
            program: U64,
            catalog: U64,
            hardware: U64,
            master: U64,
        }
        row!(writer, ordinal, "packageReport", {package_key: n(key), path: &p.path, sha256: &r.sha256, scheme: r.scheme, skipped: r.skipped, unknown: n(r.unknown), conflicts: n(r.conflicts.len()), translations: Translations {program: n(r.translations.program), catalog: n(r.translations.catalog), hardware:n(r.translations.hardware), master:n(r.translations.master)}, dropped_datapoint_types: n(r.dropped_datapoint_types), facts: if r.facts.is_some() {"available"} else {"unavailable"}, unknown_occurrences: r.facts.as_ref().map(|f|U64(f.unknown_occurrences)), baggage: if r.baggage.is_some() {"available"} else {"unavailable"}});
        for m in &r.members {
            row!(writer, ordinal, "member", {package_key: n(key), path: &m.path, role: &m.role, sha256: &m.sha256, size: U64(m.size)});
        }
        if let Some(f) = &r.facts {
            for c in &f.counts {
                row!(writer, ordinal, "installCount", {package_key:n(key), category:c.category.as_str(), disposition:c.disposition.as_str(), count:U64(c.count)});
            }
            for u in &f.unknown_constructs {
                row!(writer, ordinal, "unknownConstruct", {package_key:n(key), xpath:&u.xpath, unknown_kind:u.kind.as_str(), name:&u.name, occurrences:U64(u64::from(u.occurrences)), sample:&u.sample});
            }
            for d in &f.diagnostics {
                // Preserve the generated terminal master-ordinal selector too:
                // it is declaration identity, not detail to redact. ArtifactWriter
                // admits only the bounded typed selector, never arbitrary XPath.
                row!(writer, ordinal, "diagnostic", {package_key:n(key), diagnostic_kind:d.kind().as_str(), archive_path:d.archive_path(), xml_path:d.xml_path(), detail:d.detail(), occurrences:U64(d.occurrences())});
            }
        } else {
            row!(writer, ordinal, "historyUnavailable", {package_key:n(key), evidence_kind:"installFacts", reason:"notRecorded"});
        }
        for c in &r.conflicts {
            row!(writer, ordinal, "idConflict", {package_key:n(key), table:&c.table, id:&c.id, kept_sha256:&c.kept_sha256, other_sha256:&c.other_sha256, occurrence:U64(u64::from(c.occurrence))});
        }
        for alias in &r.source_names {
            row!(writer, ordinal, "provenance", {package_key:n(key), source_key:U64(source_key), sha256:&r.sha256, alias:alias});
            source_key = add(source_key, 1)?;
        }
        row!(writer, ordinal, "baggageSummary", {package_key:n(key), status:if r.baggage.is_some(){"available"}else{"unavailable"}, declarations:r.baggage.as_ref().map(|b|n(b.declarations.len())), payloads:r.baggage.as_ref().map(|b|n(b.payloads.len()))});
        if let Some(b) = &r.baggage {
            for d in &b.declarations {
                let v = &d.declaration;
                row!(writer, ordinal, "baggageDeclaration", {package_key:n(key), index_path:&d.index_path, declaration_ordinal:U64(d.ordinal), declaration_id:&v.id, name:&v.name, target_path:&v.target_path, install_on_import:&v.install_on_import, time_info:&v.time_info, file_version:&v.file_version, detail:&d.detail, resolution:d.resolution.as_str(), member_path:&d.member_path});
            }
            for p in &b.payloads {
                use knx_productdb::baggage::NestedArchive;
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                struct Nested {
                    state: &'static str,
                    entries: Option<U64>,
                    declared_expanded_size: Option<U64>,
                    encrypted_entries: Option<U64>,
                    archive_named_entries: Option<U64>,
                }
                let (state, counts) = match p.nested {
                    NestedArchive::NotArchive => ("notArchive", None),
                    NestedArchive::Unreadable => ("unreadable", None),
                    NestedArchive::Read {
                        entries,
                        declared_expanded_size,
                        encrypted_entries,
                        archive_named_entries,
                    } => (
                        "read",
                        Some([
                            entries,
                            declared_expanded_size,
                            encrypted_entries,
                            archive_named_entries,
                        ]),
                    ),
                };
                let nested = Nested {
                    state,
                    entries: counts.map(|v| U64(v[0])),
                    declared_expanded_size: counts.map(|v| U64(v[1])),
                    encrypted_entries: counts.map(|v| U64(v[2])),
                    archive_named_entries: counts.map(|v| U64(v[3])),
                };
                row!(writer, ordinal, "baggagePayload", {package_key:n(key), member_path:&p.member_path, sha256:&p.sha256, size:U64(p.size), media_class:p.media_class.as_str(), declarations:U64(p.declarations), nested:nested});
            }
        } else {
            row!(writer, ordinal, "historyUnavailable", {package_key:n(key), evidence_kind:"baggage", reason:"unavailable"});
        }
    }
    if ordinal != expected {
        return Err(FailureKind::ReportMismatch);
    }
    let artifact = writer.finish()?;
    if artifact.records != U64(ordinal) {
        return Err(FailureKind::ReportMismatch);
    }
    Ok(artifact)
}
