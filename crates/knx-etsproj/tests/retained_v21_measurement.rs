//! Counts, over the installed corpus, what an import-export round trip loses or changes.

use std::collections::{BTreeMap, BTreeSet};

use knx_etsproj::export::{export_knxproj, ExportWarning};
use knx_etsproj::{import_knxproj_bytes, opaque::OpaqueEntry, Container};
use knx_testsupport::{
    corpus_available, reference_ets4_path, reference_ets6_path, reference_kv_schema21_path,
};
use quick_xml::events::Event;
use quick_xml::Reader;

/// One import-export round trip over a corpus project.
struct RoundTrip {
    source_xml: Vec<u8>,
    export_xml: Vec<u8>,
    warnings: Vec<knx_etsproj::export::ExportWarning>,
}

/// `Segment` carries an `Id` in an ETS file but has no identity in the
/// domain model, so the exporter cannot reproduce it. Keying a segment by
/// its own `Id` would therefore compare a source segment against nothing.
/// It is keyed by its enclosing `Line` instead.
const NO_OWN_IDENT: &[&str] = &["Segment"];

/// Every element of an installation document, keyed by a path built from
/// each ancestor's own `Id`/`RefId`, mapped to its attributes. Two documents
/// describing the same installation produce the same key for the same
/// element, whatever order they were written in.
fn elements(xml: &[u8]) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut reader = Reader::from_reader(xml);
    let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut stack: Vec<String> = Vec::new();
    let mut buf = Vec::new();
    loop {
        let ev = reader.read_event_into(&mut buf).expect("well-formed XML");
        let (start, is_empty) = match &ev {
            Event::Start(s) => (Some(s), false),
            Event::Empty(s) => (Some(s), true),
            Event::End(_) => {
                stack.pop();
                buf.clear();
                continue;
            }
            Event::Eof => break,
            _ => {
                buf.clear();
                continue;
            }
        };
        let s = start.expect("matched above");
        let name = s.name().local_name().as_ref().to_string();
        let mut attrs: BTreeMap<String, String> = BTreeMap::new();
        for a in s.attributes() {
            let a = a.expect("well-formed attributes");
            let key = a.key.local_name().as_ref().to_string();
            let value = a
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .expect("well-formed attribute values")
                .to_string();
            attrs.insert(key, value);
        }
        let ident = if NO_OWN_IDENT.contains(&name.as_str()) {
            None
        } else {
            attrs.get("Id").or_else(|| attrs.get("RefId"))
        };
        let key = match ident {
            Some(id) => format!("{}/{name}[{id}]", stack.join("")),
            None => format!("{}/{name}", stack.join("")),
        };
        out.insert(key.clone(), attrs);
        if !is_empty {
            let own = key[stack.join("").len()..].to_string();
            stack.push(own);
        }
        buf.clear();
    }
    out
}

/// Imports a `.knxproj` and exports it again, returning both installation
/// documents and the export's warnings. Nothing here writes to disk; the
/// export stays in memory.
fn round_trip(path: &std::path::Path) -> RoundTrip {
    let bytes = std::fs::read(path).expect("corpus file is readable");
    let outcome =
        import_knxproj_bytes(bytes, path.to_string_lossy().as_ref()).expect("corpus file imports");
    let entries: Vec<OpaqueEntry> = outcome
        .opaque
        .iter()
        .cloned()
        .chain(outcome.manufacturer.iter().map(|m| OpaqueEntry {
            source_path: m.source_path.clone(),
            xpath: String::new(),
            kind: m.kind,
            name: String::new(),
            bytes: m.bytes.clone(),
            sha256: m.sha256.clone(),
        }))
        .collect();
    let exported = export_knxproj(&outcome.project, &entries).expect("export succeeds");

    let source_bytes = std::fs::read(path).expect("corpus file is readable");
    let mut source = Container::open(source_bytes).expect("corpus file opens");
    let part = source
        .project_part()
        .expect("corpus has a project part")
        .to_string();
    let source_xml = source
        .read(&format!("{part}/0.xml"))
        .expect("corpus has a 0.xml");

    let mut written = Container::open(exported.bytes.clone()).expect("export opens");
    let part = written
        .project_part()
        .expect("export has a project part")
        .to_string();
    let export_xml = written
        .read(&format!("{part}/0.xml"))
        .expect("export has a 0.xml");
    RoundTrip {
        source_xml,
        export_xml,
        warnings: exported.warnings,
    }
}

/// `(element name, attribute name) -> how many element instances lost it`.
fn losses(source_xml: &[u8], export_xml: &[u8]) -> BTreeMap<(String, String), usize> {
    let src = elements(source_xml);
    let exp = elements(export_xml);
    let mut out: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (key, attrs) in &src {
        let element = key
            .rsplit('/')
            .next()
            .expect("split always yields one part")
            .split('[')
            .next()
            .expect("split always yields one part")
            .to_string();
        let Some(written) = exp.get(key) else {
            *out.entry((element, "<whole element>".to_string()))
                .or_default() += 1;
            continue;
        };
        for name in attrs.keys() {
            if !written.contains_key(name) {
                *out.entry((element.clone(), name.clone())).or_default() += 1;
            }
        }
    }
    out
}

/// Attributes present in both documents but with a different value — the
/// corruption class §34 exists to prevent. Serialization differences
/// (whitespace escaping, timestamp precision) are not identity errors and
/// are listed by attribute name only, never by value: corpus values include
/// real hardware serial numbers.
fn divergences(source_xml: &[u8], export_xml: &[u8]) -> BTreeSet<(String, String)> {
    let src = elements(source_xml);
    let exp = elements(export_xml);
    let mut out = BTreeSet::new();
    for (key, attrs) in &src {
        let Some(written) = exp.get(key) else {
            continue;
        };
        let element = key
            .rsplit('/')
            .next()
            .expect("split always yields one part")
            .split('[')
            .next()
            .expect("split always yields one part")
            .to_string();
        for (name, value) in attrs {
            if written.get(name).is_some_and(|w| w != value) {
                out.insert((element.clone(), name.clone()));
            }
        }
    }
    out
}

/// Attributes the export writes onto an element the source did not have
/// them on, `(element, attribute) -> instances`. The blind spot
/// [`losses`] and [`divergences`] share: both walk the *source* document,
/// so an attribute that only exists in the export is invisible to them —
/// and a retained value landing on an element that never carried it looks
/// exactly like that. `<whole element>` means the export invented an
/// element the source has no counterpart for.
fn additions(source_xml: &[u8], export_xml: &[u8]) -> BTreeMap<(String, String), usize> {
    let src = elements(source_xml);
    let exp = elements(export_xml);
    let mut out: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (key, attrs) in &exp {
        let element = element_of(key);
        let Some(original) = src.get(key) else {
            *out.entry((element, "<whole element>".to_string()))
                .or_default() += 1;
            continue;
        };
        for name in attrs.keys() {
            if !original.contains_key(name) {
                *out.entry((element.clone(), name.clone())).or_default() += 1;
            }
        }
    }
    out
}

/// `"DeviceInstance"` from `".../Line[P-1_L-2]/DeviceInstance[P-1_DI-3]"`.
fn element_of(key: &str) -> String {
    key.rsplit('/')
        .next()
        .expect("split always yields one part")
        .split('[')
        .next()
        .expect("split always yields one part")
        .to_string()
}

/// The expected residue of one corpus project: attributes that go in and
/// do not come back out, `(element, attribute) -> instances`.
fn expect_losses(rt: &RoundTrip, expected: &[(&str, &str, usize)], project: &str) {
    let measured = losses(&rt.source_xml, &rt.export_xml);
    let want: BTreeMap<(String, String), usize> = expected
        .iter()
        .map(|(e, a, n)| ((e.to_string(), a.to_string()), *n))
        .collect();
    assert_eq!(
        measured, want,
        "{project}: the set of attributes lost on export changed. If the \
         change is an improvement, shrink the expectation; if it is not, \
         this is the regression the test exists for."
    );
}

/// Attributes allowed to come back holding a different value. `CreatedBy`
/// and `ToolVersion` are deliberately rewritten: this application is not
/// ETS and says so. The two device timestamps are reformatted by the
/// round trip through `time`, same instant, fewer fractional digits.
const ALLOWED_DIVERGENCE: &[(&str, &str)] = &[
    ("KNX", "CreatedBy"),
    ("KNX", "ToolVersion"),
    ("DeviceInstance", "LastDownload"),
    ("DeviceInstance", "LastModified"),
];

/// Attributes the export is allowed to write onto an element that did not
/// carry them, each with the reason it is allowed. An allow-list without
/// reasons is a way to not look.
///
/// None of these is a retained value on a foreign element — every one is
/// the model writing a field it always writes. Measured identical at this
/// branch's base commit, so none of them arrived with §34's fix.
const ALLOWED_ADDITION: &[(&str, &str, &str)] = &[
    (
        "GroupAddress",
        "Central",
        "an opt-in flag the model carries as a plain bool and the writer \
         always spells; absent in the source means the same false it writes",
    ),
    (
        "GroupAddress",
        "Unfiltered",
        "same as Central: an always-written bool whose absent form and \
         written form mean the same thing",
    ),
    (
        "DeviceInstance",
        "Name",
        "the model has no absent name, only an empty one, so a nameless \
         source device comes back with Name=\"\" — the same empty string \
         Installation/@Name loses in the other direction",
    ),
    (
        "DeviceInstance",
        "ApplicationProgramLoaded",
        "download state: absent means not loaded, which is what the model \
         holds and the writer spells out (crates/knx-etsproj/tests/download_state.rs)",
    ),
    (
        "DeviceInstance",
        "CommunicationPartLoaded",
        "download state, as above",
    ),
    (
        "DeviceInstance",
        "IndividualAddressLoaded",
        "download state, as above",
    ),
    (
        "DeviceInstance",
        "MediumConfigLoaded",
        "download state, as above",
    ),
    (
        "DeviceInstance",
        "ParametersLoaded",
        "download state, as above",
    ),
    (
        "ComObjectInstanceRef",
        "<whole element>",
        "ADR-0014: the device's GroupObjectTree is the authoritative id \
         list, so an id listed there with no instance override of its own \
         still becomes a communication object and is written back as a bare \
         <ComObjectInstanceRef RefId=…/>. The RefId comes from the same \
         device's own document, so nothing is invented — the element is \
         spelled out where the source left it implied",
    ),
    (
        "ComObjectInstanceRef",
        "IsActive",
        "NOT benign, allow-listed to keep the rest of this instrument \
         honest: ETS spells @IsActive on all 907 schema-11 refs and on none \
         of the 717 schema-≥21 ones measured here, and the importer reads \
         absent as false, so the export states inactive where the source \
         stated nothing. Whether the schema's default is true (making this \
         a misread of every modern project's objects) is not settled by the \
         corpus — no ≥21 file spells it either way. Raised in task 03's fix \
         round; it belongs to whoever owns the com-object model, not to §34",
    ),
];

fn expect_no_additions(rt: &RoundTrip, project: &str) {
    let allowed: BTreeSet<(String, String)> = ALLOWED_ADDITION
        .iter()
        .map(|(e, a, _)| (e.to_string(), a.to_string()))
        .collect();
    let unexpected: BTreeMap<_, _> = additions(&rt.source_xml, &rt.export_xml)
        .into_iter()
        .filter(|((element, attribute), _)| {
            !allowed.contains(&(element.clone(), attribute.clone()))
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "{project}: the export wrote an attribute onto an element the source \
         did not have it on: {unexpected:?}. A retained value landing on the \
         wrong element is the failure §34 exists to prevent; a model default \
         that is always written belongs in ALLOWED_ADDITION, with a reason."
    );
}

fn expect_no_corruption(rt: &RoundTrip, project: &str) {
    let allowed: BTreeSet<(String, String)> = ALLOWED_DIVERGENCE
        .iter()
        .map(|(e, a)| (e.to_string(), a.to_string()))
        .collect();
    let measured = divergences(&rt.source_xml, &rt.export_xml);
    let unexpected: BTreeSet<_> = measured.difference(&allowed).cloned().collect();
    assert!(
        unexpected.is_empty(),
        "{project}: an exported attribute holds a value the source did not \
         have: {unexpected:?}. §34's rule is that a wrong value is worse \
         than a missing one."
    );
}

/// The export's own account of what it dropped, checked against what the
/// documents say. §34's table claims these three projects lose no retained
/// attribute and no retained element; the warnings are where that claim
/// would first break, so they are asserted rather than merely collected.
fn expect_no_retained_residue(rt: &RoundTrip, project: &str) {
    let residue: Vec<_> = rt
        .warnings
        .iter()
        .filter(|w| {
            matches!(
                w,
                ExportWarning::RetainedAttributeNotExported { .. }
                    | ExportWarning::RetainedElementNotExported { .. }
            )
        })
        .collect();
    assert!(
        residue.is_empty(),
        "{project}: the export reported retained data it could not write \
         back: {residue:?}. §34 records these three projects as losing \
         none, so either the writer regressed or the table needs redoing."
    );
}

#[test]
fn schema_21_round_trip_keeps_every_attribute_but_the_empty_ones() {
    if !corpus_available() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let rt = round_trip(&reference_kv_schema21_path());
    // `Installation/@Name` and `@DefaultLine` are the empty string in this
    // project, and the domain model has no way to tell an empty value from
    // an absent one, so the writer omits both. Nothing a user typed is
    // lost; a pair of empty strings is.
    expect_losses(
        &rt,
        &[
            ("Installation", "DefaultLine", 1),
            ("Installation", "Name", 1),
        ],
        "KV schema 21",
    );
    for attr in ["Name", "DefaultLine"] {
        let value = elements(&rt.source_xml)
            .iter()
            .find(|(k, _)| k.ends_with("/Installation") || k.contains("/Installation["))
            .and_then(|(_, a)| a.get(attr).cloned())
            .unwrap_or_default();
        assert!(
            value.is_empty(),
            "Installation/@{attr} is no longer empty in the corpus, so \
             dropping it is no longer harmless"
        );
    }
    expect_no_corruption(&rt, "KV schema 21");
    expect_no_additions(&rt, "KV schema 21");
    expect_no_retained_residue(&rt, "KV schema 21");
}

#[test]
fn schema_23_round_trip_keeps_every_attribute_but_the_empty_one() {
    if !corpus_available() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let rt = round_trip(&reference_ets6_path());
    expect_losses(&rt, &[("Installation", "Name", 1)], "ETS 6.3.0 schema 23");
    expect_no_corruption(&rt, "ETS 6.3.0 schema 23");
    expect_no_additions(&rt, "ETS 6.3.0 schema 23");
    expect_no_retained_residue(&rt, "ETS 6.3.0 schema 23");
}

#[test]
fn schema_11_round_trip_keeps_every_attribute() {
    if !corpus_available() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let rt = round_trip(&reference_ets4_path());
    expect_losses(&rt, &[], "ETS4 schema 11");
    expect_no_corruption(&rt, "ETS4 schema 11");
    expect_no_additions(&rt, "ETS4 schema 11");
    expect_no_retained_residue(&rt, "ETS4 schema 11");
}

/// The sharp case from §34: a hardware serial number belongs to exactly one
/// device, and writing one device's serial onto another would be silent
/// corruption of the worst kind. Every device that had one gets its own
/// back. Values are compared, never printed.
#[test]
fn every_device_keeps_its_own_serial_number() {
    if !corpus_available() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    for (path, project) in [
        (reference_kv_schema21_path(), "KV schema 21"),
        (reference_ets6_path(), "ETS 6.3.0 schema 23"),
        (reference_ets4_path(), "ETS4 schema 11"),
    ] {
        let rt = round_trip(&path);
        let src = elements(&rt.source_xml);
        let exp = elements(&rt.export_xml);
        let mut checked = 0usize;
        for (key, attrs) in &src {
            if !key.contains("/DeviceInstance[") {
                continue;
            }
            let Some(serial) = attrs.get("SerialNumber") else {
                continue;
            };
            checked += 1;
            let written = exp
                .get(key)
                .and_then(|a| a.get("SerialNumber"))
                .unwrap_or_else(|| panic!("{project}: a device lost its SerialNumber on export"));
            assert_eq!(
                written, serial,
                "{project}: a device came back with a serial number that is \
                 not its own"
            );
        }
        if project.starts_with("KV") {
            assert!(
                checked >= 4,
                "the KV project is supposed to carry four distinct serial \
                 numbers; found {checked}"
            );
        }
    }
}
