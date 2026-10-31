//! Owns the closed handoff from retained read producers to diagnostic-bundle files.

use super::BundleFile;
use std::collections::BTreeSet;
use std::io::Read;

const MAX_SUPPORT_ENTRIES: usize = 64;
const MAX_RETAINED_ARCHIVE: usize = 512 * 1024;
const FORMAT_ONE_MEMORY_REASON: &str =
    "No independently admitted safe memory profile; source offsets are not runtime addresses.";

/// Owned files admitted from exactly one retained producer result.
///
/// This wrapper deliberately exposes its body only by consuming itself. It does
/// not implement `Serialize` or `Debug`, so callers cannot emit the source object.
pub struct RetainedBundleFiles {
    files: Vec<BundleFile>,
}

impl RetainedBundleFiles {
    /// Moves the admitted files into the one downstream bundle collection.
    pub fn into_files(self) -> Vec<BundleFile> {
        self.files
    }
}

/// A closed, payload-free refusal at the retained-result admission boundary.
pub struct RetainedAdmissionUnavailable {
    code: &'static str,
}

impl RetainedAdmissionUnavailable {
    const fn source() -> Self {
        Self {
            code: "retained-source-unavailable",
        }
    }

    /// Stable code only; no source error, bytes, path or identifier crosses the boundary.
    pub fn code(&self) -> &'static str {
        self.code
    }
}

/// Admits one existing read-diagnostics result after its producer invariants are checked.
///
/// Declared memory (format 2) is admitted as the producer wrote it, including its
/// "keep private" notice: it is not credential-classified, so it may leave only
/// through the bundle's explicit consent (owner decision, ADR-0156).
pub fn admit_retained_diagnostics(
    export: &crate::device_diagnostics::Export,
    expected_build: &str,
) -> Result<RetainedBundleFiles, RetainedAdmissionUnavailable> {
    let memory = export.memory.is_some();
    // Format 2 pins its own memory reason through the writer grammar below.
    if !build_token(expected_build)
        || export.report.build != expected_build
        || (!memory && export.report.memory_reason != FORMAT_ONE_MEMORY_REASON)
        || export.report.items.iter().any(|item| {
            matches!(item.status.as_str(), "partial" | "refused-ambiguous")
                && !item.bytes.is_empty()
        })
    {
        return Err(RetainedAdmissionUnavailable::source());
    }
    let archive = crate::device_diagnostics::bundle_export(export)
        .map_err(|_| RetainedAdmissionUnavailable::source())?;
    let files = if memory {
        fixed_members(
            archive,
            &[
                "report.json",
                "properties.json",
                "memory.json",
                "README.txt",
            ],
            5,
        )?
    } else {
        fixed_members(
            archive,
            &["report.json", "properties.json", "README.txt"],
            4,
        )?
    };
    Ok(RetainedBundleFiles { files })
}

/// Admits one existing universal-support result after its producer invariants are checked.
pub fn admit_retained_support(
    report: &crate::support_report::SupportReport,
    expected_build: &str,
) -> Result<RetainedBundleFiles, RetainedAdmissionUnavailable> {
    validate_support(report, expected_build)?;
    let archive = report
        .bundle()
        .map_err(|_| RetainedAdmissionUnavailable::source())?;
    let files = fixed_members(archive, &["report.json", "README.txt"], 2)?;
    Ok(RetainedBundleFiles { files })
}

fn fixed_members(
    archive: Vec<u8>,
    names: &[&str],
    archive_members: usize,
) -> Result<Vec<BundleFile>, RetainedAdmissionUnavailable> {
    if archive.len() > MAX_RETAINED_ARCHIVE {
        return Err(RetainedAdmissionUnavailable::source());
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive))
        .map_err(|_| RetainedAdmissionUnavailable::source())?;
    if zip.len() != archive_members {
        return Err(RetainedAdmissionUnavailable::source());
    }
    let mut files = Vec::with_capacity(names.len());
    for name in names {
        let mut member = zip
            .by_name(name)
            .map_err(|_| RetainedAdmissionUnavailable::source())?;
        if member.is_dir() || member.size() > 256 * 1024 {
            return Err(RetainedAdmissionUnavailable::source());
        }
        let expected = member.size();
        let mut bytes = Vec::new();
        (&mut member)
            .take(256 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| RetainedAdmissionUnavailable::source())?;
        if bytes.len() as u64 != expected || bytes.len() > 256 * 1024 {
            return Err(RetainedAdmissionUnavailable::source());
        }
        files.push(BundleFile::new(*name, bytes));
    }
    Ok(files)
}

fn validate_support(
    report: &crate::support_report::SupportReport,
    expected_build: &str,
) -> Result<(), RetainedAdmissionUnavailable> {
    if report.kind != "knxbench-support-report"
        || report.format != 1
        || report.written
        || !build_token(expected_build)
        || report.knxbench_version != expected_build
        || !matches!(
            report.support.as_str(),
            "unknown" | "excluded" | "verified" | "untested" | "unsupported"
        )
        || report.context.is_some()
        || report.prior_run.is_some()
        || report.prior_run_scope.is_some()
        || report.facts.len() > MAX_SUPPORT_ENTRIES
        || report.missing_evidence.len() > MAX_SUPPORT_ENTRIES
        || report.withheld.len() > MAX_SUPPORT_ENTRIES
        || !unique_facts(&report.facts)
        || !report.facts.iter().all(fact_is_closed)
        || !report
            .missing_evidence
            .iter()
            .all(|value| allowed_missing(value))
        || !report.withheld.iter().all(|value| allowed_withheld(value))
    {
        return Err(RetainedAdmissionUnavailable::source());
    }
    Ok(())
}

fn unique_facts(facts: &[crate::support_report::Fact]) -> bool {
    let mut seen = BTreeSet::new();
    facts
        .iter()
        .all(|fact| seen.insert((fact.field.as_str(), fact.origin.as_str())))
}

fn fact_is_closed(fact: &crate::support_report::Fact) -> bool {
    let value = fact.value.as_deref();
    let observed = fact.status == "observed" && value.is_some();
    let absent = value.is_none()
        && matches!(
            fact.status.as_str(),
            "unavailable" | "withheld" | "partial-unavailable" | "unavailable-or-refused"
        );
    if !(observed || absent) {
        return false;
    }
    match (fact.field.as_str(), fact.origin.as_str(), value) {
        ("applicationId" | "assertedApplicationId", "user" | "project" | "catalogue", Some(v)) => {
            program_token(v)
        }
        ("applicationId" | "assertedApplicationId", "user" | "project" | "catalogue", None) => true,
        ("firmware", "user", Some(v)) => firmware_version(v),
        ("firmware", "user", None) => true,
        ("functionalCheck", "user", Some(v)) => matches!(
            v,
            "not-checked" | "works-as-configured" | "does-not-work-as-configured"
        ),
        ("mask", "user" | "product" | "live", Some(v)) => hex(v, 4),
        ("mask", "user" | "product" | "live", None) => true,
        ("sourceSha256", "catalogue", Some(v)) => hex(v, 64),
        ("sourceSha256", "catalogue", None) => true,
        ("applicationVersion", "catalogue", Some(v)) => {
            !v.is_empty() && v.len() <= 8 && v.bytes().all(|b| b.is_ascii_digit())
        }
        ("applicationVersion", "catalogue", None) => true,
        ("refusalCategory", "target-policy", Some("target-excluded")) => true,
        ("refusalCategory", "product-default-planner" | "offline-planner", Some(v)) => {
            unsupported_category(v)
        }
        ("diagnosticProfile", "live", Some(v)) => matches!(
            v,
            "system2-status-v1" | "programming-mode-v1" | "descriptor-only-v1"
        ),
        ("manufacturer", "live", Some(v)) => hex(v, 4),
        ("hardwareType", "live", Some(v)) => hex(v, 12),
        ("addressTableState" | "associationTableState" | "applicationState", "live", Some(v)) => {
            hex(v, 2) && u8::from_str_radix(v, 16).is_ok_and(|state| state <= 5)
        }
        ("programmingMode", "live", Some("true" | "false")) => true,
        (
            "manufacturer"
            | "hardwareType"
            | "addressTableState"
            | "associationTableState"
            | "applicationState"
            | "programmingMode",
            "live",
            None,
        ) => true,
        ("liveDiagnostics", "live", None) => fact.status == "partial-unavailable",
        _ => false,
    }
}

fn build_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_+".contains(&b))
}

fn firmware_version(value: &str) -> bool {
    let mut count = 0;
    let valid = value.split('.').all(|part| {
        count += 1;
        !part.is_empty() && part.len() <= 5 && part.bytes().all(|byte| byte.is_ascii_digit())
    });
    valid && (1..=4).contains(&count)
}

fn program_token(value: &str) -> bool {
    let Some((manufacturer, application)) = value.split_once('_') else {
        return false;
    };
    let Some(manufacturer) = manufacturer.strip_prefix("M-") else {
        return false;
    };
    let Some(application) = application.strip_prefix("A-") else {
        return false;
    };
    let parts = application.split('-').collect::<Vec<_>>();
    hex(manufacturer, 4)
        && matches!(parts.len(), 3 | 4)
        && hex(parts[0], 4)
        && hex(parts[1], 2)
        && hex(parts[2], 4)
        && (parts.len() == 3
            || parts[3]
                .strip_prefix('O')
                .is_some_and(|suffix| hex(suffix, 4)))
}

fn hex(value: &str, digits: usize) -> bool {
    value.len() == digits && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn unsupported_category(value: &str) -> bool {
    matches!(
        value,
        "not-memory-mapped"
            | "procedure-style"
            | "unmodelled-step"
            | "procedure-shape"
            | "procedure-contents"
            | "parameter-evaluation"
            | "modules"
            | "parameter-value"
            | "image-structure"
            | "byte-order"
            | "product-data"
            | "configuration"
            | "vendor-download-plugin"
    )
}

fn allowed_missing(value: &str) -> bool {
    matches!(
        value,
        "hardware-verification"
            | "recovery-coverage"
            | "user-functional-check"
            | "application-identity-conflict"
            | "product-identity"
            | "firmware"
            | "product-source-over-budget-or-unavailable"
            | "live-diagnostics-not-collected"
            | "readable-product-code"
            | "plan-assessment-unavailable-or-over-budget"
            | "matching-product-package"
            | "catalogue-readable"
            | "reviewed-family-diagnostics"
    )
}

fn allowed_withheld(value: &str) -> bool {
    matches!(
        value,
        "installation-identifiers"
            | "credentials"
            | "configuration-values"
            | "raw-memory-and-binaries"
            | "unknown-source-content"
            | "unrecognised-application-id-withheld"
            | "context-rights-not-confirmed"
    )
}
