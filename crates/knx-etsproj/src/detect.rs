//! Stage 2: detecting the ETS schema version.
//!
//! The version is read from the namespace bound to the root `KNX` QName,
//! never guessed from a filename or the ZIP entry layout. Every
//! ETS-written schema namespace observed so far is exactly
//! `http://knx.org/xml/project/<integer>`. A trailing integer alone does
//! not give a foreign namespace KNX semantics.

use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};

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
    /// Set when `knx_master.xml`'s own root namespace disagrees with the
    /// project part's (`0.xml`). IMPORT_EXPORT §3 notes that `xknxproject`
    /// reads the version from `knx_master.xml` instead of the project part;
    /// both reference projects agree. `None` alone does not prove agreement:
    /// an absent master or unreadable root provides no comparison.
    pub namespace_disagreement: Option<SchemaVersion>,
    /// A present master's root metadata could not be interpreted. The project
    /// schema remains authoritative and the master bytes remain opaque. The
    /// report projects a payload-free boundary, not this error's source values.
    /// Container read failures are fatal instead: bytes must be retainable.
    pub master_metadata_error: Option<DetectError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectError {
    Container(ContainerError),
    NoDefaultNamespace {
        entry: String,
    },
    UnparsableNamespace {
        entry: String,
        namespace: String,
    },
    UnsupportedNamespace {
        entry: String,
        namespace: String,
    },
    UnexpectedRoot {
        entry: String,
        name: String,
    },
    NamespaceMismatch {
        entry: String,
        expected: String,
        actual: String,
    },
    MalformedRoot {
        entry: String,
        cause: String,
    },
}

impl std::fmt::Display for DetectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectError::Container(e) => write!(f, "{e}"),
            DetectError::NoDefaultNamespace { entry } => {
                write!(f, "{entry}: root element has no bound XML namespace")
            }
            DetectError::UnparsableNamespace { entry, namespace } => write!(
                f,
                "{entry}: root namespace {namespace:?} has no trailing integer version"
            ),
            DetectError::UnsupportedNamespace { entry, namespace } => write!(
                f,
                "{entry}: {namespace:?} is not a canonical KNX project namespace"
            ),
            DetectError::UnexpectedRoot { entry, name } => {
                write!(f, "{entry}: expected a KNX root element, found {name:?}")
            }
            DetectError::NamespaceMismatch {
                entry,
                expected,
                actual,
            } => write!(
                f,
                "{entry}: root namespace {actual:?} differs from topology namespace {expected:?}"
            ),
            DetectError::MalformedRoot { entry, cause } => {
                write!(f, "{entry}: malformed XML root: {cause}")
            }
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

/// Reads `xml`'s root element and extracts its schema version, bound
/// namespace, and `CreatedBy`/`ToolVersion` attributes. `entry_name` is only
/// used to identify the source in error messages.
fn version_from_root(xml: &[u8], entry_name: &str) -> Result<RootInfo, DetectError> {
    // Read the original bytes, as the topology/metadata parsers do. Invalid
    // encoding must not become replacement characters in detection facts.
    let mut reader = Reader::from_reader(xml);

    loop {
        match reader.read_event() {
            Ok(Event::Start(start)) | Ok(Event::Empty(start)) => {
                let mut attributes = Vec::new();
                for attribute in start.attributes() {
                    let attribute = attribute.map_err(|error| DetectError::MalformedRoot {
                        entry: entry_name.to_string(),
                        cause: error.to_string(),
                    })?;
                    let value = attribute
                        .normalized_value(XmlVersion::Implicit1_0)
                        .map_err(|error| DetectError::MalformedRoot {
                            entry: entry_name.to_string(),
                            cause: error.to_string(),
                        })?;
                    let key = attribute.key.as_ref();
                    if key == "xmlns" || key.starts_with("xmlns:") {
                        // Namespaces in XML 1.0 sections 2.3/3: compare the
                        // normalized XML value, not its entity-escaped spelling.
                        const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";
                        const XMLNS_NAMESPACE: &str = "http://www.w3.org/2000/xmlns/";
                        let prefix = key.strip_prefix("xmlns:");
                        if prefix.is_some_and(|p| p.is_empty() || p == "xmlns")
                            || (prefix == Some("xml") && value != XML_NAMESPACE)
                            || (prefix != Some("xml") && value == XML_NAMESPACE)
                            || value == XMLNS_NAMESPACE
                            || (prefix.is_some() && value.is_empty())
                        {
                            return Err(DetectError::MalformedRoot {
                                entry: entry_name.to_string(),
                                cause: format!("invalid XML namespace binding {key:?}"),
                            });
                        }
                    }
                    attributes.push((key.to_string(), value.into_owned()));
                }
                let attribute_value = |name: &str| {
                    attributes
                        .iter()
                        .find(|(key, _)| key == name)
                        .map(|(_, value)| value.clone())
                };
                let root_name = start.name().as_ref().to_string();
                let (prefix, local_name) = root_name
                    .split_once(':')
                    .map_or((None, root_name.as_str()), |(prefix, name)| {
                        (Some(prefix), name)
                    });
                if local_name != "KNX" {
                    return Err(DetectError::UnexpectedRoot {
                        entry: entry_name.to_string(),
                        name: root_name,
                    });
                }
                let namespace_attribute = prefix
                    .map(|prefix| format!("xmlns:{prefix}"))
                    .unwrap_or_else(|| "xmlns".to_string());
                let namespace = attribute_value(namespace_attribute.as_str()).ok_or_else(|| {
                    DetectError::NoDefaultNamespace {
                        entry: entry_name.to_string(),
                    }
                })?;
                let version = namespace
                    .rsplit('/')
                    .next()
                    .and_then(|segment| segment.parse::<u32>().ok())
                    .ok_or_else(|| DetectError::UnparsableNamespace {
                        entry: entry_name.to_string(),
                        namespace: namespace.clone(),
                    })?;
                if namespace != format!("http://knx.org/xml/project/{version}") {
                    return Err(DetectError::UnsupportedNamespace {
                        entry: entry_name.to_string(),
                        namespace,
                    });
                }
                let created_by = attribute_value("CreatedBy");
                let tool_version = attribute_value("ToolVersion");
                return Ok(RootInfo {
                    version: SchemaVersion(version),
                    namespace,
                    created_by,
                    tool_version,
                });
            }
            Ok(Event::Eof) => {
                return Err(DetectError::NoDefaultNamespace {
                    entry: entry_name.to_string(),
                });
            }
            Err(error) => {
                return Err(DetectError::MalformedRoot {
                    entry: entry_name.to_string(),
                    cause: error.to_string(),
                });
            }
            _ => {}
        }
    }
}

/// Project metadata is parsed with the topology's known table. Do not apply
/// that table to a document declaring a different schema or root identity.
pub(crate) fn require_project_namespace(
    xml: &[u8],
    entry_name: &str,
    expected: &str,
) -> Result<(), DetectError> {
    let root = version_from_root(xml, entry_name)?;
    if root.namespace != expected {
        return Err(DetectError::NamespaceMismatch {
            entry: entry_name.to_string(),
            expected: expected.to_string(),
            actual: root.namespace,
        });
    }
    Ok(())
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

    let (namespace_disagreement, master_metadata_error) =
        if container.find("knx_master.xml").is_some() {
            match version_from_master(container) {
                Ok(version) => (
                    Some(version).filter(|version| *version != root.version),
                    None,
                ),
                Err(DetectError::Container(error)) => return Err(DetectError::Container(error)),
                Err(error) => (None, Some(error)),
            }
        } else {
            (None, None)
        };

    Ok(Detected {
        version: root.version,
        namespace: root.namespace,
        created_by: root.created_by,
        tool_version: root.tool_version,
        namespace_disagreement,
        master_metadata_error,
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
    #[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
    fn the_ets4_reference_project_is_schema_eleven() {
        assert!(
            crate::testutil::corpus_available(),
            "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
        );
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        let d = detect(&mut c).unwrap();
        assert_eq!(d.version, SchemaVersion(11));
        assert_eq!(d.namespace, "http://knx.org/xml/project/11");
        assert_eq!(d.created_by.as_deref(), Some("ETS4"));
        assert_eq!(d.tool_version.as_deref(), Some("ETS 4.1.8 (Build 3614)"));
        assert_eq!(d.namespace_disagreement, None);
    }

    #[test]
    #[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
    fn the_ets6_reference_project_is_schema_twenty_three() {
        assert!(
            crate::testutil::corpus_available(),
            "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
        );
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
