//! Loads device configuration modules, validates their documents and produces their exports.
//!
//! A module is declarative data (ADR-0175, `docs/contracts/device-configuration-modules.md`):
//! a document type, rules as JSON expression trees and binary layouts. This crate is pure:
//! no I/O, no project types, deterministic results and explicit limits.

mod export;
mod expr;
mod json;
mod rules;
mod schema;

pub use export::crc16_ccitt_false;
pub use expr::EvalError;
pub use json::Json;

use sha2::{Digest, Sha256};
use std::fmt;

/// Largest module package accepted.
pub const MAX_MODULE_BYTES: usize = 262_144;
/// Largest `max_document_bytes` a module may declare.
pub const MAX_DOCUMENT_BYTES: usize = 1_048_576;
/// Evaluation steps one validation or one export may use.
pub const STEP_LIMIT: u64 = 50_000_000;
/// Findings reported before the rest are only counted.
pub const MAX_FINDINGS: usize = 1000;
/// The module format this engine reads.
pub const FORMAT: &str = "knxbench-device-configuration-module";
/// The module format version this engine reads.
pub const FORMAT_VERSION: i64 = 1;

/// Whether a finding comes from the document's structure or from a module rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingKind {
    Structure,
    Rule,
}

/// One problem in a document, located by JSON pointer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub kind: FindingKind,
    /// JSON pointer into the document; empty for the document as a whole.
    pub path: String,
    /// The violated rule's id; `None` for structural findings.
    pub rule: Option<String>,
    pub message: String,
}

impl Finding {
    pub(crate) fn structure(path: &str, message: String) -> Self {
        Finding {
            kind: FindingKind::Structure,
            path: path.to_owned(),
            rule: None,
            message,
        }
    }

    pub(crate) fn rule(path: &str, rule: &str, message: &str) -> Self {
        Finding {
            kind: FindingKind::Rule,
            path: path.to_owned(),
            rule: Some(rule.to_owned()),
            message: message.to_owned(),
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let path = if self.path.is_empty() {
            "(document)"
        } else {
            &self.path
        };
        match &self.rule {
            Some(rule) => write!(f, "{path}: {} [{rule}]", self.message),
            None => write!(f, "{path}: {}", self.message),
        }
    }
}

/// The result of validating one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validation {
    /// Reported findings, at most [`MAX_FINDINGS`].
    pub findings: Vec<Finding>,
    /// Findings beyond [`MAX_FINDINGS`] that were counted but not kept.
    pub omitted: usize,
}

impl Validation {
    pub fn is_valid(&self) -> bool {
        self.findings.is_empty() && self.omitted == 0
    }

    fn from(mut findings: Vec<Finding>) -> Self {
        let omitted = findings.len().saturating_sub(MAX_FINDINGS);
        findings.truncate(MAX_FINDINGS);
        Validation { findings, omitted }
    }
}

/// A module package that cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError(pub String);

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LoadError {}

/// Validation or export stopped without a result for reasons other than the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    /// The evaluation step budget was exhausted.
    Budget,
    /// The module asked for something its declared types do not allow.
    Module(String),
}

impl From<EvalError> for EngineError {
    fn from(e: EvalError) -> Self {
        match e {
            EvalError::Budget => EngineError::Budget,
            EvalError::Module(m) => EngineError::Module(m),
        }
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EngineError::Budget => write!(f, "evaluation exceeded {STEP_LIMIT} steps"),
            EngineError::Module(m) => write!(f, "module error: {m}"),
        }
    }
}

impl std::error::Error for EngineError {}

/// Why an export produced no bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    /// The module declares no export with this id.
    UnknownExport(String),
    /// The document is not valid; nothing is exported.
    Invalid(Validation),
    /// A value of a valid document does not fit the layout.
    Refused {
        path: String,
        message: String,
    },
    Engine(EngineError),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ExportError::UnknownExport(id) => write!(f, "the module has no export \"{id}\""),
            ExportError::Invalid(v) => {
                write!(
                    f,
                    "the document is invalid ({} findings)",
                    v.findings.len() + v.omitted
                )
            }
            ExportError::Refused { path, message } => {
                let path = if path.is_empty() { "(document)" } else { path };
                write!(f, "export refused at {path}: {message}")
            }
            ExportError::Engine(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for ExportError {}

/// Summary of one declared export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportInfo {
    pub id: String,
    pub title: String,
    pub extension: String,
    pub max_bytes: usize,
}

/// One declared member, for module authors and editors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberInfo {
    pub name: String,
    /// `int`, `enum`, `text`, `const`, `array`, `ref` or `table`.
    pub kind: &'static str,
    pub title: Option<String>,
    pub description: Option<String>,
}

/// One declared table, for module authors and editors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableInfo {
    pub name: String,
    pub min_rows: usize,
    pub max_rows: usize,
    pub key: Option<String>,
    pub columns: Vec<MemberInfo>,
    pub title: Option<String>,
    pub description: Option<String>,
}

/// The declared shape of a module's document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outline {
    pub members: Vec<MemberInfo>,
    pub tables: Vec<TableInfo>,
    pub rules: usize,
}

fn member_info(m: &schema::Member) -> MemberInfo {
    let kind = match m.ty {
        schema::Ty::Int { .. } => "int",
        schema::Ty::Enum { .. } => "enum",
        schema::Ty::Text { .. } => "text",
        schema::Ty::Const(_) => "const",
        schema::Ty::Array { .. } => "array",
        schema::Ty::Ref { .. } => "ref",
        schema::Ty::Table(_) => "table",
    };
    MemberInfo {
        name: m.name.clone(),
        kind,
        title: m.title.clone(),
        description: m.description.clone(),
    }
}

/// A loaded, checked module package.
#[derive(Debug)]
pub struct Module {
    id: String,
    version: String,
    title: String,
    sha256: String,
    max_document_bytes: usize,
    schema: schema::Schema,
    rules: Vec<rules::Rule>,
    exports: Vec<export::Export>,
}

fn valid_module_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    (1..=64).contains(&bytes.len())
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'.' || *b == b'-')
}

fn valid_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.bytes().all(|b| b.is_ascii_digit())
                && (p.len() == 1 || !p.starts_with('0'))
                && p.parse::<u32>().is_ok_and(|n| n <= 65535)
        })
}

impl Module {
    /// Loads and checks a module package; any problem refuses the whole module.
    pub fn load(bytes: &[u8]) -> Result<Module, LoadError> {
        let json =
            json::parse(bytes, MAX_MODULE_BYTES).map_err(|e| LoadError(format!("module: {e}")))?;
        Self::from_json(&json, bytes).map_err(LoadError)
    }

    fn from_json(json: &Json, bytes: &[u8]) -> Result<Module, String> {
        let f = json::fields(
            json,
            "",
            &[
                "format",
                "format_version",
                "id",
                "version",
                "title",
                "max_document_bytes",
                "document",
                "rules",
                "exports",
            ],
            &[],
        )?;
        if f.str("format")? != FORMAT {
            return Err(format!(
                "/format: not a device configuration module (expected \"{FORMAT}\")"
            ));
        }
        let format_version = f.int("format_version", i64::MIN, i64::MAX)?;
        if format_version != FORMAT_VERSION {
            return Err(format!(
                "/format_version: unsupported version {format_version}; this KNXBench reads version {FORMAT_VERSION}"
            ));
        }
        let id = f.str("id")?;
        if !valid_module_id(id) {
            return Err(format!("/id: \"{id}\" is not a valid module id"));
        }
        let version = f.str("version")?;
        if !valid_version(version) {
            return Err(format!("/version: \"{version}\" is not MAJOR.MINOR.PATCH"));
        }
        let title = f.str("title")?;
        if !schema::printable(title, 80) {
            return Err("/title: must be 1..80 printable characters".into());
        }
        let max_document_bytes =
            usize::try_from(f.int("max_document_bytes", 1, MAX_DOCUMENT_BYTES as i64)?)
                .expect("bounded");
        let schema = schema::load(f.req("document"), "/document")?;
        let rules = rules::load(f.req("rules"), &schema, "/rules")?;
        let exports = export::load(f.req("exports"), &schema, "/exports")?;
        Ok(Module {
            id: id.to_owned(),
            version: version.to_owned(),
            title: title.to_owned(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
            max_document_bytes,
            schema,
            rules,
            exports,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    /// SHA-256 of the exact package bytes, lowercase hex.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    pub fn max_document_bytes(&self) -> usize {
        self.max_document_bytes
    }

    pub fn exports(&self) -> Vec<ExportInfo> {
        self.exports
            .iter()
            .map(|e| ExportInfo {
                id: e.id.clone(),
                title: e.title.clone(),
                extension: e.extension.clone(),
                max_bytes: e.max_bytes,
            })
            .collect()
    }

    /// The declared document shape: root members, tables and the number of rules.
    pub fn outline(&self) -> Outline {
        let tables = self
            .schema
            .tables
            .iter()
            .map(|t| TableInfo {
                name: t.name.clone(),
                min_rows: t.min_rows,
                max_rows: t.max_rows,
                key: t.key.map(|k| t.columns[k].name.clone()),
                columns: t.columns.iter().map(member_info).collect(),
                title: t.title.clone(),
                description: t.description.clone(),
            })
            .collect();
        Outline {
            members: self.schema.root.iter().map(member_info).collect(),
            tables,
            rules: self.rules.len(),
        }
    }

    fn check(
        &self,
        document: &[u8],
        limit: u64,
    ) -> Result<Result<schema::Doc, Validation>, EngineError> {
        let json = match json::parse(document, self.max_document_bytes) {
            Ok(json) => json,
            Err(e) => return Ok(Err(Validation::from(vec![Finding::structure("", e)]))),
        };
        let doc = match schema::check(&self.schema, &json) {
            Ok(doc) => doc,
            Err(findings) => return Ok(Err(Validation::from(findings))),
        };
        let findings = rules::run(&self.rules, &self.schema, &doc, limit)?;
        Ok(if findings.is_empty() {
            Ok(doc)
        } else {
            Err(Validation::from(findings))
        })
    }

    /// Validates a document: structure first, then every rule.
    pub fn validate(&self, document: &[u8]) -> Result<Validation, EngineError> {
        Ok(match self.check(document, STEP_LIMIT)? {
            Ok(_) => Validation {
                findings: Vec::new(),
                omitted: 0,
            },
            Err(validation) => validation,
        })
    }

    /// Produces the bytes of export `export_id` for a valid document.
    pub fn export(&self, document: &[u8], export_id: &str) -> Result<Vec<u8>, ExportError> {
        let export = self
            .exports
            .iter()
            .find(|e| e.id == export_id)
            .ok_or_else(|| ExportError::UnknownExport(export_id.to_owned()))?;
        let doc = match self
            .check(document, STEP_LIMIT)
            .map_err(ExportError::Engine)?
        {
            Ok(doc) => doc,
            Err(validation) => return Err(ExportError::Invalid(validation)),
        };
        export::run(export, &self.schema, &doc, STEP_LIMIT).map_err(|r| match r {
            export::Refusal::Value { path, message } => ExportError::Refused { path, message },
            export::Refusal::Eval(e) => ExportError::Engine(e.into()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_stops_at_the_step_budget() {
        let package = serde_json::json!({
            "format": FORMAT, "format_version": 1, "id": "budget", "version": "1.0.0", "title": "Budget",
            "max_document_bytes": 65536,
            "document": { "type": "object", "members": { "t": { "type": "table", "min_rows": 0, "max_rows": 100,
                "row": { "type": "object", "members": { "v": { "type": "int", "min": 0, "max": 9 } } } } } },
            "rules": [{ "id": "r", "message": "m", "require": { "all": { "table": "t", "as": "a", "where":
                { "all": { "table": "t", "as": "b", "where": { "all": { "table": "t", "as": "c", "where": true } } } } } } }],
            "exports": []
        });
        let module = Module::load(&serde_json::to_vec(&package).unwrap()).unwrap();
        let rows: Vec<_> = (0..100).map(|_| serde_json::json!({ "v": 0 })).collect();
        let doc = serde_json::to_vec(&serde_json::json!({ "t": rows })).unwrap();
        assert_eq!(module.check(&doc, 10_000).err(), Some(EngineError::Budget));
        assert!(
            module.check(&doc, 10_000_000).unwrap().is_ok(),
            "same work fits a larger budget"
        );
    }

    #[test]
    fn ids_and_versions_follow_the_contract() {
        for ok in ["a", "example.demo-panel", "0x"] {
            assert!(valid_module_id(ok), "{ok}");
        }
        for bad in ["", "A", ".a", "a_b", &"a".repeat(65)] {
            assert!(!valid_module_id(bad), "{bad}");
        }
        for ok in ["0.0.0", "1.2.3", "65535.0.10"] {
            assert!(valid_version(ok), "{ok}");
        }
        for bad in ["1.2", "1.2.3.4", "01.2.3", "1.2.65536", "1.-2.3", "1.2.x"] {
            assert!(!valid_version(bad), "{bad}");
        }
    }
}
