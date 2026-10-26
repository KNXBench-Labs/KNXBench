//! A tester's download report: what one download wrote and how it ended.
//!
//! ADR-0121. A report names the program, scope and plan a download ran, the
//! parameter values it wrote and its witnessed outcome, so that a maintainer
//! can review it and, together with the tester's check of the device, record
//! the program as verified (`data/verified_downloads.json`). A report is
//! evidence for that review, never a verification by itself.
//!
//! It carries no project name, device name, individual address, gateway,
//! key, backup path or free-text error: those identify an installation, or
//! may. A failed run's cause travels as the closed, payload-free
//! [`DownloadFailure`] (DIAG-02) instead. Parameter values are included
//! (owner decision 2026-10-13) because they answer byte-order questions; texts among them may be personal, which
//! the guidance in [`DownloadReport::summary`] says.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use knx_productdb::query::program_identity;
use knx_productdb::{Connection, ProductDbError};
use serde::Serialize;

use crate::commissioning_activity::{Restart, Written};
use crate::device_download::PreparedDownload;
use crate::download_failure::DownloadFailure;
use crate::download_support::{scope_code, SupportLevel};
use crate::post_download_check::{IdentityCheck, PostDownloadCheck};

/// What a report file says it is.
pub const REPORT_KIND: &str = "knxbench-download-report";
/// The report format; raised when a field changes meaning. Added, optional
/// fields change no existing meaning: a reader of format 1 that lacks
/// `postDownloadCheck` reads the run as not checked, and one that lacks
/// `failure` (reports before DIAG-02) reads a stopped run's cause as not
/// recorded, never as a success.
pub const REPORT_FORMAT: u32 = 1;
/// Where reports may be mailed instead of posted publicly.
pub const REPORT_MAILBOX: &str = "issue@knxbench.com";
/// The repository whose issue form takes reports.
pub const REPOSITORY: &str = "https://github.com/KNXBench-Labs/KNXBench";
/// The issue form, `.github/ISSUE_TEMPLATE/download-report.yml`.
pub const ISSUE_TEMPLATE: &str = "download-report.yml";

/// One download's report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadReport {
    /// Always [`REPORT_KIND`].
    pub kind: &'static str,
    /// Always [`REPORT_FORMAT`].
    pub format: u32,
    /// The KNXBench build that planned and ran the download.
    pub knxbench_version: String,
    /// The application program, as the product data names it.
    pub program: ReportedProgram,
    /// `complete`, `partial-parameters`, `partial-group-addresses` or
    /// `partial-both`, as in the verified-download evidence.
    pub scope: &'static str,
    /// The support level the plan had: `verified` or `untested`.
    pub support: &'static str,
    /// The inferences the plan rested on (ADR-0086).
    pub inferences: Vec<String>,
    /// The plan's warnings (ADR-0120).
    pub warnings: Vec<String>,
    /// The plan's size.
    pub plan: ReportedPlan,
    /// How the run ended; `None` while it has not.
    pub outcome: Option<ReportedOutcome>,
    /// The read-only check after the transfer, once the run has ended;
    /// omitted while it runs. Absent in an older report: not checked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_download_check: Option<PostDownloadCheck>,
    /// A stopped run's payload-free cause, step and restart evidence;
    /// omitted for a completed run and in reports older than DIAG-02.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<DownloadFailure>,
    /// The project's values by `ParameterRef` id; absent ones took their
    /// product default.
    pub parameter_values: BTreeMap<String, String>,
}

/// The program a report is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportedProgram {
    /// `ApplicationProgram/@Id`.
    pub id: String,
    /// `@Name`, when the product data has one.
    pub name: Option<String>,
    /// `@ApplicationVersion`.
    pub application_version: Option<String>,
    /// SHA-256 of the program file it was read from; `None` when the
    /// product database no longer lists the program.
    pub source_sha256: Option<String>,
    /// Manufacturer code, four hex digits.
    pub manufacturer: String,
    /// Mask version, four hex digits.
    pub mask: String,
}

/// The size of the plan a report is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportedPlan {
    /// Steps of the plan.
    pub steps: usize,
    /// Octets the plan writes, each read back.
    pub data_octets: usize,
    /// Group-address links of the configuration.
    pub group_links: usize,
}

/// How a run ended, as witnessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportedOutcome {
    /// Whether the run reached its end (`true`) or stopped (`false`).
    pub completed: bool,
    /// What was written.
    pub written: Written,
    /// The closing restart, for a completed run.
    pub restart: Option<Restart>,
    /// The 1-based step a stopped run stopped in, when known.
    pub stopped_in_step: Option<usize>,
}

impl ReportedOutcome {
    /// A run that reached its end.
    pub fn completed(written: Written, restart: Restart) -> Self {
        Self {
            completed: true,
            written,
            restart: Some(restart),
            stopped_in_step: None,
        }
    }

    /// A run that stopped.
    pub fn stopped(written: Written, stopped_in_step: Option<usize>) -> Self {
        Self {
            completed: false,
            written,
            restart: None,
            stopped_in_step,
        }
    }

    /// One line: `written yes, restart acknowledged`.
    pub fn line(&self) -> String {
        let written = match self.written {
            Written::Yes => "yes",
            Written::No => "no",
            Written::Partially => "partially",
        };
        match (self.completed, self.restart, self.stopped_in_step) {
            (true, Some(restart), _) => format!(
                "completed, written {written}, restart {}",
                match restart {
                    Restart::Acknowledged => "acknowledged",
                    Restart::NotInPlan => "not in the plan",
                    Restart::Unconfirmed => "not confirmed",
                }
            ),
            (_, _, Some(step)) => format!("stopped in step {step}, written {written}"),
            _ => format!("stopped, written {written}"),
        }
    }
}

/// The report of `prepared` before its run: everything but the outcome.
/// `level` is the support level the run is admitted with.
pub fn prepare_report(
    conn: &Connection,
    prepared: &PreparedDownload,
    level: &SupportLevel,
    knxbench_version: &str,
) -> Result<DownloadReport, ProductDbError> {
    let program_id = prepared.request.program_id.as_str();
    let identity = program_identity(conn, program_id)?;
    let inferences = match level {
        SupportLevel::Untested { inferences, .. } => {
            inferences.iter().map(ToString::to_string).collect()
        }
        _ => Vec::new(),
    };
    Ok(DownloadReport {
        kind: REPORT_KIND,
        format: REPORT_FORMAT,
        knxbench_version: knxbench_version.to_string(),
        program: ReportedProgram {
            id: program_id.to_string(),
            name: identity.as_ref().and_then(|i| i.name.clone()),
            application_version: identity
                .as_ref()
                .and_then(|i| i.application_version.clone()),
            source_sha256: identity.map(|i| i.source_sha256),
            manufacturer: format!("{:04X}", prepared.plan.manufacturer),
            mask: format!("{:04X}", prepared.plan.mask.0),
        },
        scope: scope_code(prepared.partial.as_ref().map(|(parts, _)| *parts)),
        support: level.code(),
        inferences,
        warnings: prepared.warnings.iter().map(ToString::to_string).collect(),
        plan: ReportedPlan {
            steps: prepared.plan.steps.len(),
            data_octets: prepared.plan.data_octets(),
            group_links: prepared.request.links.len(),
        },
        outcome: None,
        post_download_check: None,
        failure: None,
        parameter_values: prepared.request.values.clone(),
    })
}

impl DownloadReport {
    /// The report with its run's outcome.
    pub fn with_outcome(mut self, outcome: ReportedOutcome) -> Self {
        self.outcome = Some(outcome);
        self
    }

    /// The report with the read-only check that followed its run; a run
    /// that failed or was never checked carries `notChecked`.
    pub fn with_check(mut self, check: PostDownloadCheck) -> Self {
        self.post_download_check = Some(check);
        self
    }

    /// The report with a stopped run's diagnostic; `None` for a completed run.
    pub fn with_failure(mut self, failure: Option<DownloadFailure>) -> Self {
        self.failure = failure;
        self
    }

    /// The report file's contents.
    pub fn to_json(&self) -> String {
        let mut json = serde_json::to_string_pretty(self).expect("a report always serialises");
        json.push('\n');
        json
    }

    /// A file name for the report: `knxbench-download-report-<program>.json`.
    pub fn file_name(&self) -> String {
        let program: String = self
            .program
            .id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        format!("{REPORT_KIND}-{program}.json")
    }

    /// The outcome line, or that the run has not ended.
    pub fn outcome_line(&self) -> String {
        self.outcome
            .map(|outcome| outcome.line())
            .unwrap_or_else(|| "not ended".to_string())
    }

    /// A short plain-text summary for an issue or a mail body. It names the
    /// report file to attach and says what the file contains; it does not
    /// contain the parameter values itself.
    pub fn summary(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "KNXBench download report (format {})", self.format);
        let _ = writeln!(
            out,
            "Program: {}{}{}",
            self.program.id,
            self.program
                .name
                .as_deref()
                .map(|name| format!(" — {name}"))
                .unwrap_or_default(),
            self.program
                .application_version
                .as_deref()
                .map(|version| format!(" (version {version})"))
                .unwrap_or_default()
        );
        let _ = writeln!(
            out,
            "Mask {}h, manufacturer {}h; scope: {}; support: {}",
            self.program.mask, self.program.manufacturer, self.scope, self.support
        );
        let _ = writeln!(out, "Outcome: {}", self.outcome_line());
        match (&self.failure, self.outcome) {
            (Some(failure), _) => {
                let _ = writeln!(out, "Cause: {}", failure.line());
            }
            (None, Some(outcome)) if !outcome.completed => {
                let _ = writeln!(out, "Cause: not recorded in this report");
            }
            _ => {}
        }
        if let Some(check) = &self.post_download_check {
            let exemplar = match check.identity() {
                Some(IdentityCheck::Match(_)) => "; not proven the same physical device",
                _ => "",
            };
            let _ = writeln!(
                out,
                "Read-only check after the download: {}{exemplar}; application running: unknown",
                check.line()
            );
        }
        let _ = writeln!(out, "KNXBench: {}", self.knxbench_version);
        for warning in &self.warnings {
            let rule = warning.split(':').next().unwrap_or(warning);
            let _ = writeln!(out, "Warning: {rule}");
        }
        let _ = writeln!(
            out,
            "Report file: {} — it contains the {} parameter values written, \
             no project or device name and no address.",
            self.file_name(),
            self.parameter_values.len()
        );
        out
    }

    /// The issue form, with the program, version and outcome filled in
    /// (GitHub fills an issue form's text fields from query parameters named
    /// by their ids). The report file is attached by hand.
    pub fn issue_url(&self) -> String {
        format!(
            "{REPOSITORY}/issues/new?template={ISSUE_TEMPLATE}&title={}&program={}&version={}&outcome={}",
            percent_encode(&format!("[Download] {}", self.program.id)),
            percent_encode(&self.program.id),
            percent_encode(&self.knxbench_version),
            percent_encode(&self.outcome_line())
        )
    }

    /// A `mailto:` link to [`REPORT_MAILBOX`] with the summary as body.
    /// The report file is attached by hand.
    pub fn mailto(&self) -> String {
        format!(
            "mailto:{REPORT_MAILBOX}?subject={}&body={}",
            percent_encode(&format!("KNXBench download report: {}", self.program.id)),
            percent_encode(&format!(
                "{}\nPlease attach the report file.\nDoes the device work as configured? \
                 What did you check?\n",
                self.summary()
            ))
        )
    }
}

/// RFC 3986 percent-encoding of everything but unreserved characters, for a
/// query value of an `https:` or `mailto:` URL.
fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> DownloadReport {
        DownloadReport {
            kind: REPORT_KIND,
            format: REPORT_FORMAT,
            knxbench_version: "0.1.0-alpha.10+gabc".into(),
            program: ReportedProgram {
                id: "M-0083_A-0027-15-0BAC".into(),
                name: Some("Taster Plus".into()),
                application_version: Some("15".into()),
                source_sha256: Some("ab".repeat(32)),
                manufacturer: "0083".into(),
                mask: "0701".into(),
            },
            scope: "complete",
            support: "untested",
            inferences: vec![],
            warnings: vec![
                "base-image-low-octet-first: the program … (KNOWN_LIMITATIONS §170, ADR-0120)"
                    .into(),
            ],
            plan: ReportedPlan {
                steps: 25,
                data_octets: 1416,
                group_links: 3,
            },
            outcome: None,
            post_download_check: None,
            failure: None,
            parameter_values: [("P-1_R-1".to_string(), "300".to_string())].into(),
        }
    }

    #[test]
    fn the_file_says_what_it_is_and_carries_values_but_no_installation() {
        let report = report().with_outcome(ReportedOutcome::completed(
            Written::Yes,
            Restart::Acknowledged,
        ));
        let json: serde_json::Value = serde_json::from_str(&report.to_json()).unwrap();
        assert_eq!(json["kind"], "knxbench-download-report");
        assert_eq!(json["format"], 1);
        assert_eq!(json["program"]["mask"], "0701");
        assert_eq!(json["parameterValues"]["P-1_R-1"], "300");
        assert_eq!(json["outcome"]["written"], "yes");
        assert_eq!(json["outcome"]["restart"], "acknowledged");
        let mut keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        let mut expected = [
            "kind",
            "format",
            "knxbenchVersion",
            "program",
            "scope",
            "support",
            "inferences",
            "warnings",
            "plan",
            "outcome",
            "parameterValues",
        ];
        expected.sort_unstable();
        assert_eq!(
            keys, expected,
            "no address, device, project, gateway or error text"
        );
        let mut program: Vec<&str> = json["program"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        program.sort_unstable();
        assert_eq!(
            program,
            [
                "applicationVersion",
                "id",
                "manufacturer",
                "mask",
                "name",
                "sourceSha256"
            ]
        );
    }

    #[test]
    fn outcomes_read_as_one_line() {
        assert_eq!(report().outcome_line(), "not ended");
        let line = |outcome: ReportedOutcome| outcome.line();
        assert_eq!(
            line(ReportedOutcome::completed(
                Written::Yes,
                Restart::Unconfirmed
            )),
            "completed, written yes, restart not confirmed"
        );
        assert_eq!(
            line(ReportedOutcome::stopped(Written::Partially, Some(7))),
            "stopped in step 7, written partially"
        );
        assert_eq!(
            line(ReportedOutcome::stopped(Written::No, None)),
            "stopped, written no"
        );
    }

    #[test]
    fn a_stopped_run_names_its_cause_or_says_it_was_not_recorded() {
        let stopped = report().with_outcome(ReportedOutcome::stopped(Written::Partially, Some(4)));
        assert!(stopped
            .summary()
            .contains("Cause: not recorded in this report\n"));
        let plan = knx_core::commissioning::memory_download::MemoryDownloadPlan {
            mask: knx_core::commissioning::load_state::MaskVersion(0x0701),
            manufacturer: 0x0083,
            steps: vec![knx_core::commissioning::memory_download::MemoryDownloadStep::Connect],
        };
        let failure = DownloadFailure::task_ended(&plan, Some(1), 0);
        let line = failure.line();
        let recorded = stopped.with_failure(Some(failure));
        assert!(recorded.summary().contains(&format!("Cause: {line}\n")));
        assert!(!recorded.summary().contains("not recorded"));
        let completed = report().with_outcome(ReportedOutcome::completed(
            Written::Yes,
            Restart::Acknowledged,
        ));
        assert!(!completed.summary().contains("Cause:"));
    }

    #[test]
    fn the_summary_names_the_file_and_withholds_the_values() {
        let summary = report().summary();
        assert!(summary.contains("Program: M-0083_A-0027-15-0BAC — Taster Plus (version 15)"));
        assert!(summary.contains("Warning: base-image-low-octet-first\n"));
        assert!(summary.contains("knxbench-download-report-M-0083_A-0027-15-0BAC.json"));
        assert!(summary.contains("1 parameter values"));
        assert!(!summary.contains("300"), "values stay in the file");
    }

    #[test]
    fn the_links_fill_the_form_and_the_mail() {
        let report = report().with_outcome(ReportedOutcome::stopped(Written::No, Some(3)));
        assert_eq!(
            report.issue_url(),
            "https://github.com/KNXBench-Labs/KNXBench/issues/new?template=download-report.yml\
             &title=%5BDownload%5D%20M-0083_A-0027-15-0BAC&program=M-0083_A-0027-15-0BAC\
             &version=0.1.0-alpha.10%2Bgabc&outcome=stopped%20in%20step%203%2C%20written%20no"
        );
        let mail = report.mailto();
        assert!(mail.starts_with(
            "mailto:issue@knxbench.com?subject=KNXBench%20download%20report%3A%20M-0083_A-0027-15-0BAC&body="
        ));
        assert!(mail.contains("Please%20attach%20the%20report%20file."));
        assert!(!mail.contains(' ') && !mail.contains('\n'));
    }

    #[test]
    fn percent_encoding_keeps_only_unreserved_characters() {
        assert_eq!(percent_encode("aZ09-._~"), "aZ09-._~");
        assert_eq!(percent_encode("a b&c=d/ü"), "a%20b%26c%3Dd%2F%C3%BC");
    }

    #[test]
    fn odd_program_ids_make_safe_file_names() {
        let mut report = report();
        report.program.id = "M-0001/../x y".into();
        assert_eq!(
            report.file_name(),
            "knxbench-download-report-M-0001____x_y.json"
        );
    }

    /// The public summary keeps the exemplar caveat a matching identity needs.
    #[test]
    fn a_matching_check_in_the_summary_does_not_claim_the_same_device() {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../../apps/knx-web/src/postDownloadCheck.fixtures.json"
        ))
        .unwrap();
        let check = |name: &str| -> PostDownloadCheck {
            serde_json::from_value(fixtures[name].clone()).unwrap()
        };
        let summary = report().with_check(check("matched")).summary();
        assert!(
            summary.contains(
                "Read-only check after the download: reached; identity matches the plan; \
                 load states match the plan; not proven the same physical device; \
                 application running: unknown"
            ),
            "{summary}"
        );
        let unreached = report().with_check(check("notReached")).summary();
        assert!(!unreached.contains("physical device"), "{unreached}");
        assert!(unreached.contains("application running: unknown"));
    }

    /// The link fills the issue form's fields by id: the form must have them.
    #[test]
    fn the_issue_form_has_the_fields_the_link_fills() {
        let form = include_str!("../../../.github/ISSUE_TEMPLATE/download-report.yml");
        for id in ["program", "version", "outcome"] {
            assert!(form.contains(&format!("    id: {id}\n")), "field {id}");
        }
        assert_eq!(ISSUE_TEMPLATE, "download-report.yml");
        assert!(form.contains(REPORT_MAILBOX));
    }
}
