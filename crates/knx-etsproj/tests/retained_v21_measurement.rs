//! Measures, against the installed corpus, which source attributes survive a schema-≥21 import but not the export back out (`KNOWN_LIMITATIONS.md` §34).

use std::collections::{BTreeMap, BTreeSet};

use knx_etsproj::{export::export_knxproj, import_knxproj_bytes, opaque::OpaqueEntry, Container};
use knx_testsupport::{corpus_available, reference_ets4_path, reference_ets6_path, reference_kv_schema21_path};
use quick_xml::events::Event;
use quick_xml::Reader;

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
                .unescape_value()
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
/// documents. Nothing here writes to disk; the export stays in memory.
fn round_trip(path: &std::path::Path) -> (Vec<u8>, Vec<u8>) {
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
    let part = source.project_part().expect("corpus has a project part").to_string();
    let source_xml = source.read(&format!("{part}/0.xml")).expect("corpus has a 0.xml");

    let mut written = Container::open(exported.bytes).expect("export opens");
    let part = written.project_part().expect("export has a project part").to_string();
    let export_xml = written.read(&format!("{part}/0.xml")).expect("export has a 0.xml");
    (source_xml, export_xml)
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
        let Some(written) = exp.get(key) else { continue };
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

#[test]
fn measure_kv_schema_21() {
    if !corpus_available() {
        return;
    }
    let (src, exp) = round_trip(&reference_kv_schema21_path());
    panic!("KV losses: {:#?}\ndiverged: {:#?}", losses(&src, &exp), divergences(&src, &exp));
}

#[test]
fn measure_ets6_schema_23() {
    if !corpus_available() {
        return;
    }
    let (src, exp) = round_trip(&reference_ets6_path());
    panic!("ETS6 losses: {:#?}\ndiverged: {:#?}", losses(&src, &exp), divergences(&src, &exp));
}

#[test]
fn zz_dump() {
    if !corpus_available() {
        return;
    }
    let (s, e) = round_trip(&reference_ets6_path());
    std::fs::write("/tmp/claude-1000/-mnt-daten-i-Sourcecode-KNXBench/7b0667bc-b774-4338-8b9a-e55d0e678e45/scratchpad/ets6-src.xml", s).unwrap();
    std::fs::write("/tmp/claude-1000/-mnt-daten-i-Sourcecode-KNXBench/7b0667bc-b774-4338-8b9a-e55d0e678e45/scratchpad/ets6-exp.xml", e).unwrap();
    let (s, e) = round_trip(&reference_kv_schema21_path());
    std::fs::write("/tmp/claude-1000/-mnt-daten-i-Sourcecode-KNXBench/7b0667bc-b774-4338-8b9a-e55d0e678e45/scratchpad/kv-src.xml", s).unwrap();
    std::fs::write("/tmp/claude-1000/-mnt-daten-i-Sourcecode-KNXBench/7b0667bc-b774-4338-8b9a-e55d0e678e45/scratchpad/kv-exp.xml", e).unwrap();
}

#[test]
fn measure_ets4_schema_11() {
    if !corpus_available() {
        return;
    }
    let (src, exp) = round_trip(&reference_ets4_path());
    panic!("ETS4 losses: {:#?}\ndiverged: {:#?}", losses(&src, &exp), divergences(&src, &exp));
}
