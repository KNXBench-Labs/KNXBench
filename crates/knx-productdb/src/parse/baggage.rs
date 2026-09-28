//! Bounded parser for `Baggages.xml`, the manufacturer's baggage index.
//!
//! PDB-10 (ADR-0042) types each `Baggage` declaration: `@Id`, `@Name`,
//! `@TargetPath`, `@InstallOnImport` and `FileInfo/@TimeInfo`/`@Version`,
//! all kept as raw lexemes. The Project Schema only says each baggage is an
//! external file, so none of these is interpreted here. Anything else inside
//! a declaration, or outside the `KNX/ManufacturerData/Manufacturer/Baggages`
//! chain, is reported as an unknown construct, not dropped.

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::report::{UnknownCollector, UnknownConstruct};
use crate::xml::{attrs, local_name};
use crate::ProductDbError;

const MAX_DEPTH: usize = 128;
const MAX_DECLARATIONS: u64 = 100_000;
const CHAIN: [&str; 4] = ["KNX", "ManufacturerData", "Manufacturer", "Baggages"];
const BAGGAGE_XPATH: &str = "/KNX/ManufacturerData/Manufacturer/Baggages/Baggage";
const FILE_INFO_XPATH: &str = "/KNX/ManufacturerData/Manufacturer/Baggages/Baggage/FileInfo";

/// One `Baggage` declaration in document order. Every field is the raw
/// attribute value; `None` means the attribute was absent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BaggageDeclaration {
    pub id: Option<String>,
    pub name: Option<String>,
    pub target_path: Option<String>,
    pub install_on_import: Option<String>,
    pub time_info: Option<String>,
    pub file_version: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BaggageIndexIngest {
    pub declarations: Vec<BaggageDeclaration>,
    /// `Manufacturer` elements on the spine, and the last one's `@RefId`.
    /// Resolution uses the index's directory; these let a caller say when
    /// the document disagrees with it.
    pub manufacturers: u32,
    pub manufacturer_ref: Option<String>,
    pub unknown: Vec<UnknownConstruct>,
}

fn xml_error(source_path: &str, cause: impl Into<String>) -> ProductDbError {
    ProductDbError::Xml {
        source_path: source_path.into(),
        cause: cause.into(),
    }
}

/// Reads the known attributes of `element` into the declaration and reports
/// every other (non-namespace-declaration) attribute at `xpath`.
fn take_attrs(
    source_path: &str,
    element: &BytesStart,
    xpath: &str,
    known: &[&str],
    unknown: &mut UnknownCollector,
) -> Result<crate::xml::Attrs, ProductDbError> {
    let values = attrs(element, source_path)?;
    for raw in values.names() {
        if raw == "xmlns" || raw.starts_with("xmlns:") {
            continue;
        }
        // Only an unprefixed attribute is the known one; `x:Name` is a
        // foreign-namespace attribute that happens to share a local name.
        if !known.contains(&raw) {
            unknown.attribute(xpath, raw, values.evidence_value(raw).unwrap_or_default());
        }
    }
    Ok(values)
}

pub(crate) fn parse_baggage_index(
    source_path: &str,
    bytes: &[u8],
) -> Result<BaggageIndexIngest, ProductDbError> {
    let mut reader = Reader::from_reader(bytes);
    let mut buf = Vec::new();
    let mut parents = Vec::<String>::new();
    let mut declarations = Vec::new();
    let mut unknown = UnknownCollector::default();
    // `Some` while inside a declaration; its `FileInfo` was seen already.
    let mut current: Option<(BaggageDeclaration, bool)> = None;
    let mut manufacturers = 0u32;
    let mut manufacturer_ref: Option<String> = None;

    loop {
        buf.clear();
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|error| xml_error(source_path, error.to_string()))?;
        let is_start = matches!(&event, Event::Start(_));
        match event {
            Event::Start(element) | Event::Empty(element) => {
                let name = local_name(&element);
                let depth = parents.len();
                if depth == CHAIN.len() && parents == CHAIN && name == "Baggage" {
                    if declarations.len() as u64 >= MAX_DECLARATIONS {
                        return Err(xml_error(source_path, "baggage declaration limit exceeded"));
                    }
                    let values = take_attrs(
                        source_path,
                        &element,
                        BAGGAGE_XPATH,
                        &["Id", "Name", "TargetPath", "InstallOnImport"],
                        &mut unknown,
                    )?;
                    let declaration = BaggageDeclaration {
                        id: values.get("Id").map(str::to_string),
                        name: values.get("Name").map(str::to_string),
                        target_path: values.get("TargetPath").map(str::to_string),
                        install_on_import: values.get("InstallOnImport").map(str::to_string),
                        ..Default::default()
                    };
                    if is_start {
                        current = Some((declaration, false));
                    } else {
                        declarations.push(declaration);
                    }
                } else if depth == CHAIN.len() + 1
                    && name == "FileInfo"
                    && matches!(current, Some((_, false)))
                {
                    let values = take_attrs(
                        source_path,
                        &element,
                        FILE_INFO_XPATH,
                        &["TimeInfo", "Version"],
                        &mut unknown,
                    )?;
                    if let Some((declaration, seen)) = current.as_mut() {
                        declaration.time_info = values.get("TimeInfo").map(str::to_string);
                        declaration.file_version = values.get("Version").map(str::to_string);
                        *seen = true;
                    }
                } else if depth < CHAIN.len()
                    && name == CHAIN[depth]
                    && parents[..] == CHAIN[..depth]
                {
                    // The spine. `KNX/@CreatedBy`/`@ToolVersion` are the
                    // document-envelope gap every parser shares
                    // (KNOWN_LIMITATIONS §7) and `Manufacturer/@RefId` is
                    // checked against the index's directory; anything else
                    // on the spine is reported like any unknown attribute.
                    let known: &[&str] = match depth {
                        0 => &["CreatedBy", "ToolVersion"],
                        2 => &["RefId"],
                        _ => &[],
                    };
                    let mut xpath = String::new();
                    for part in parents.iter().chain(std::iter::once(&name)) {
                        xpath.push('/');
                        xpath.push_str(part);
                    }
                    let values = take_attrs(source_path, &element, &xpath, known, &mut unknown)?;
                    if depth == 2 {
                        manufacturers += 1;
                        manufacturer_ref = values.get("RefId").map(str::to_string);
                    }
                } else {
                    // Outside the known chain, a second `FileInfo`, or any
                    // child of `FileInfo`: named at its parent's path.
                    unknown.element(&format!("/{}", parents.join("/")), &name);
                }
                if is_start {
                    parents.push(name);
                    if parents.len() > MAX_DEPTH {
                        return Err(xml_error(
                            source_path,
                            "baggage index nesting limit exceeded",
                        ));
                    }
                }
            }
            Event::End(_) => {
                parents.pop();
                if parents.len() == CHAIN.len() {
                    if let Some((declaration, _)) = current.take() {
                        declarations.push(declaration);
                    }
                }
            }
            Event::Text(text) if !parents.is_empty() => {
                let text = text.into_inner();
                // Whitespace between elements is formatting; anything else is
                // content no field models.
                if !text.trim_matches([' ', '\t', '\r', '\n']).is_empty() {
                    unknown.element(&format!("/{}", parents.join("/")), "#text");
                }
            }
            Event::CData(_) if !parents.is_empty() => {
                unknown.element(&format!("/{}", parents.join("/")), "#text");
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(BaggageIndexIngest {
        declarations,
        manufacturers,
        manufacturer_ref,
        unknown: unknown.into_vec(),
    })
}

/// Why an index's declarations cannot be bound to the manufacturer
/// directory it sits in: the document names another manufacturer, several,
/// or none. `None` when `Manufacturer/@RefId` is exactly that directory.
pub(crate) fn manufacturer_mismatch(
    index_path: &str,
    index: &BaggageIndexIngest,
) -> Option<&'static str> {
    let directory = index_path.strip_suffix("/Baggages.xml")?;
    match (index.manufacturers, index.manufacturer_ref.as_deref()) {
        (1, Some(reference)) if reference == directory => None,
        (1, Some(_)) => Some("Manufacturer RefId differs from the index's directory"),
        (1, None) => Some("Manufacturer has no RefId"),
        _ => Some("index does not declare exactly one Manufacturer"),
    }
}

/// The archive path a declaration names: `<M-XXXX>/Baggages/<TargetPath>/<Name>`
/// next to its index. `Err` carries why it cannot name one. Nothing is
/// normalized: a `..`, `.`, empty or backslashed component is refused
/// rather than resolved, and matching against members is exact.
pub(crate) fn declared_member_path(
    index_path: &str,
    declaration: &BaggageDeclaration,
) -> Result<String, &'static str> {
    let manufacturer = index_path
        .strip_suffix("/Baggages.xml")
        .filter(|dir| !dir.is_empty() && !dir.contains('/'))
        .ok_or("index is not directly inside a manufacturer directory")?;
    let name = declaration
        .name
        .as_deref()
        .ok_or("declaration has no Name")?;
    let target = declaration
        .target_path
        .as_deref()
        .ok_or("declaration has no TargetPath")?;
    let is_unsafe = |part: &str| matches!(part, "" | "." | "..") || part.contains('\\');
    if is_unsafe(name) || name.contains('/') {
        return Err("Name is not a single safe path component");
    }
    let mut path = format!("{manufacturer}/Baggages");
    if !target.is_empty() {
        if target.split('/').any(is_unsafe) {
            return Err("TargetPath has an empty, dot or backslashed component");
        }
        path.push('/');
        path.push_str(target);
    }
    path.push('/');
    path.push_str(name);
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INDEX: &str = "M-0001/Baggages.xml";

    fn declaration(target: &str, name: &str) -> BaggageDeclaration {
        BaggageDeclaration {
            target_path: Some(target.into()),
            name: Some(name.into()),
            ..Default::default()
        }
    }

    #[test]
    fn declarations_keep_raw_lexemes_and_report_the_rest() {
        let xml = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Baggages>
<Baggage Id="M-0001_BG-a" Name="a.png" TargetPath="Img" InstallOnImport="0" Extra="x"><FileInfo TimeInfo="2020-01-01T00:00:00Z" Version="7" Odd="y"><Deep/></FileInfo><FileInfo/><Stray/></Baggage>
<Baggage Id="M-0001_BG-b" Name="b.bin" TargetPath=""/>
</Baggages><Surprise/></Manufacturer></ManufacturerData></KNX>"#;
        let index = parse_baggage_index(INDEX, xml).unwrap();
        assert_eq!(index.declarations.len(), 2);
        let a = &index.declarations[0];
        assert_eq!(a.id.as_deref(), Some("M-0001_BG-a"));
        assert_eq!(a.install_on_import.as_deref(), Some("0"));
        assert_eq!(a.time_info.as_deref(), Some("2020-01-01T00:00:00Z"));
        assert_eq!(a.file_version.as_deref(), Some("7"));
        assert_eq!(index.declarations[1].install_on_import, None);
        let mut seen: Vec<_> = index
            .unknown
            .iter()
            .map(|u| {
                (
                    u.xpath.as_str(),
                    u.kind.as_str(),
                    u.name.as_str(),
                    u.occurrences,
                )
            })
            .collect();
        seen.sort();
        assert_eq!(
            seen,
            [
                (
                    "/KNX/ManufacturerData/Manufacturer",
                    "Element",
                    "Surprise",
                    1
                ),
                (BAGGAGE_XPATH, "Attribute", "Extra", 1),
                (BAGGAGE_XPATH, "Element", "FileInfo", 1),
                (BAGGAGE_XPATH, "Element", "Stray", 1),
                (FILE_INFO_XPATH, "Attribute", "Odd", 1),
                (FILE_INFO_XPATH, "Element", "Deep", 1),
            ]
        );
    }

    #[test]
    fn spine_attributes_and_text_are_reported_not_dropped() {
        let xml = br#"<KNX xmlns="http://knx.org/xml/project/20" CreatedBy="t" ToolVersion="1" Extra="e"><ManufacturerData Odd="o"><Manufacturer RefId="M-0001" Also="a"><Baggages Odd="o">
<Baggage Id="i" Name="a.png" TargetPath="">loose<![CDATA[x]]></Baggage>
</Baggages></Manufacturer></ManufacturerData></KNX>"#;
        let index = parse_baggage_index(INDEX, xml).unwrap();
        assert_eq!(
            (index.manufacturers, index.manufacturer_ref.as_deref()),
            (1, Some("M-0001"))
        );
        let mut seen: Vec<_> = index
            .unknown
            .iter()
            .map(|u| {
                (
                    u.xpath.as_str(),
                    u.kind.as_str(),
                    u.name.as_str(),
                    u.occurrences,
                )
            })
            .collect();
        seen.sort();
        assert_eq!(
            seen,
            [
                ("/KNX", "Attribute", "Extra", 1),
                ("/KNX/ManufacturerData", "Attribute", "Odd", 1),
                ("/KNX/ManufacturerData/Manufacturer", "Attribute", "Also", 1),
                (
                    "/KNX/ManufacturerData/Manufacturer/Baggages",
                    "Attribute",
                    "Odd",
                    1
                ),
                (BAGGAGE_XPATH, "Element", "#text", 2),
            ]
        );
    }

    #[test]
    fn an_index_is_bound_only_to_the_manufacturer_it_sits_under() {
        let index = |manufacturers: &str| {
            let xml = format!(r#"<KNX><ManufacturerData>{manufacturers}</ManufacturerData></KNX>"#);
            parse_baggage_index(INDEX, xml.as_bytes()).unwrap()
        };
        let one = |r: &str| format!(r#"<Manufacturer {r}><Baggages/></Manufacturer>"#);
        assert_eq!(
            manufacturer_mismatch(INDEX, &index(&one(r#"RefId="M-0001""#))),
            None
        );
        assert!(manufacturer_mismatch(INDEX, &index(&one(r#"RefId="M-0002""#))).is_some());
        assert!(manufacturer_mismatch(INDEX, &index(&one(""))).is_some());
        assert!(manufacturer_mismatch(INDEX, &index("")).is_some());
        let two = one(r#"RefId="M-0001""#).repeat(2);
        assert!(manufacturer_mismatch(INDEX, &index(&two)).is_some());
    }

    #[test]
    fn member_paths_are_exact_and_refuse_unsafe_components() {
        assert_eq!(
            declared_member_path(INDEX, &declaration("A/B", "c.png")),
            Ok("M-0001/Baggages/A/B/c.png".into())
        );
        assert_eq!(
            declared_member_path(INDEX, &declaration("", "c.png")),
            Ok("M-0001/Baggages/c.png".into())
        );
        for (target, name) in [
            ("../x", "c"),
            ("A//B", "c"),
            ("A/", "c"),
            ("./A", "c"),
            ("A\\B", "c"),
            ("A", ".."),
            ("A", "b/c"),
            ("A", ""),
        ] {
            assert!(
                declared_member_path(INDEX, &declaration(target, name)).is_err(),
                "{target:?}/{name:?} must not resolve"
            );
        }
        assert!(declared_member_path("Baggages.xml", &declaration("", "c")).is_err());
        assert!(declared_member_path(INDEX, &BaggageDeclaration::default()).is_err());
    }
}
