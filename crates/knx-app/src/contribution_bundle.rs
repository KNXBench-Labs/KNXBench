//! Deterministic evidence ZIPs; disclosure is explicit and no network exists here.
use super::contribution::{self, Analysis, AnalysisError, FORMAT_VERSION};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Write};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Audience {
    Public,
    Private,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BundleOptions {
    pub audience: Audience,
    #[serde(default)]
    pub sample_ids: Vec<String>,
    #[serde(default)]
    pub include_original: bool,
    #[serde(default)]
    pub consent: bool,
    #[serde(default)]
    pub original_consent: bool,
    #[serde(default)]
    pub expected_manifest_sha256: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub path: String,
    pub size: usize,
    pub sha256: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub format_version: u32,
    pub analyzer_version: String,
    pub audience: Audience,
    pub disclosure: String,
    pub original_sha256: Option<String>,
    pub files: Vec<Artifact>,
    pub limitations: Vec<String>,
}
pub struct Bundle {
    pub bytes: Vec<u8>,
    pub manifest: Manifest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub manifest: Manifest,
    pub manifest_sha256: String,
    pub files: Vec<PreviewFile>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewFile {
    pub path: String,
    pub text: Option<String>,
}

struct Prepared {
    manifest: Manifest,
    files: Vec<(String, Vec<u8>)>,
}

pub fn preview_bundle(
    bytes: &[u8],
    filename: &str,
    options: &BundleOptions,
) -> Result<Preview, AnalysisError> {
    let prepared = prepare(bytes, filename, options)?;
    let manifest_sha256 = manifest_digest(&prepared.manifest)?;
    Ok(Preview {
        manifest: prepared.manifest,
        manifest_sha256,
        files: prepared
            .files
            .into_iter()
            .map(|(path, bytes)| {
                let text = if path.starts_with("original/") {
                    None
                } else {
                    String::from_utf8(bytes).ok()
                };
                PreviewFile { path, text }
            })
            .collect(),
    })
}

pub fn build_bundle(
    bytes: &[u8],
    filename: &str,
    options: &BundleOptions,
) -> Result<Bundle, AnalysisError> {
    if !options.consent {
        return Err(AnalysisError::Input(
            "review the disclosure and explicitly consent before exporting",
        ));
    }
    let Prepared { manifest, files } = prepare(bytes, filename, options)?;
    if options
        .expected_manifest_sha256
        .as_ref()
        .is_some_and(|expected| manifest_digest(&manifest).as_ref() != Ok(expected))
    {
        return Err(AnalysisError::Input(
            "evidence changed since preview; preview and consent again",
        ));
    }
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let file_options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("manifest.json", file_options)
        .map_err(|_| AnalysisError::Internal("evidence ZIP failed"))?;
    zip.write_all(
        &serde_json::to_vec_pretty(&manifest)
            .map_err(|_| AnalysisError::Internal("manifest serialization failed"))?,
    )
    .map_err(|_| AnalysisError::Internal("evidence ZIP failed"))?;
    for (path, data) in files {
        zip.start_file(path, file_options)
            .map_err(|_| AnalysisError::Internal("evidence ZIP failed"))?;
        zip.write_all(&data)
            .map_err(|_| AnalysisError::Internal("evidence ZIP failed"))?;
    }
    let bytes = zip
        .finish()
        .map_err(|_| AnalysisError::Internal("evidence ZIP failed"))?
        .into_inner();
    if options.audience == Audience::Public && bytes.len() > 24_000_000 {
        return Err(AnalysisError::Input(
            "public evidence exceeds 24 MB; select fewer samples",
        ));
    }
    Ok(Bundle { bytes, manifest })
}

fn prepare(
    bytes: &[u8],
    filename: &str,
    options: &BundleOptions,
) -> Result<Prepared, AnalysisError> {
    if options.include_original
        && (options.audience != Audience::Private || !options.original_consent)
    {
        return Err(AnalysisError::Input(
            "originals require private audience and separate original consent",
        ));
    }
    if options.sample_ids.len() > 16 {
        return Err(AnalysisError::Input("select at most 16 context members"));
    }
    let mut report = contribution::analyze(bytes, filename)?;
    let mut selected = std::collections::BTreeSet::new();
    let mut context = Vec::new();
    for id in &options.sample_ids {
        if !selected.insert(id) {
            return Err(AnalysisError::Input("duplicate context selection"));
        }
        let member = report
            .members
            .iter()
            .find(|m| &m.id == id)
            .ok_or(AnalysisError::Input("unknown context selection"))?;
        if !member.sample_allowed {
            return Err(AnalysisError::Input(
                "member is not shareable XML or contains known key material",
            ));
        }
        context.push((
            format!("samples/{id}.xml"),
            super::contribution_structure::sample(bytes, &member.path)?,
        ));
    }
    if options.include_original {
        if !report.original_allowed {
            return Err(AnalysisError::Input(
                "original contains unexamined data or known key material",
            ));
        }
        let extension = if report.kind == "project" {
            "knxproj"
        } else {
            "knxprod"
        };
        context.push((format!("original/source.{extension}"), bytes.to_vec()));
    }
    reduce(&mut report);
    let findings = serde_json::to_vec_pretty(&report)
        .map_err(|_| AnalysisError::Internal("evidence serialization failed"))?;
    let readme = format!("# KNXBench contribution evidence v{FORMAT_VERSION}\n\nThis is a reduced report, not an anonymization or compatibility guarantee.\nReduced findings exclude source values. Selected context XML and any private original are UNMODIFIED and may contain personal or proprietary data; a project sample or original also carries its retained network endpoints (tunnelling targets, IP configuration), MAC addresses, additional device addresses and the user names in ETS project traces. Inspect the manifest and every selected sample before sharing.\n\n## Maintainer validation\n\n1. Inspect findings and check availability. Zero findings is not complete support.\n2. Request an explicitly shared contextual sample or private original when needed.\n3. Reproduce with the reported analyzer version; compare specs/product/project facts.\n4. Derive a permission-reviewed regression fixture and independently justify its expected result.\n5. Implement, rerun import/retention/rollback and offline-plan regressions, and disclose remaining gaps.\n\nNo hardware was contacted; offline plans do not establish hardware verification.\nPublic GitHub attachments are public immediately. Mail drafts do not send or attach files.\n");
    let mut files = vec![
        ("findings.json".to_string(), findings),
        ("README.md".to_string(), readme.into_bytes()),
    ];
    files.extend(context);
    let manifest = Manifest {
        format_version: FORMAT_VERSION, analyzer_version: report.analyzer_version.clone(), audience: options.audience,
        disclosure: if options.include_original { "private-original" } else if options.sample_ids.is_empty() { "reduced" } else { "context-samples" }.into(),
        original_sha256: options.include_original.then(|| knx_productdb::sha256_hex(bytes)),
        files: files.iter().map(|(path, data)| Artifact { path: path.clone(), size: data.len(), sha256: knx_productdb::sha256_hex(data) }).collect(),
        limitations: vec!["Reduced evidence omits values, concrete source identity and detailed diagnostics; context may be needed to reproduce".into(), "No automatic anonymization, semantic completeness, ETS parity or hardware verification".into()],
    };
    Ok(Prepared { manifest, files })
}

fn manifest_digest(manifest: &Manifest) -> Result<String, AnalysisError> {
    serde_json::to_vec_pretty(manifest)
        .map(|bytes| knx_productdb::sha256_hex(&bytes))
        .map_err(|_| AnalysisError::Internal("manifest serialization failed"))
}

fn reduce(report: &mut Analysis) {
    report.members.clear();
    report.procedure_resolutions.clear();
    for finding in &mut report.findings {
        finding.source_path = None;
        finding.sample = None;
        if finding.stage.starts_with("offline") {
            finding.name = None;
        }
        // Omit predicate-bearing paths wholesale: values may contain slashes or brackets.
        // Full value-free expanded ancestry stays in the separate structure observations.
        finding.xpath = finding
            .xpath
            .as_ref()
            .filter(|p| !p.contains(['[', ']', '\'', '"']))
            .cloned();
        finding.detail = match finding.category.as_str() {
            "unknown" => "Uninterpreted construct; request context for semantic validation",
            "unsupported" => {
                "Recognized capability boundary; request context for the exact refusal"
            }
            "inference" => {
                "Offline support relies on an inference; request context for the named rule"
            }
            "observation" => {
                "Declarative procedure observation; not global novelty or hardware support"
            }
            "retained" => "Preserved source data not interpreted by the domain model",
            "conflict" => "Validation or mapping conflict; request context",
            _ => "Check refused; detailed source-bearing diagnostics withheld",
        }
        .into();
    }
}
