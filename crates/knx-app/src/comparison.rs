//! Loads native or ETS project inputs for read-only semantic comparison.

use std::path::{Path, PathBuf};

use knx_core::Project;
use knx_etsproj::ImportReport;

/// One normalized project plus import diagnostics when the source was a raw
/// ETS archive. Native stores have no import report.
pub struct ComparisonInput {
    pub project: Project,
    pub import_report: Option<ImportReport>,
}

#[derive(Debug)]
pub enum ComparisonInputError {
    NotFound(PathBuf),
    UnsupportedFormat(PathBuf),
    NativeStore {
        path: PathBuf,
        detail: String,
    },
    EtsProject {
        path: PathBuf,
        source: knx_etsproj::ImportFailure,
    },
}

impl std::fmt::Display for ComparisonInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(path) => write!(f, "comparison input not found: {}", path.display()),
            Self::UnsupportedFormat(path) => write!(
                f,
                "unsupported comparison input format for {}; expected .knxdb or .knxproj",
                path.display()
            ),
            Self::NativeStore { path, detail } => {
                write!(
                    f,
                    "failed to load native project {}: {detail}",
                    path.display()
                )
            }
            Self::EtsProject { path, source } => {
                write!(
                    f,
                    "failed to import ETS project {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ComparisonInputError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::EtsProject { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Detects the supported input format from its explicit extension, validates
/// existence before opening (SQLite would otherwise create a typo'd path),
/// and returns the normalized project used by `knx-diff` callers.
///
/// Opening a native store may run the normal forward-only store migration.
/// Importing `.knxproj` is in-memory and retains the full `ImportReport` so a
/// caller cannot silently hide losses or unsupported data from the user.
pub fn load_comparison_input(path: &Path) -> Result<ComparisonInput, ComparisonInputError> {
    if !path.exists() {
        return Err(ComparisonInputError::NotFound(path.to_path_buf()));
    }

    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("knxdb") => {
            let connection = knx_store::open_and_migrate(path).map_err(|error| {
                ComparisonInputError::NativeStore {
                    path: path.to_path_buf(),
                    detail: error.to_string(),
                }
            })?;
            let project = knx_store::load_project(&connection).map_err(|error| {
                ComparisonInputError::NativeStore {
                    path: path.to_path_buf(),
                    detail: error.to_string(),
                }
            })?;
            Ok(ComparisonInput {
                project,
                import_report: None,
            })
        }
        Some("knxproj") => {
            let outcome = knx_etsproj::import_knxproj(path).map_err(|source| {
                ComparisonInputError::EtsProject {
                    path: path.to_path_buf(),
                    source,
                }
            })?;
            Ok(ComparisonInput {
                project: outcome.project,
                import_report: Some(outcome.report),
            })
        }
        _ => Err(ComparisonInputError::UnsupportedFormat(path.to_path_buf())),
    }
}
