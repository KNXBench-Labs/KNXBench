//! Counts a private schema-23 project's object ids through the production RefId rules.
//!
//! KNOWN_LIMITATIONS §125 measured ETS 6's device-local communication-object
//! id form against one schema-23 project and named its lifting condition: a
//! second, independently produced schema-23 project whose `GroupObjectTree`
//! ids are counted the same way. §1 asks for an independent module-using
//! schema-23 sample. This test performs that count on a privately supplied
//! project that is never committed.
//!
//! The count deliberately bypasses the importer. It reads the archive by
//! index, takes every object id from `GroupObjectTree`/`Node`
//! `@GroupObjectInstances` and `ComObjectInstanceRef/@RefId`, and runs each
//! one through the production rules [`device_local_com_object_number`] and
//! [`module_com_object_ref`]. It then resolves the id against the device's own
//! application program (`Hardware2ProgramRefId` → `Hardware.xml` →
//! `ApplicationProgramRef`) and compares the referenced `ComObject/@Number`
//! with the number the rule returned. Whole-project import acceptance is a
//! separate concern and is not claimed here.
//!
//! Privacy: the input path comes only from `KNXBENCH_PRIVATE_SCHEMA23_PROJECT`.
//! Every assertion is a boolean with a generic message; no id, name, address,
//! count or digest of the private file is printed or compared by value. The
//! ignore reason intentionally does not mention `OriginalData`, so the
//! repository corpus runner does not select it without its variable.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;

use knx_etsproj::values::{device_local_com_object_number, module_com_object_ref};
use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};

const PROJECT_VAR: &str = "KNXBENCH_PRIVATE_SCHEMA23_PROJECT";
const SCHEMA_23: &str = "http://knx.org/xml/project/23";
const MEMBER_LIMIT: u64 = 256 * 1024 * 1024;

/// One element start (or empty element) with its depth and attributes.
struct Element {
    depth: usize,
    name: String,
    attributes: BTreeMap<String, String>,
}

/// A flat, document-ordered element list; `End` events only lower the depth.
fn elements(bytes: &[u8]) -> Vec<Element> {
    let mut reader = Reader::from_reader(bytes);
    let mut out = Vec::new();
    let mut depth = 0usize;
    loop {
        let event = reader.read_event();
        let (start, empty) = match event {
            Ok(Event::Start(start)) => (start, false),
            Ok(Event::Empty(start)) => (start, true),
            Ok(Event::End(_)) => {
                depth = depth.saturating_sub(1);
                continue;
            }
            Ok(Event::Eof) => break,
            Ok(_) => continue,
            Err(_) => panic!("a selected XML member of the private project is not well formed"),
        };
        let name = start.name().local_name().as_ref().to_string();
        let mut attributes = BTreeMap::new();
        for attribute in start.attributes() {
            let attribute = attribute.expect("a private XML attribute is malformed");
            let key = attribute.key.as_ref().to_string();
            let value = attribute
                .normalized_value(XmlVersion::Implicit1_0)
                .expect("a private XML attribute value is malformed")
                .into_owned();
            attributes.insert(key, value);
        }
        out.push(Element {
            depth,
            name,
            attributes,
        });
        if !empty {
            depth += 1;
        }
    }
    out
}

/// What an archive member is to this count, decided by its decoded name.
#[derive(PartialEq)]
enum Member {
    Installation,
    Hardware,
    Program,
}

fn member_kind(name: &str) -> Option<Member> {
    let (top, rest) = name.split_once('/')?;
    if top.len() == 6 && top.starts_with("P-") && !rest.contains('/') {
        let stem = rest.strip_suffix(".xml")?;
        return (!stem.is_empty() && stem.bytes().all(|b| b.is_ascii_digit()))
            .then_some(Member::Installation);
    }
    if top.len() == 6 && top.starts_with("M-") && !rest.contains('/') {
        if rest == "Hardware.xml" {
            return Some(Member::Hardware);
        }
        if rest.ends_with(".xml") && rest.contains("_A-") {
            return Some(Member::Program);
        }
    }
    None
}

/// Reads the selected members **by index**: the count must not depend on
/// name-based lookup inside the archive.
fn read_members(path: &str) -> Vec<(Member, Vec<u8>)> {
    let file = File::open(path).expect("the configured private project cannot be opened");
    let mut archive = zip::ZipArchive::new(file).expect("the private project is not a ZIP archive");
    let mut out = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .expect("a private archive entry is unreadable");
        let Some(kind) = member_kind(entry.name()) else {
            continue;
        };
        assert!(
            entry.size() <= MEMBER_LIMIT,
            "a selected private member exceeds the limit"
        );
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .expect("a selected private member does not decompress");
        out.push((kind, bytes));
    }
    out
}

/// Per-program communication objects: `ComObject/@Id → @Number` and
/// `ComObjectRef/@Id → @RefId`, both fully qualified as written.
#[derive(Default)]
struct ProgramObjects {
    numbers: BTreeMap<String, String>,
    references: BTreeMap<String, String>,
}

impl ProgramObjects {
    /// The `ComObject/@Number` behind a fully qualified `ComObjectRef/@Id`.
    fn number_behind(&self, reference: &str) -> Option<&str> {
        let object = self.references.get(reference)?;
        self.numbers.get(object).map(String::as_str)
    }
}

/// The project-side facts one device contributes.
#[derive(Default)]
struct Device {
    hardware_to_program: String,
    has_address: bool,
    object_ids: BTreeSet<String>,
    links: Vec<String>,
    module_refs: Vec<String>,
    repeat_counter_above_one: bool,
}

#[derive(Default)]
struct Census {
    schema_23_roots: usize,
    other_roots: usize,
    devices: Vec<Device>,
    group_address_suffixes: BTreeSet<String>,
    node_style_tree: bool,
    root_style_tree: bool,
}

fn scan_installation(bytes: &[u8], census: &mut Census) {
    let mut device: Option<(usize, Device)> = None;
    for element in elements(bytes) {
        if let Some((depth, _)) = &device {
            if element.depth <= *depth {
                census
                    .devices
                    .push(device.take().expect("device in progress").1);
            }
        }
        let attr = |key: &str| element.attributes.get(key).map(String::as_str);
        match element.name.as_str() {
            "KNX" if element.depth == 0 => {
                if attr("xmlns") == Some(SCHEMA_23) {
                    census.schema_23_roots += 1;
                } else {
                    census.other_roots += 1;
                }
            }
            "DeviceInstance" => {
                device = Some((
                    element.depth,
                    Device {
                        hardware_to_program: attr("Hardware2ProgramRefId")
                            .unwrap_or("")
                            .to_string(),
                        has_address: attr("Address").is_some_and(|a| !a.is_empty()),
                        ..Device::default()
                    },
                ));
            }
            "GroupAddress" => {
                if let Some((_, suffix)) = attr("Id").and_then(|id| id.rsplit_once('_')) {
                    census.group_address_suffixes.insert(suffix.to_string());
                }
            }
            _ => {}
        }
        let Some((_, current)) = device.as_mut() else {
            continue;
        };
        match element.name.as_str() {
            "GroupObjectTree" | "Node" => {
                if let Some(ids) = attr("GroupObjectInstances") {
                    if element.name == "Node" {
                        census.node_style_tree = true;
                    } else {
                        census.root_style_tree = true;
                    }
                    current
                        .object_ids
                        .extend(ids.split_whitespace().map(str::to_string));
                }
            }
            "ComObjectInstanceRef" => {
                if let Some(id) = attr("RefId") {
                    current.object_ids.insert(id.to_string());
                }
                if let Some(links) = attr("Links") {
                    current
                        .links
                        .extend(links.split_whitespace().map(str::to_string));
                }
            }
            "ModuleInstance" => {
                current
                    .module_refs
                    .push(attr("RefId").unwrap_or("").to_string());
                let counters = attr("RepeatIndex").unwrap_or("");
                current.repeat_counter_above_one |= counters
                    .split_whitespace()
                    .filter_map(|pair| pair.split_once('x'))
                    .any(|(_, counter)| counter.parse::<u32>().is_ok_and(|n| n > 1));
            }
            _ => {}
        }
    }
    if let Some((_, current)) = device {
        census.devices.push(current);
    }
}

fn scan_hardware(bytes: &[u8], programs_by_hardware: &mut BTreeMap<String, Vec<String>>) {
    let mut current: Option<(usize, String)> = None;
    for element in elements(bytes) {
        if let Some((depth, _)) = &current {
            if element.depth <= *depth {
                current = None;
            }
        }
        if element.name == "Hardware2Program" {
            let id = element.attributes.get("Id").cloned().unwrap_or_default();
            programs_by_hardware.entry(id.clone()).or_default();
            current = Some((element.depth, id));
        } else if element.name == "ApplicationProgramRef" {
            if let (Some((_, id)), Some(program)) = (&current, element.attributes.get("RefId")) {
                programs_by_hardware
                    .entry(id.clone())
                    .or_default()
                    .push(program.clone());
            }
        }
    }
}

fn scan_program(bytes: &[u8], programs: &mut BTreeMap<String, ProgramObjects>) {
    let mut program = String::new();
    for element in elements(bytes) {
        let attr = |key: &str| element.attributes.get(key).cloned();
        match element.name.as_str() {
            "ApplicationProgram" => {
                program = attr("Id").unwrap_or_default();
                programs.entry(program.clone()).or_default();
            }
            "ComObject" => {
                if let (Some(id), Some(number)) = (attr("Id"), attr("Number")) {
                    programs
                        .entry(program.clone())
                        .or_default()
                        .numbers
                        .insert(id, number);
                }
            }
            "ComObjectRef" => {
                if let (Some(id), Some(target)) = (attr("Id"), attr("RefId")) {
                    programs
                        .entry(program.clone())
                        .or_default()
                        .references
                        .insert(id, target);
                }
            }
            _ => {}
        }
    }
}

/// Boolean outcome of the id count; nothing in it identifies the input.
#[derive(Default)]
struct Outcome {
    device_local_ids: bool,
    module_ids: bool,
    every_device_program_resolves: bool,
    every_id_has_a_known_shape: bool,
    every_device_local_id_resolves_with_equal_number: bool,
    every_module_id_resolves_with_equal_number: bool,
    every_link_names_a_group_address: bool,
    some_links: bool,
}

fn count(
    census: &Census,
    programs: &BTreeMap<String, ProgramObjects>,
    hw: &BTreeMap<String, Vec<String>>,
) -> Outcome {
    let mut outcome = Outcome {
        every_device_program_resolves: true,
        every_id_has_a_known_shape: true,
        every_device_local_id_resolves_with_equal_number: true,
        every_module_id_resolves_with_equal_number: true,
        every_link_names_a_group_address: true,
        ..Outcome::default()
    };
    for device in &census.devices {
        let resolved = hw
            .get(&device.hardware_to_program)
            .filter(|refs| refs.len() == 1)
            .and_then(|refs| {
                programs
                    .get(&refs[0])
                    .map(|objects| (refs[0].as_str(), objects))
            });
        let Some((program, objects)) = resolved else {
            outcome.every_device_program_resolves = false;
            continue;
        };
        for id in &device.object_ids {
            if let Ok(number) = device_local_com_object_number(id) {
                outcome.device_local_ids = true;
                let behind = objects.number_behind(&format!("{program}_{id}"));
                if behind != Some(number.to_string().as_str()) {
                    outcome.every_device_local_id_resolves_with_equal_number = false;
                }
            } else if let Ok((short, number)) = module_com_object_ref(id) {
                outcome.module_ids = true;
                let behind = objects.number_behind(&format!("{program}_{short}"));
                if behind != Some(number.to_string().as_str()) {
                    outcome.every_module_id_resolves_with_equal_number = false;
                }
            } else {
                outcome.every_id_has_a_known_shape = false;
            }
        }
        for link in &device.links {
            outcome.some_links = true;
            if !census.group_address_suffixes.contains(link) {
                outcome.every_link_names_a_group_address = false;
            }
        }
    }
    outcome
}

#[test]
#[ignore = "requires a private schema-23 project; set KNXBENCH_PRIVATE_SCHEMA23_PROJECT and run with --ignored"]
fn a_second_schema_23_project_resolves_every_object_id_through_the_production_rules() {
    let path = std::env::var(PROJECT_VAR)
        .unwrap_or_else(|_| panic!("SKIP is not a pass: set {PROJECT_VAR} to the private project"));
    assert!(
        std::path::Path::new(&path).is_file(),
        "{PROJECT_VAR} does not name a readable file"
    );

    let mut census = Census::default();
    let mut programs = BTreeMap::new();
    let mut programs_by_hardware = BTreeMap::new();
    for (kind, bytes) in read_members(&path) {
        match kind {
            Member::Installation => scan_installation(&bytes, &mut census),
            Member::Hardware => scan_hardware(&bytes, &mut programs_by_hardware),
            Member::Program => scan_program(&bytes, &mut programs),
        }
    }

    // The sample is what §1/§125 ask for: schema 23, with devices and modules.
    assert!(
        census.schema_23_roots > 0,
        "no schema-23 installation file found"
    );
    assert_eq!(
        census.other_roots, 0,
        "an installation file uses another namespace"
    );
    assert!(
        !census.devices.is_empty(),
        "the private project has no devices"
    );

    let outcome = count(&census, &programs, &programs_by_hardware);
    assert!(
        outcome.every_device_program_resolves,
        "a device's program does not resolve uniquely"
    );
    assert!(
        outcome.every_id_has_a_known_shape,
        "an object id matches neither production rule"
    );
    assert!(
        outcome.device_local_ids,
        "no device-local object id was counted"
    );
    assert!(
        outcome.every_device_local_id_resolves_with_equal_number,
        "a device-local id does not name a ComObjectRef with the same object number"
    );
    assert!(outcome.module_ids, "no module object id was counted");
    assert!(
        outcome.every_module_id_resolves_with_equal_number,
        "a module object id does not name a ComObjectRef with the same object number"
    );
    assert!(outcome.some_links, "no group link was counted");
    assert!(
        outcome.every_link_names_a_group_address,
        "a group link names no group address"
    );

    // Shapes this sample contributes to §1, §52 and §68 (presence only).
    assert!(
        census.node_style_tree,
        "no Node-style object tree in the sample"
    );
    assert!(
        census.root_style_tree,
        "no root-style object tree in the sample"
    );
    assert!(
        census.devices.iter().any(|d| !d.has_address),
        "no device without an individual address in the sample"
    );
    assert!(
        census.devices.iter().any(|d| {
            let mut refs = d.module_refs.clone();
            refs.sort();
            refs.windows(2).any(|w| w[0] == w[1])
        }),
        "no device instantiates one module more than once"
    );
    assert!(
        census.devices.iter().any(|d| d.repeat_counter_above_one),
        "no RepeatIndex counter above one in the sample"
    );
}

#[test]
fn member_kinds_select_only_installation_hardware_and_program_files() {
    assert!(member_kind("P-TEST/0.xml") == Some(Member::Installation));
    assert!(member_kind("P-TEST/12.xml") == Some(Member::Installation));
    assert!(member_kind("P-TEST/project.xml").is_none());
    assert!(member_kind("P-TEST/AddinData/x/0.xml").is_none());
    assert!(member_kind("M-TEST/Hardware.xml") == Some(Member::Hardware));
    assert!(member_kind("M-TEST/M-TEST_A-0001-01-0001.xml") == Some(Member::Program));
    assert!(member_kind("M-TEST/Catalog.xml").is_none());
    assert!(member_kind("M-TEST/Baggages/a_A-b.xml").is_none());
}

#[test]
fn the_count_rejects_a_mismatched_object_number_and_an_unknown_link() {
    let installation = br#"<KNX xmlns="http://knx.org/xml/project/23"><Project><Installations><Installation>
        <Topology><Area><Line><Segment>
          <DeviceInstance Id="D1" Address="1" Hardware2ProgramRefId="H1">
            <ComObjectInstanceRefs><ComObjectInstanceRef RefId="O-2_R-3" Links="GA-9"/></ComObjectInstanceRefs>
            <GroupObjectTree GroupObjectInstances="O-1_R-2 MD-1_M-1_MI-1_O-2-5_R-7"/>
          </DeviceInstance>
        </Segment></Line></Area></Topology>
        <GroupAddresses><GroupRanges><GroupRange><GroupAddress Id="P-TEST-0_GA-1"/></GroupRange></GroupRanges></GroupAddresses>
        </Installation></Installations></Project></KNX>"#;
    let hardware = br#"<KNX><Hardware2Program Id="H1"><ApplicationProgramRef RefId="A1"/></Hardware2Program></KNX>"#;
    let program = br#"<KNX><ApplicationProgram Id="A1">
        <ComObject Id="A1_O-1" Number="1"/><ComObjectRef Id="A1_O-1_R-2" RefId="A1_O-1"/>
        <ComObject Id="A1_O-2" Number="99"/><ComObjectRef Id="A1_O-2_R-3" RefId="A1_O-2"/>
        <ComObject Id="A1_MD-1_O-2-5" Number="5"/><ComObjectRef Id="A1_MD-1_O-2-5_R-7" RefId="A1_MD-1_O-2-5"/>
        </ApplicationProgram></KNX>"#;
    let mut census = Census::default();
    let mut programs = BTreeMap::new();
    let mut hw = BTreeMap::new();
    scan_installation(installation, &mut census);
    scan_hardware(hardware, &mut hw);
    scan_program(program, &mut programs);
    let outcome = count(&census, &programs, &hw);
    assert!(outcome.every_device_program_resolves);
    assert!(outcome.every_id_has_a_known_shape);
    assert!(outcome.device_local_ids && outcome.module_ids);
    assert!(outcome.every_module_id_resolves_with_equal_number);
    // `O-2_R-3` claims object number 2, but its ComObject says 99.
    assert!(!outcome.every_device_local_id_resolves_with_equal_number);
    // `GA-9` names no group address of the installation.
    assert!(!outcome.every_link_names_a_group_address);
}
