//! Stage 2: detecting the ETS schema version.
//!
//! The version is read from the root `<KNX xmlns="...">` element's default
//! namespace, never guessed from a filename or the ZIP entry layout. Every
//! ETS-written schema namespace observed so far ends in `/<integer>`
//! (`.../project/11`, `.../project/23`); that trailing integer is the schema
//! version.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::{Container, ContainerError};

/// An ETS project schema version, e.g. `SchemaVersion(11)` for ETS4's
/// `http://knx.org/xml/project/11`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaVersion(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detected {
    pub version: SchemaVersion,
    pub namespace: String,
    /// `KNX/@CreatedBy`.
    pub created_by: Option<String>,
    /// `KNX/@ToolVersion`.
    pub tool_version: Option<String>,
    /// Set when `knx_master.xml`'s own default namespace disagrees with the
    /// project part's (`0.xml`). IMPORT_EXPORT §3 notes that `xknxproject`
    /// reads the version from `knx_master.xml` instead of the project part;
    /// both reference projects agree, so this stays `None` today and exists
    /// for a future sample that doesn't. A failure to read or parse
    /// `knx_master.xml` is not itself fatal to detection — it is reported as
    /// no disagreement rather than propagated, since the project part's own
    /// version is what matters for parsing.
    pub namespace_disagreement: Option<SchemaVersion>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectError {
    Container(ContainerError),
    NoDefaultNamespace { entry: String },
    UnparsableNamespace { entry: String, namespace: String },
}

impl std::fmt::Display for DetectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectError::Container(e) => write!(f, "{e}"),
            DetectError::NoDefaultNamespace { entry } => {
                write!(f, "{entry}: root element has no default (xmlns) namespace")
            }
            DetectError::UnparsableNamespace { entry, namespace } => write!(
                f,
                "{entry}: default namespace {namespace:?} has no trailing integer version"
            ),
        }
    }
}

impl std::error::Error for DetectError {}

impl From<ContainerError> for DetectError {
    fn from(e: ContainerError) -> Self {
        DetectError::Container(e)
    }
}

/// What the root element of an ETS-written XML document carries.
struct RootInfo {
    version: SchemaVersion,
    namespace: String,
    created_by: Option<String>,
    tool_version: Option<String>,
}

/// Reads `xml`'s root element and extracts its schema version, default
/// namespace, and `CreatedBy`/`ToolVersion` attributes. `entry_name` is only
/// used to identify the source in error messages.
fn version_from_root(xml: &[u8], entry_name: &str) -> Result<RootInfo, DetectError> {
    // Lossy: a non-UTF-8 byte in an ETS-written file would mean the root
    // element itself can't be trusted, which is exactly what
    // NoDefaultNamespace already reports.
    let text = String::from_utf8_lossy(xml);
    let mut reader = Reader::from_str(&text);

    loop {
        match reader.read_event() {
            Ok(Event::Start(start)) | Ok(Event::Empty(start)) => {
                let namespace = start
                    .try_get_attribute("xmlns")
                    .ok()
                    .flatten()
                    .map(|a| a.value.into_owned())
                    .ok_or_else(|| DetectError::NoDefaultNamespace {
                        entry: entry_name.to_string(),
                    })?;
                let version = namespace
                    .rsplit('/')
                    .next()
                    .and_then(|segment| segment.parse::<u32>().ok())
                    .ok_or_else(|| DetectError::UnparsableNamespace {
                        entry: entry_name.to_string(),
                        namespace: namespace.clone(),
                    })?;
                let created_by = start
                    .try_get_attribute("CreatedBy")
                    .ok()
                    .flatten()
                    .map(|a| a.value.into_owned());
                let tool_version = start
                    .try_get_attribute("ToolVersion")
                    .ok()
                    .flatten()
                    .map(|a| a.value.into_owned());
                return Ok(RootInfo {
                    version: SchemaVersion(version),
                    namespace,
                    created_by,
                    tool_version,
                });
            }
            Ok(Event::Eof) | Err(_) => {
                return Err(DetectError::NoDefaultNamespace {
                    entry: entry_name.to_string(),
                });
            }
            _ => {}
        }
    }
}

/// The schema version `knx_master.xml`'s own root element declares.
fn version_from_master(container: &mut Container) -> Result<SchemaVersion, DetectError> {
    let xml = container.read("knx_master.xml")?;
    Ok(version_from_root(&xml, "knx_master.xml")?.version)
}

/// Detects the schema version, reading only from the project part's `0.xml`
/// (never assumed, never inferred from a filename).
pub fn detect(container: &mut Container) -> Result<Detected, DetectError> {
    let part = container.project_part()?.to_string();
    let entry = format!("{part}/0.xml");
    let xml = container.read(&entry)?;
    detect_from_bytes(&xml, &entry, container)
}

/// Detects the schema version from an already-read `0.xml`. A caller that
/// also needs the parsed document (`import_knxproj_bytes`) reads `0.xml`
/// once and passes the bytes here, rather than `detect` reading the same
/// entry from the container a second time — `Container::read` has no
/// cache, so a second call re-decompresses the entire entry, and `0.xml`
/// is typically the largest file in the container.
pub fn detect_from_bytes(
    xml: &[u8],
    entry_name: &str,
    container: &mut Container,
) -> Result<Detected, DetectError> {
    let root = version_from_root(xml, entry_name)?;

    let namespace_disagreement = version_from_master(container)
        .ok()
        .filter(|master_version| *master_version != root.version);

    Ok(Detected {
        version: root.version,
        namespace: root.namespace,
        created_by: root.created_by,
        tool_version: root.tool_version,
        namespace_disagreement,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_ets4_bytes() -> Vec<u8> {
        std::fs::read(knx_testsupport::reference_ets4_path())
            .expect("reference ETS4 project lives under OriginalData/ (gitignored); see knx_testsupport::corpus_available")
    }

    fn reference_ets6_bytes() -> Vec<u8> {
        std::fs::read(knx_testsupport::reference_ets6_path())
            .expect("reference ETS6 project lives under OriginalData/ (gitignored); see knx_testsupport::corpus_available")
    }

    #[test]
    fn the_ets4_reference_project_is_schema_eleven() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        let d = detect(&mut c).unwrap();
        assert_eq!(d.version, SchemaVersion(11));
        assert_eq!(d.namespace, "http://knx.org/xml/project/11");
        assert_eq!(d.created_by.as_deref(), Some("ETS4"));
        assert_eq!(d.tool_version.as_deref(), Some("ETS 4.1.8 (Build 3614)"));
        assert_eq!(d.namespace_disagreement, None);
    }

    #[test]
    fn the_ets6_reference_project_is_schema_twenty_three() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let mut c = Container::open(reference_ets6_bytes()).unwrap();
        assert_eq!(detect(&mut c).unwrap().version, SchemaVersion(23));
    }

    #[test]
    fn a_namespace_without_a_trailing_integer_is_an_error_not_a_guess() {
        let xml = br#"<KNX xmlns="http://knx.org/xml/project/eleven"/>"#;
        assert!(matches!(
            version_from_root(xml, "0.xml"),
            Err(DetectError::UnparsableNamespace { .. })
        ));
    }

    #[test]
    fn a_document_with_no_default_namespace_is_an_error() {
        let xml = br#"<KNX CreatedBy="ETS4"/>"#;
        assert!(matches!(
            version_from_root(xml, "0.xml"),
            Err(DetectError::NoDefaultNamespace { .. })
        ));
    }
}
