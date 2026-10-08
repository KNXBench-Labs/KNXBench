//! Read-only compatibility evidence on a disposable product database.
//!
//! Local diagnostics are source data, never an automatically public report.
use serde::{Deserialize, Serialize};

pub const FORMAT_VERSION: u32 = 1;
pub const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Finding {
    pub id: String,
    pub stage: String,
    pub category: String,
    pub source_path: Option<String>,
    pub xpath: Option<String>,
    pub name: Option<String>,
    pub occurrences: u64,
    pub detail: String,
    pub sample: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Check {
    pub name: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Member {
    pub id: String,
    pub path: String,
    pub size: u64,
    pub sample_allowed: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Analysis {
    pub format_version: u32,
    pub analyzer_version: String,
    pub kind: String,
    pub scheme: Option<u32>,
    pub status: String,
    pub source_size: usize,
    pub findings: Vec<Finding>,
    pub checks: Vec<Check>,
    pub members: Vec<Member>,
    pub structure: Vec<super::contribution_structure::Shape>,
    pub original_allowed: bool,
    pub metrics: Vec<Metric>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Metric {
    pub stage: String,
    pub entity: String,
    pub disposition: String,
    pub count: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AnalysisError {
    Input(&'static str),
    Internal(&'static str),
}
impl std::fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(s) | Self::Internal(s) => f.write_str(s),
        }
    }
}
impl std::error::Error for AnalysisError {}

/// Source names only choose the explicit file-kind boundary; never persisted.
/// No caller project/product connection or transport can reach this service.
pub fn analyze(bytes: &[u8], filename: &str) -> Result<Analysis, AnalysisError> {
    if bytes.is_empty() || bytes.len() > MAX_INPUT_BYTES {
        return Err(AnalysisError::Input(
            "analysis input must be between 1 byte and 32 MiB",
        ));
    }
    let project_input = filename.to_ascii_lowercase().ends_with(".knxproj");
    if !project_input && !filename.to_ascii_lowercase().ends_with(".knxprod") {
        return Err(AnalysisError::Input(
            "select a .knxproj project or .knxprod product package",
        ));
    }
    let dir = tempfile::tempdir()
        .map_err(|_| AnalysisError::Internal("temporary analysis workspace unavailable"))?;
    let db = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite"))
        .map_err(|_| AnalysisError::Internal("temporary analysis database unavailable"))?;
    let mut analysis = Analysis {
        format_version: FORMAT_VERSION,
        analyzer_version: env!("CARGO_PKG_VERSION").into(),
        kind: if project_input { "project" } else { "product" }.into(),
        scheme: None,
        status: "refused".into(),
        source_size: bytes.len(),
        findings: vec![],
        checks: vec![],
        members: vec![],
        structure: vec![],
        original_allowed: false,
        metrics: vec![],
    };
    let complete = super::contribution_structure::inspect(bytes, &mut analysis)?;
    if !complete {
        analysis.status = "partial".into();
        analysis.checks.push(Check {
            name: "typed-import".into(),
            status: "not-examined".into(),
            detail: "Structural scan incomplete; typed import not attempted".into(),
        });
        for name in [
            format!("{}-import", analysis.kind),
            format!("offline-{}-plan", analysis.kind),
        ] {
            analysis.checks.push(Check {
                name,
                status: "not-examined".into(),
                detail: "Structural scan incomplete; downstream stage not run".into(),
            });
        }
        return Ok(analysis);
    }
    if project_input {
        analysis.kind = "project".into();
        analyze_project(bytes, &db, &mut analysis);
    } else {
        match knx_productdb::install_package(&db, "source.knxprod", bytes) {
            Ok(report) => {
                analysis.scheme = Some(report.scheme);
                analysis.status = "complete".into();

                for conflict in report.conflicts {
                    analysis.findings.push(Finding {
                        id: String::new(),
                        stage: "product-import".into(),
                        category: "conflict".into(),
                        source_path: None,
                        xpath: None,
                        name: None,
                        occurrences: conflict.occurrence.into(),
                        detail: format!("{conflict:?}"),
                        sample: None,
                    });
                }
                if let Some(facts) = report.facts {
                    analysis
                        .metrics
                        .extend(facts.counts.into_iter().map(|r| Metric {
                            stage: "product-import".into(),
                            entity: r.category.as_str().into(),
                            disposition: r.disposition.as_str().into(),
                            count: r.count,
                        }));
                    for row in facts.unknown_constructs {
                        analysis.findings.push(Finding {
                            id: String::new(),
                            stage: "product-import".into(),
                            category: "unknown".into(),
                            source_path: None,
                            xpath: Some(row.xpath),
                            name: Some(row.name),
                            occurrences: row.occurrences.into(),
                            detail: "Retained but not interpreted by the product parser".into(),
                            sample: row.sample,
                        });
                    }
                    for row in facts.diagnostics {
                        analysis.findings.push(Finding {
                            id: String::new(),
                            stage: "product-import".into(),
                            category: "unsupported".into(),
                            source_path: Some(row.archive_path().into()),
                            xpath: Some(row.xml_path().into()),
                            name: Some(row.kind().as_str().into()),
                            occurrences: row.occurrences(),
                            detail: row.detail().into(),
                            sample: None,
                        });
                    }
                }
                analysis.checks.push(Check {
                    name: "product-import".into(),
                    status: "measured".into(),
                    detail:
                        "Installed only into a disposable database; user product data unchanged"
                            .into(),
                });
                match knx_productdb::query::programs(&db, None) {
                    Ok(programs) => {
                        let total = programs.len();
                        for program in programs.into_iter().take(64) {
                            record_level(
                                &mut analysis,
                                "offline-product-plan",
                                &program.id,
                                crate::download_support::support_level(
                                    &db,
                                    &program.id,
                                    &std::collections::BTreeMap::new(),
                                ),
                            );
                        }
                        let status = if total > 64 {
                            analysis.status = "partial".into();
                            "partial"
                        } else {
                            "measured"
                        };
                        analysis.checks.push(Check { name: "offline-product-plan".into(), status: status.into(), detail: format!("Product defaults only; examined {} of {total} programs, no hardware evidence or contact", total.min(64)) });
                    }
                    Err(_) => analysis.checks.push(Check {
                        name: "offline-product-plan".into(),
                        status: "unavailable".into(),
                        detail: "Program enumeration unavailable".into(),
                    }),
                }
            }
            Err(error) => {
                analysis.findings.push(Finding {
                    id: String::new(),
                    stage: "product-import".into(),
                    category: "refused".into(),
                    source_path: None,
                    xpath: None,
                    name: None,
                    occurrences: 1,
                    detail: error.to_string(),
                    sample: None,
                });
                analysis.checks.push(Check {
                    name: "product-import".into(),
                    status: "refused".into(),
                    detail: "No typed import accepted; this is not a compatibility approval".into(),
                });
            }
        }
    }
    let plan_check = format!("offline-{}-plan", analysis.kind);
    if !analysis.checks.iter().any(|c| c.name == plan_check) {
        analysis.checks.push(Check {
            name: plan_check,
            status: "not-examined".into(),
            detail: "Typed import refused; offline plan not run".into(),
        });
    }
    let sensitive = analysis
        .members
        .iter()
        .any(|m| m.reason.starts_with("Known credential"));
    for (i, finding) in analysis.findings.iter_mut().enumerate() {
        finding.id = format!("finding-{}", i + 1);
        if sensitive {
            finding.sample = None;
            finding.detail = "Detailed source-bearing diagnostics withheld because known key material was detected".into();
        }
    }
    Ok(analysis)
}

fn analyze_project(bytes: &[u8], db: &knx_productdb::Connection, analysis: &mut Analysis) {
    let outcome = match knx_etsproj::import_knxproj_bytes(bytes.to_vec(), "source.knxproj") {
        Ok(outcome) => outcome,
        Err(error) => {
            analysis.findings.push(Finding {
                id: String::new(),
                stage: "project-import".into(),
                category: "refused".into(),
                source_path: None,
                xpath: None,
                name: None,
                occurrences: 1,
                detail: error.to_string(),
                sample: None,
            });
            analysis.checks.push(Check {
                name: "project-import".into(),
                status: "refused".into(),
                detail: "Typed project import was refused; no compatibility approval".into(),
            });
            return;
        }
    };
    analysis.scheme = Some(outcome.report.source.schema_version);
    analysis.status = "complete".into();
    for row in outcome.report.counts.rows {
        for (disposition, count) in [("read", row.read), ("mapped", row.mapped)] {
            analysis.metrics.push(Metric {
                stage: "project-import".into(),
                entity: row.entity.clone(),
                disposition: disposition.into(),
                count: count.into(),
            });
        }
    }
    for row in outcome.report.inferred {
        analysis.findings.push(Finding {
            id: String::new(),
            stage: "project-import".into(),
            category: "inference".into(),
            source_path: None,
            xpath: None,
            name: None,
            occurrences: 1,
            detail: format!("{row:?}"),
            sample: None,
        });
    }
    for row in outcome.report.conflicts {
        analysis.findings.push(Finding {
            id: String::new(),
            stage: "project-import".into(),
            category: "conflict".into(),
            source_path: None,
            xpath: None,
            name: None,
            occurrences: 1,
            detail: format!("{row:?}"),
            sample: None,
        });
    }
    if let Some(detail) = outcome.report.source.namespace_disagreement {
        analysis.findings.push(Finding {
            id: String::new(),
            stage: "project-import".into(),
            category: "conflict".into(),
            source_path: None,
            xpath: None,
            name: None,
            occurrences: 1,
            detail,
            sample: None,
        });
    }
    for row in outcome.report.unknown {
        analysis.findings.push(Finding {
            id: String::new(),
            stage: "project-import".into(),
            category: "unknown".into(),
            source_path: Some(row.source_path),
            xpath: Some(row.xpath),
            name: Some(row.name),
            occurrences: row.occurrences.into(),
            detail: "Preserved source construct not interpreted by the project parser".into(),
            sample: row.sample,
        });
    }
    for row in outcome.report.errors {
        analysis.findings.push(Finding {
            id: String::new(),
            stage: row.stage.into(),
            category: "conflict".into(),
            source_path: None,
            xpath: Some(row.xpath),
            name: None,
            occurrences: 1,
            detail: row.detail,
            sample: None,
        });
    }
    for row in outcome.report.unsupported {
        analysis.findings.push(Finding {
            id: String::new(),
            stage: "project-import".into(),
            category: "unsupported".into(),
            source_path: None,
            xpath: None,
            name: None,
            occurrences: 1,
            detail: format!("{}: {}", row.what, row.consequence),
            sample: None,
        });
    }
    for row in outcome
        .report
        .opaque
        .into_iter()
        .filter(|r| !r.xpath.is_empty())
    {
        analysis.findings.push(Finding {
            id: String::new(),
            stage: "project-import".into(),
            category: "retained".into(),
            source_path: Some(row.source_path),
            xpath: Some(row.xpath),
            name: Some(row.name),
            occurrences: 1,
            detail: row.reason,
            sample: None,
        });
    }
    analysis.checks.push(Check {
        name: "project-import".into(),
        status: "measured".into(),
        detail: "Actual project import/validation/mapping; no user project opened or saved".into(),
    });
    for file in outcome.manufacturer {
        if let Err(error) = knx_productdb::ingest_file(db, &file.source_path, &file.bytes) {
            analysis.status = "partial".into();
            analysis.findings.push(Finding {
                id: String::new(),
                stage: "manufacturer-data".into(),
                category: "refused".into(),
                source_path: Some(file.source_path),
                xpath: None,
                name: None,
                occurrences: 1,
                detail: error.to_string(),
                sample: None,
            });
        }
    }
    if outcome.project.devices.iter().take(65).count() > 64 {
        analysis.status = "partial".into();
        analysis.checks.push(Check {
            name: "offline-project-plan".into(),
            status: "not-examined".into(),
            detail: "Project has more than 64 devices; bounded analysis does not run device plans"
                .into(),
        });
        return;
    }
    let rows = crate::project_readiness::project_readiness(
        db,
        &outcome.project,
        &std::collections::BTreeMap::new(),
    );
    for row in rows {
        match row.readiness {
            crate::project_readiness::DeviceReadiness::Graded(level) => {
                record_level(analysis, "offline-project-plan", &row.program_ref, level)
            }
            disposition => analysis.findings.push(Finding {
                id: String::new(),
                stage: "offline-project-plan".into(),
                category: disposition.code().into(),
                source_path: None,
                xpath: None,
                name: Some(row.program_ref),
                occurrences: 1,
                detail: "Device not prepared: excluded or missing individual address".into(),
                sample: None,
            }),
        }
    }
    analysis.checks.push(Check {
        name: "offline-project-plan".into(),
        status: "measured".into(),
        detail: "Project values and links prepared offline; no hardware verified or contacted"
            .into(),
    });
}

fn record_level(
    analysis: &mut Analysis,
    stage: &str,
    program: &str,
    level: crate::download_support::SupportLevel,
) {
    use crate::download_support::SupportLevel;
    let (category, detail) = match level {
        SupportLevel::Unsupported { category, detail } => {
            ("unsupported", format!("{}: {}", category.code(), detail))
        }
        SupportLevel::Untested { inferences, .. } if !inferences.is_empty() => (
            "inference",
            inferences
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; "),
        ),
        SupportLevel::Untested { .. } => (
            "untested",
            "Offline plan prepared, no hardware verification".into(),
        ),
        SupportLevel::Verified { .. } => return,
    };
    analysis.findings.push(Finding {
        id: String::new(),
        stage: stage.into(),
        category: category.into(),
        source_path: None,
        xpath: None,
        name: Some(program.into()),
        occurrences: 1,
        detail,
        sample: None,
    });
}
