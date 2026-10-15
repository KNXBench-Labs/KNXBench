//! Privacy-allowlisted support evidence, independent of executable download admission.
use std::collections::BTreeMap;
use std::io::{Cursor, Write};

use knx_core::{DeviceId, Project};
use knx_productdb::code::{load_program_code, LoadStep, ProgramCode};
use knx_productdb::query::{program_identity, resolve_program};
use knx_productdb::Connection;
use serde::{Deserialize, Serialize};

const MAX_REPORT_SOURCE_BYTES: u32 = 8 * 1024 * 1024;
const MAX_PARAMETER_TEXT_OCTETS: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FunctionalCheck {
    NotChecked,
    WorksAsConfigured,
    DoesNotWorkAsConfigured,
}

/// Manual inputs are untrusted; their origin never becomes a live observation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportInput {
    pub program_id: Option<String>,
    pub firmware: Option<String>,
    pub mask: Option<u16>,
    pub functional_check: Option<FunctionalCheck>,
    #[serde(default)]
    pub include_context: bool,
    #[serde(default)]
    pub context_rights: bool,
}

/// A fact's value is selected by its producer, not arbitrary exported error text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fact {
    pub field: String,
    pub origin: String,
    pub status: String,
    pub value: Option<String>,
}

/// A source projection excludes bytes, unknown attributes and free-text metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    pub segments: Vec<(u32, u32)>,
    pub steps: Vec<String>,
    pub omitted_steps: usize,
    pub truncated: bool,
}

/// The universal report is evidence for investigation, never an unlock token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportReport {
    pub kind: &'static str,
    pub format: u32,
    pub knxbench_version: String,
    pub support: String,
    /// Report generation itself writes nothing, irrespective of a prior run.
    pub written: bool,
    pub facts: Vec<Fact>,
    pub context: Option<Context>,
    pub missing_evidence: Vec<String>,
    pub withheld: Vec<String>,
    pub prior_run: Option<crate::download_report::ReportedOutcome>,
    pub prior_run_scope: Option<&'static str>,
}

impl SupportReport {
    pub fn fact(&mut self, field: &str, origin: &str, value: Option<String>, absent: &str) {
        self.facts.push(Fact {
            field: field.into(),
            origin: origin.into(),
            status: if value.is_some() { "observed" } else { absent }.into(),
            value,
        });
    }

    /// No parameter values, free-text warnings or installation metadata cross over.
    pub fn prior_run(&mut self, report: &crate::download_report::DownloadReport) {
        self.prior_run = report.outcome;
        self.prior_run_scope = self.prior_run.map(|_| report.scope);
        let functional_check = self.facts.iter().any(|f| {
            f.field == "functionalCheck"
                && matches!(
                    f.value.as_deref(),
                    Some("works-as-configured" | "does-not-work-as-configured")
                )
        });
        if self.prior_run.is_some() && !functional_check {
            self.missing_evidence.push("user-functional-check".into());
        }
    }

    pub fn summary(&self) -> String {
        let mut s = format!("KNXBench universal support report (format {})\nBuild: {}\nSupport: {}\nThis report does not enable a download or certify a device.\n", self.format, self.knxbench_version, self.support);
        for f in &self.facts {
            s.push_str(&format!(
                "{} [{}; {}]: {}\n",
                f.field,
                f.origin,
                f.status,
                f.value.as_deref().unwrap_or("unknown")
            ));
        }
        s.push_str(&format!(
            "Missing evidence: {}\nWithheld: {}\n",
            self.missing_evidence.join(", "),
            self.withheld.join(", ")
        ));
        if let Some(run) = self.prior_run {
            s.push_str(&format!(
                "Prior run ({}): {}\nTransport completion is not a functional test.\n",
                self.prior_run_scope.unwrap_or("unknown"),
                run.line()
            ));
        }
        s
    }

    /// Fixed archive paths, no originals or arbitrary filenames; caller enforces consent.
    pub fn bundle(&self) -> Result<Vec<u8>, String> {
        let json = serde_json::to_vec_pretty(self).map_err(|_| "report-encoding")?;
        if json.len() > 512 * 1024 {
            return Err("report-size-limit".into());
        }
        let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in [
            ("report.json", json),
            ("README.txt", self.summary().into_bytes()),
        ] {
            out.start_file(name, zip::write::SimpleFileOptions::default())
                .map_err(|_| "bundle-encoding")?;
            out.write_all(&bytes).map_err(|_| "bundle-encoding")?;
        }
        Ok(out.finish().map_err(|_| "bundle-encoding")?.into_inner())
    }
}

fn program_token(s: &str) -> bool {
    // Privacy allowlist for observed ApplicationProgram IDs, not a full format validator.
    // The optional O-hash suffix is witnessed in retained product/download evidence.
    let parts: Vec<_> = s.split('_').collect();
    parts.len() == 2
        && parts[0].strip_prefix("M-").is_some_and(|v| hex(v, 4))
        && parts[1].strip_prefix("A-").is_some_and(|v| {
            let p: Vec<_> = v.split('-').collect();
            (p.len() == 3 || (p.len() == 4 && p[3].strip_prefix('O').is_some_and(|v| hex(v, 4))))
                && hex(p[0], 4)
                && hex(p[1], 2)
                && hex(p[2], 4)
        })
}
fn hex(s: &str, n: usize) -> bool {
    s.len() == n && s.bytes().all(|b| b.is_ascii_hexdigit())
}
fn manual_token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 48
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_+".contains(&b))
}

/// All inputs are optional. A missing catalogue/program/plan is a reportable state.
pub fn build_report(
    conn: Option<&Connection>,
    project: Option<&Project>,
    device: Option<DeviceId>,
    input: &ReportInput,
    build: &str,
) -> SupportReport {
    let mut out = SupportReport {
        kind: "knxbench-support-report",
        format: 1,
        knxbench_version: build.into(),
        support: "unknown".into(),
        written: false,
        facts: vec![],
        context: None,
        missing_evidence: vec!["hardware-verification".into(), "recovery-coverage".into()],
        withheld: vec![
            "installation-identifiers".into(),
            "credentials".into(),
            "configuration-values".into(),
            "raw-memory-and-binaries".into(),
            "unknown-source-content".into(),
        ],
        prior_run: None,
        prior_run_scope: None,
    };
    let selected = project.and_then(|p| p.devices.iter().find(|d| Some(d.id) == device));
    if selected
        .and_then(|d| d.address)
        .is_some_and(|a| knx_core::ContactableAddress::new(a).is_err())
    {
        out.support = "excluded".into();
        out.fact(
            "refusalCategory",
            "target-policy",
            Some("target-excluded".into()),
            "unavailable",
        );
    }
    let resolved =
        selected.and_then(|d| conn.and_then(|c| resolve_program(c, &d.program_ref).ok().flatten()));
    let requested = resolved
        .as_deref()
        .or_else(|| {
            selected
                .map(|d| d.program_ref.as_str())
                .filter(|s| program_token(s))
        })
        .or(input.program_id.as_deref());
    let id = requested.filter(|s| program_token(s));
    out.fact(
        "applicationId",
        if resolved.is_some() {
            "catalogue"
        } else if selected.is_some_and(|d| Some(d.program_ref.as_str()) == id) {
            "project"
        } else {
            "user"
        },
        id.map(str::to_string),
        "unavailable",
    );
    if let Some(asserted) = input
        .program_id
        .as_deref()
        .filter(|user| Some(*user) != requested)
    {
        let allowed = program_token(asserted);
        out.fact(
            "assertedApplicationId",
            "user",
            allowed.then(|| asserted.to_string()),
            "withheld",
        );
        if allowed {
            out.missing_evidence
                .push("application-identity-conflict".into());
        } else {
            out.withheld
                .push("unrecognised-application-id-withheld".into());
        }
    }
    if requested.is_some() && id.is_none() {
        out.withheld
            .push("unrecognised-application-id-withheld".into());
    }
    if id.is_none() {
        out.missing_evidence.push("product-identity".into());
    }
    out.fact(
        "firmware",
        "user",
        input
            .firmware
            .as_deref()
            .filter(|s| manual_token(s))
            .map(str::to_string),
        "unavailable",
    );
    if let Some(check) = &input.functional_check {
        out.fact(
            "functionalCheck",
            "user",
            Some(
                match check {
                    FunctionalCheck::NotChecked => "not-checked",
                    FunctionalCheck::WorksAsConfigured => "works-as-configured",
                    FunctionalCheck::DoesNotWorkAsConfigured => "does-not-work-as-configured",
                }
                .into(),
            ),
            "unavailable",
        );
    }
    if input.firmware.as_deref().is_none_or(|s| !manual_token(s)) {
        out.missing_evidence.push("firmware".into());
    }
    out.fact(
        "mask",
        "user",
        input.mask.map(|m| format!("{m:04X}")),
        "unavailable",
    );
    if let (Some(c), Some(id)) = (conn, id) {
        match program_identity(c, id) {
            Ok(Some(identity)) => {
                let bounded = knx_productdb::blob::source_file_length(c, &identity.source_sha256)
                    .ok()
                    .flatten()
                    .is_some_and(|size| size <= MAX_REPORT_SOURCE_BYTES);
                out.fact(
                    "sourceSha256",
                    "catalogue",
                    hex(&identity.source_sha256, 64).then_some(identity.source_sha256),
                    "withheld",
                );
                out.fact(
                    "applicationVersion",
                    "catalogue",
                    identity
                        .application_version
                        .filter(|s| s.len() <= 8 && s.bytes().all(|b| b.is_ascii_digit())),
                    "withheld",
                );
                if !bounded {
                    out.missing_evidence.extend([
                        "product-source-over-budget-or-unavailable".into(),
                        "live-diagnostics-not-collected".into(),
                    ]);
                    return out;
                }
                let mut assessment_allowed = false;
                if let Ok(Some(code)) = load_program_code(c, id) {
                    assessment_allowed = knx_productdb::image::text_parameter_buffers_fit(
                        c,
                        id,
                        &code,
                        MAX_PARAMETER_TEXT_OCTETS,
                    )
                    .unwrap_or(false);
                    out.fact(
                        "mask",
                        "product",
                        code.mask_version
                            .as_deref()
                            .and_then(|s| s.strip_prefix("MV-"))
                            .filter(|s| hex(s, 4))
                            .map(str::to_string),
                        "unavailable",
                    );
                    if input.include_context && input.context_rights {
                        out.context = Some(context(&code));
                    } else if input.include_context {
                        out.withheld.push("context-rights-not-confirmed".into());
                    }
                } else {
                    out.missing_evidence.push("readable-product-code".into());
                }
                if !assessment_allowed {
                    out.missing_evidence.extend([
                        "plan-assessment-unavailable-or-over-budget".into(),
                        "live-diagnostics-not-collected".into(),
                    ]);
                    return out;
                }
                if let (None, Ok(evidence)) =
                    (selected, crate::download_support::shipped_evidence())
                {
                    let level = crate::download_support::support_level(c, id, &evidence);
                    out.support = level.code().into();
                    if let crate::download_support::SupportLevel::Unsupported { category, .. } =
                        level
                    {
                        out.fact(
                            "refusalCategory",
                            "product-default-planner",
                            Some(category.to_string()),
                            "unavailable",
                        );
                    }
                }
                if let Some((p, d)) = project.zip(selected) {
                    if let Some(address) = d.address {
                        if knx_core::is_project_excluded(address) {
                            out.support = "excluded".into();
                        } else {
                            match crate::device_download::prepare_device_download(c, p, address) {
                                Ok(prepared) => {
                                    if let Ok(e) = crate::download_support::shipped_evidence() {
                                        out.support = crate::download_support::download_level(
                                            &prepared.request.program_id,
                                            None,
                                            &prepared.plan,
                                            &prepared.inferences,
                                            &e,
                                        )
                                        .code()
                                        .into();
                                    }
                                }
                                Err(e) => {
                                    out.support = "unsupported".into();
                                    out.fact(
                                        "refusalCategory",
                                        "offline-planner",
                                        Some(
                                            crate::project_readiness::prepare_category(&e)
                                                .to_string(),
                                        ),
                                        "unavailable",
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Ok(None) => out.missing_evidence.push("matching-product-package".into()),
            Err(_) => out.missing_evidence.push("catalogue-readable".into()),
        }
    } else {
        out.missing_evidence.push("matching-product-package".into());
    }
    out.missing_evidence
        .push("live-diagnostics-not-collected".into());
    out
}

fn context(code: &ProgramCode) -> Context {
    let mut omitted = 0;
    let mut steps = Vec::new();
    let mut count = 0;
    for p in &code.load_procedures {
        for s in &p.steps {
            count += 1;
            if steps.len() >= 256 {
                continue;
            }
            let mut a = BTreeMap::<&str, u32>::new();
            let kind = match s {
                LoadStep::Connect => "connect",
                LoadStep::Disconnect => "disconnect",
                LoadStep::Restart => "restart",
                LoadStep::Load { lsm } => {
                    a.insert("machine", (*lsm).into());
                    "load"
                }
                LoadStep::Unload { lsm } => {
                    a.insert("machine", (*lsm).into());
                    "unload"
                }
                LoadStep::LoadCompleted { lsm } => {
                    a.insert("machine", (*lsm).into());
                    "load-completed"
                }
                LoadStep::AbsSegment {
                    lsm,
                    segment_type,
                    address,
                    size,
                    access,
                    memory_type,
                    flags,
                } => {
                    for (k, v) in [
                        ("machine", *lsm as u32),
                        ("subtype", *segment_type as u32),
                        ("memoryAddress", *address as u32),
                        ("size", *size as u32),
                        ("access", *access as u32),
                        ("memoryType", *memory_type as u32),
                        ("flags", *flags as u32),
                    ] {
                        a.insert(k, v);
                    }
                    "absolute-segment"
                }
                LoadStep::TaskSegment { lsm, address } => {
                    a.insert("machine", (*lsm).into());
                    a.insert("memoryAddress", (*address).into());
                    "task-segment"
                }
                LoadStep::TaskCtrl1 {
                    lsm,
                    address,
                    count,
                } => {
                    a.insert("machine", (*lsm).into());
                    a.insert("memoryAddress", (*address).into());
                    a.insert("count", (*count).into());
                    "task-control-1"
                }
                LoadStep::TaskCtrl2 {
                    lsm,
                    callback,
                    address,
                    segment_0,
                    segment_1,
                } => {
                    for (k, v) in [
                        ("machine", *lsm as u32),
                        ("callback", *callback as u32),
                        ("memoryAddress", *address as u32),
                        ("segment0", *segment_0 as u32),
                        ("segment1", *segment_1 as u32),
                    ] {
                        a.insert(k, v);
                    }
                    "task-control-2"
                }
                LoadStep::CompareProp {
                    object_index,
                    property_id,
                    data,
                } => {
                    a.insert("objectIndex", (*object_index).into());
                    a.insert("propertyId", (*property_id).into());
                    a.insert("comparisonOctets", data.len() as u32);
                    "compare-property-values-withheld"
                }
                LoadStep::Unmodelled { .. } => {
                    omitted += 1;
                    "unsupported-source-step-withheld"
                }
            };
            steps.push(format!(
                "{kind} {}",
                serde_json::to_string(&a).expect("numeric map")
            ));
        }
    }
    Context {
        segments: code
            .segments
            .iter()
            .take(128)
            .map(|s| (s.address, s.size))
            .collect(),
        truncated: count > 256 || code.segments.len() > 128,
        steps,
        omitted_steps: omitted,
    }
}
