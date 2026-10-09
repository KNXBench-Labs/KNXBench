//! Offline authoring tool, not a production app API or an ETS exporter.
//!
//! Input is the versioned original descriptor from tools/community_demos.py.

use knx_core::*;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    version: String,
    products: Vec<ProductSpec>,
    projects: Vec<ProjectSpec>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductSpec {
    key: String,
    app: String,
    product: String,
    program: String,
    objects: Vec<ObjectSpec>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObjectSpec {
    name: String,
    dpt: String,
    direction: String,
    number: u16,
    channel: u16,
    text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectSpec {
    key: String,
    name: String,
    zones: Vec<ZoneSpec>,
    lines: Vec<LineSpec>,
    parts: Vec<PartSpec>,
    devices: Vec<DeviceSpec>,
    groups: Vec<GroupSpec>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ZoneSpec {
    number: u8,
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LineSpec {
    number: u8,
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PartSpec {
    key: String,
    name: String,
    kind: String,
    parent: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeviceSpec {
    key: String,
    name: String,
    product: String,
    address: [u8; 3],
    part: String,
    links: Vec<LinkSpec>,
    description: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkSpec {
    number: u16,
    groups: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupSpec {
    address: String,
    name: String,
    dpt: String,
    central: bool,
}

type Error = Box<dyn std::error::Error>;

fn authored<T>(value: T, layer: Layer) -> Override<T> {
    Override::Value(Resolved { value, layer })
}

fn source(id: impl Into<String>) -> SourceRef {
    SourceRef {
        path: "community-demos/1.0.0".into(),
        ets_id: id.into(),
    }
}

fn dpt(text: &str) -> Result<DptRef, Error> {
    let (main, sub) = text.split_once('.').ok_or("invalid demo DPT")?;
    Ok(DptRef {
        main: main.parse()?,
        sub: Some(sub.parse()?),
    })
}

fn build_project(
    spec: &ProjectSpec,
    products: &BTreeMap<&str, &ProductSpec>,
) -> Result<Project, Error> {
    let mut project = Project::new(Language("en-US".into()));
    project.info.project_id = format!("KNXB-DEMO-{}", spec.key);
    project.info.name = spec.name.clone();
    project.info.project_number = Some("Fictional offline demo 1.0.0".into());
    project.info.completion = CompletionStatus::Editing;
    // Existing monolithic-program model shape, not provenance of an ETS import.
    project.info.ets_schema_version = 11;
    let mut installation = Installation {
        id: InstallationId(0),
        name: spec.name.clone(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::Editing,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    };
    let area_id = project.ids.next_area_id()?;
    let mut area = Area {
        id: area_id,
        source: source("demo-area-1"),
        name: spec.name.clone(),
        address: 1,
        completion: CompletionStatus::Editing,
        lines: vec![],
    };
    let mut lines = BTreeMap::new();
    for s in &spec.lines {
        IndividualAddress::new(1, s.number, 0)?;
        let id = project.ids.next_line_id()?;
        if lines.insert(s.number, id).is_some() {
            return Err("duplicate demo line".into());
        }
        area.lines.push(id);
        installation.topology.lines.push(Line {
            id,
            source: source(format!("demo-line-{}", s.number)),
            name: s.name.clone(),
            address: s.number,
            medium_ref: "MT-0".into(),
            domain_address: None,
            domain_address_is_checked: None,
            ip_routing_multicast_address: None,
            multicast_ttl: None,
            completion: CompletionStatus::Editing,
            devices: vec![],
        });
    }
    installation.default_line = lines.get(&1).copied();
    installation.topology.areas.push(area);
    let mut parts = BTreeMap::new();
    for s in &spec.parts {
        if parts
            .insert(s.key.as_str(), project.ids.next_building_part_id()?)
            .is_some()
        {
            return Err("duplicate demo building part".into());
        }
    }
    for s in &spec.parts {
        let id = parts[s.key.as_str()];
        let parent = s
            .parent
            .as_deref()
            .map(|p| parts.get(p).copied().ok_or("missing building parent"))
            .transpose()?;
        let kind = match s.kind.as_str() {
            "Building" => BuildingPartType::Building,
            "Floor" => BuildingPartType::Floor,
            "Room" => BuildingPartType::Room,
            "DistributionBoard" => BuildingPartType::DistributionBoard,
            "BuildingPart" => BuildingPartType::BuildingPart,
            _ => return Err("unknown building kind".into()),
        };
        let children = spec
            .parts
            .iter()
            .filter(|p| p.parent.as_deref() == Some(s.key.as_str()))
            .map(|p| parts[p.key.as_str()])
            .collect();
        installation.buildings.push(BuildingPart {
            id,
            source: source(format!("demo-part-{}", s.key)),
            name: s.name.clone(),
            number: None,
            kind,
            default_line: None,
            completion: CompletionStatus::Editing,
            children,
            devices: vec![],
            parent,
        });
    }
    let labels = [
        "Switching",
        "Dimming",
        "Blinds",
        "Room heating",
        "Occupancy",
        "Scenes",
        "Central commands",
    ];
    let mut ranges = BTreeMap::new();
    for (main, name) in labels.into_iter().enumerate() {
        let id = project.ids.next_group_range_id()?;
        let mut children = vec![];
        for zone in &spec.zones {
            let start = GroupAddress::parse(
                &format!("{main}/{}/0", zone.number),
                GroupAddressStyle::ThreeLevel,
            )?;
            let end = GroupAddress::parse(
                &format!("{main}/{}/255", zone.number),
                GroupAddressStyle::ThreeLevel,
            )?;
            let middle = project.ids.next_group_range_id()?;
            if ranges.insert((main as u8, zone.number), middle).is_some() {
                return Err("duplicate demo zone".into());
            }
            children.push(middle);
            installation.group_ranges.push(GroupRange {
                id: middle,
                source: source(format!("demo-range-{main}-{}", zone.number)),
                name: zone.name.clone(),
                start,
                end,
                parent: Some(id),
                children: vec![],
            });
        }
        installation.group_ranges.push(GroupRange {
            id,
            source: source(format!("demo-range-{main}")),
            name: name.into(),
            start: GroupAddress::from_raw((main as u16) << 11),
            end: GroupAddress::from_raw(((main as u16) << 11) + 2047),
            parent: None,
            children,
        });
    }
    let mut groups = BTreeMap::new();
    for s in &spec.groups {
        let address = GroupAddress::parse(&s.address, GroupAddressStyle::ThreeLevel)?;
        let id = project.ids.next_group_address_id()?;
        if groups.insert(s.address.as_str(), id).is_some() {
            return Err("duplicate demo group address".into());
        }
        let key = (
            ((address.raw() >> 11) & 31) as u8,
            ((address.raw() >> 8) & 7) as u8,
        );
        let range = Some(*ranges.get(&key).ok_or("group address has no zone range")?);
        installation.group_addresses.push(GroupAddressEntry {
            id,
            source: source(format!("demo-ga-{}", s.address)),
            name: s.name.clone(),
            address,
            central: s.central,
            unfiltered: false,
            range,
            declared_dpt: authored(dpt(&s.dpt)?, Layer::UserEdit),
        });
    }
    let mut individual_addresses = std::collections::BTreeSet::new();
    let mut device_keys = std::collections::BTreeSet::new();
    for s in &spec.devices {
        if !device_keys.insert(&s.key) {
            return Err("duplicate demo device key".into());
        }
        let product = products
            .get(s.product.as_str())
            .ok_or("missing demo product")?;
        let address = IndividualAddress::new(s.address[0], s.address[1], s.address[2])?;
        if address.area() != 1 || (address.device() == 0 && product.key != "LC") {
            return Err("invalid illustrative coupler/device placement".into());
        }
        if !individual_addresses.insert(address) {
            return Err("duplicate individual address".into());
        }
        let line = *lines.get(&address.line()).ok_or("missing device line")?;
        let part = *parts
            .get(s.part.as_str())
            .ok_or("missing device building part")?;
        let id = project.ids.next_device_id()?;
        let mut com_objects = vec![];
        let mut object_numbers = std::collections::BTreeSet::new();
        for object in &product.objects {
            if !object_numbers.insert(object.number) {
                return Err("duplicate demo object number".into());
            }
            let cid = project.ids.next_com_object_instance_id()?;
            com_objects.push(cid);
            let send = match object.direction.as_str() {
                "Send" => true,
                "Receive" => false,
                _ => return Err("unknown object direction".into()),
            };
            let mut links = vec![];
            for binding in s.links.iter().filter(|b| b.number == object.number) {
                for ga in &binding.groups {
                    let gid = *groups
                        .get(ga.as_str())
                        .ok_or("missing linked group address")?;
                    let link = GroupLink {
                        ga: gid,
                        direction: if send {
                            Direction::Send
                        } else {
                            Direction::Receive
                        },
                    };
                    if links.contains(&link) {
                        return Err("duplicate demo link".into());
                    }
                    links.push(link);
                }
            }
            let dtype = dpt(&object.dpt)?;
            let bits = format_width_bits(dtype.main).ok_or("unknown demo DPT width")?;
            let size = if bits < 8 {
                ObjectSize::Bit(bits as u8)
            } else {
                ObjectSize::Byte((bits / 8) as u16)
            };
            project.devices.insert_com_object(ComObjectInstance { id: cid,
                source: source(format!("{}_O-{}_R-1", product.app, object.number)), device: id, number: object.number,
                text: authored(Text::Literal(object.text.clone()), Layer::Program),
                description: authored(Text::Literal(format!("Fictional channel {} / {}. Unlinked objects are reserve functions, not missing connections.", object.channel, object.name)), Layer::Program),
                dpt: authored(dtype, Layer::Program), flags: ResolvedFlags {
                    read: authored(send, Layer::Program), write: authored(!send, Layer::Program),
                    transmit: authored(send, Layer::Program), update: authored(!send, Layer::Program),
                    communication: authored(true, Layer::Program), read_on_init: authored(false, Layer::Program) },
                size: Some(Resolved { value: size, layer: Layer::Program }), is_active: true, links, module_instance: None });
        }
        if s.links.iter().any(|b| !object_numbers.contains(&b.number)) {
            return Err("link names an unknown communication object".into());
        }
        project.devices.insert(DeviceInstance {
            id,
            source: source(format!("demo-device-{}", s.key)),
            name: s.name.clone(),
            description: Some(s.description.clone()),
            address: Some(address),
            product_ref: product.product.clone(),
            program_ref: product.program.clone(),
            commissioning: CommissioningState {
                completion: CompletionStatus::Editing,
                ..Default::default()
            },
            visibility_calculated: true,
            com_objects,
            binary_data: vec![],
        });
        installation
            .topology
            .lines
            .iter_mut()
            .find(|l| l.id == line)
            .ok_or("missing line")?
            .devices
            .push(id);
        installation
            .buildings
            .iter_mut()
            .find(|p| p.id == part)
            .ok_or("missing part")?
            .devices
            .push(id);
        installation.parameters.push(ParameterInstance {
            id: project.ids.next_parameter_instance_id()?,
            device: id,
            source: source(format!("{}_P-1_R-1", product.app)),
            raw: "2".into(),
        });
    }
    project.installations.push(installation);
    for ga in &project.installations[0].group_addresses {
        if resolve_group_address_type(&project, ga.id).outcome != GroupAddressTypeOutcome::Declared
        {
            return Err(format!("inconsistent DPT at {}", ga.name).into());
        }
    }
    Ok(project)
}

fn read_bundle(input: &Path) -> Result<Bundle, Error> {
    let bundle: Bundle = serde_json::from_slice(&std::fs::read(input.join("projects.json"))?)?;
    if bundle.version != "1.0.0" {
        return Err("unknown demo descriptor version".into());
    }
    let expected = [
        "single-family-home",
        "multi-unit-residential",
        "office-building",
    ];
    if bundle.projects.len() != expected.len()
        || bundle
            .projects
            .iter()
            .zip(expected)
            .any(|(p, key)| p.key != key)
    {
        return Err("unknown, duplicate or reordered demo project key".into());
    }
    let unique_products: std::collections::BTreeSet<_> =
        bundle.products.iter().map(|p| &p.key).collect();
    if unique_products.len() != bundle.products.len() {
        return Err("duplicate demo product key".into());
    }
    Ok(bundle)
}

fn practice_expected(baseline: &Project) -> Result<Project, Error> {
    let (device, com) = baseline
        .devices
        .iter()
        .filter(|d| d.product_ref.ends_with("_P-DEMO-SW4"))
        .find_map(|d| {
            d.com_objects
                .iter()
                .filter_map(|id| baseline.devices.com_object(*id))
                .find(|c| c.number % 4 == 0 && c.links.is_empty())
                .map(|c| (d, c))
        })
        .ok_or("no reserve practice object")?;
    let mut expected = baseline.clone();
    Command::SetDeviceDescription {
        device: device.id,
        description: Some(format!(
            "{} Practice note: reviewed offline.",
            device.description.as_deref().unwrap_or_default()
        )),
    }
    .apply(&mut expected)?;
    let parameter = baseline.installations[0]
        .parameters
        .iter()
        .find(|p| p.device == device.id)
        .ok_or("missing practice parameter")?;
    Command::SetParameterValue {
        id: parameter.id,
        device: device.id,
        ets_id: parameter.source.ets_id.clone(),
        raw: "5".into(),
    }
    .apply(&mut expected)?;
    let ga = expected.ids.next_group_address_id()?;
    let address = GroupAddress::parse("0/0/200", GroupAddressStyle::ThreeLevel)?;
    let range = baseline.installations[0]
        .group_ranges
        .iter()
        .find(|r| r.parent.is_some() && r.start.raw() == 0 && r.end.raw() == 255)
        .ok_or("missing practice range")?
        .id;
    Command::CreateGroupAddress {
        entry: GroupAddressEntry {
            id: ga,
            source: SourceRef {
                path: format!("KB-GA-{}", ga.0),
                ets_id: format!("KB-GA-{}", ga.0),
            },
            name: "Practice light / Switch command".into(),
            address,
            central: false,
            unfiltered: false,
            range: Some(range),
            declared_dpt: Override::Absent,
        },
        installation: None,
    }
    .apply(&mut expected)?;
    Command::LinkComObject {
        com_object: com.id,
        ga,
        direction: Direction::Receive,
    }
    .apply(&mut expected)?;
    Ok(expected)
}

fn verify_practice_model(baseline: &Project, actual: &Project) -> Result<(), Error> {
    let mut expected = practice_expected(baseline)?;
    // Save legitimately stamps project modification time. Every other field,
    // including exact provenance, object order, allocator and loaded flags, is compared.
    expected.info.last_modified = actual.info.last_modified;
    if expected != *actual {
        return Err(
            "practice model differs beyond the four intended edits and save timestamp".into(),
        );
    }
    Ok(())
}

fn main() -> Result<(), Error> {
    let input = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: community_demos SPEC_DIRECTORY")?,
    );
    let bundle = read_bundle(&input)?;
    let products = bundle
        .products
        .iter()
        .map(|p| (p.key.as_str(), p))
        .collect();
    if let Some(practice_dir) = std::env::args_os().nth(2) {
        let directory = PathBuf::from(practice_dir);
        for spec in &bundle.projects {
            let baseline = build_project(spec, &products)?;
            let store = knx_store::open_existing_read_only(
                &directory.join(format!("{}-practice.knxdb", spec.key)),
            )?;
            let actual = knx_store::load_project(&store.conn)?;
            verify_practice_model(&baseline, &actual)?;
            let copy = knx_store::open_and_migrate_in_memory()?;
            knx_store::save_project(&copy, &actual)?;
            if knx_store::load_project(&copy)? != actual {
                return Err("practice native model roundtrip differs".into());
            }
            println!(
                "{}: full practice model equality and read-only save/load verified",
                spec.key
            );
        }
        return Ok(());
    }
    let report_path = input.join("generation-report.json");
    if report_path.exists()
        || bundle
            .projects
            .iter()
            .any(|s| input.join(format!("{}.knxdb", s.key)).exists())
    {
        return Err("refusing to overwrite existing generation output".into());
    }
    let temporary = tempfile::tempdir()?;
    let catalogue = knx_productdb::open_and_migrate(&temporary.path().join("products.sqlite"))?;
    let bytes = std::fs::read(input.join("fictional-demo-devices.knxprod"))?;
    let installed =
        knx_productdb::install_package(&catalogue, "fictional-demo-devices.knxprod", &bytes)?;
    if !installed.conflicts.is_empty() {
        return Err("fictional catalogue has conflicting identities".into());
    }
    for p in &bundle.products {
        let evaluated = knx_productdb::device_evaluation::evaluate_device(
            &catalogue,
            &p.app,
            vec![(format!("{}_P-1_R-1", p.app), "2".into())],
            &[],
        )?;
        if !evaluated.stale.is_empty()
            || !evaluated.findings.is_empty()
            || evaluated.activation.parameter_refs.len() != 1
            || evaluated.activation.com_object_refs.len() != p.objects.len()
        {
            return Err(format!("fictional catalogue does not resolve {}", p.key).into());
        }
    }
    for spec in &bundle.projects {
        let project = build_project(spec, &products)?;
        let path = input.join(format!("{}.knxdb", spec.key));
        if path.exists() {
            return Err("refusing to overwrite a native project".into());
        }
        let conn = knx_store::open_and_migrate(&path)?;
        knx_store::save_project(&conn, &project)?;
        if knx_store::load_project(&conn)? != project {
            return Err("native model roundtrip differs".into());
        }
        println!("{}: {} devices", spec.key, project.devices.iter().count());
    }
    let report = serde_json::json!({
        "native_schema": knx_core::CURRENT_SCHEMA_VERSION,
        "product_schema": knx_productdb::CURRENT_PRODUCTDB_VERSION,
        "product_input_scheme": installed.scheme,
        "catalogue_unknown_occurrences": installed.unknown,
        "catalogue_conflicts": installed.conflicts.len(),
        "programs_evaluated": bundle.products.len(),
        "native_model_roundtrips": bundle.projects.len(),
        "hardware_or_ets_verification": false,
    });
    use std::io::Write;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(report_path)?
        .write_all(serde_json::to_string_pretty(&report)?.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn specifications() -> (tempfile::TempDir, Bundle) {
        let tmp = tempfile::tempdir().unwrap();
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/community_demos.py");
        let result = Command::new("python3")
            .arg(script)
            .arg("specs")
            .arg(tmp.path().join("input"))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let bundle = read_bundle(&tmp.path().join("input")).unwrap();
        (tmp, bundle)
    }

    #[test]
    fn practice_acceptance_rejects_unrelated_persisted_changes() {
        let (_tmp, bundle) = specifications();
        let products = bundle
            .products
            .iter()
            .map(|p| (p.key.as_str(), p))
            .collect();
        let baseline = build_project(&bundle.projects[0], &products).unwrap();
        assert!(verify_practice_model(&baseline, &baseline).is_err());
        let mut completed = practice_expected(&baseline).unwrap();
        assert!(verify_practice_model(&baseline, &completed).is_ok());
        completed.info.name.push_str(" altered");
        assert!(verify_practice_model(&baseline, &completed).is_err());
    }

    #[test]
    fn descriptor_rejects_traversal_and_duplicate_project_keys() {
        let (tmp, _) = specifications();
        let path = tmp.path().join("input/projects.json");
        let original = std::fs::read(&path).unwrap();
        for key in ["../outside", "single-family-home"] {
            let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
            value["projects"][1]["key"] = key.into();
            std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
            assert!(read_bundle(&tmp.path().join("input")).is_err(), "{key}");
        }
    }

    #[test]
    fn malformed_addresses_dangling_links_and_dpt_mismatches_are_refused() {
        let (_tmp, mut bundle) = specifications();
        let products = bundle
            .products
            .iter()
            .map(|p| (p.key.as_str(), p))
            .collect();
        let spec = &mut bundle.projects[0];
        let original = spec.devices[2].address;
        spec.devices[2].address = spec.devices[0].address;
        assert!(build_project(spec, &products)
            .unwrap_err()
            .to_string()
            .contains("duplicate individual"));
        spec.devices[2].address = [16, 1, 2];
        assert!(build_project(spec, &products).is_err());
        spec.devices[2].address = original;
        let old = spec.devices[2].links[0].groups[0].clone();
        spec.devices[2].links[0].groups[0] = "31/7/255".into();
        assert!(build_project(spec, &products)
            .unwrap_err()
            .to_string()
            .contains("missing linked"));
        spec.devices[2].links[0].groups[0] = old;
        spec.groups[0].dpt = "9.001".into();
        assert!(build_project(spec, &products)
            .unwrap_err()
            .to_string()
            .contains("inconsistent DPT"));
    }

    #[test]
    fn fictional_catalogue_resolves_every_program_and_parameter() {
        let (tmp, bundle) = specifications();
        let products =
            knx_productdb::open_and_migrate(&tmp.path().join("products.sqlite")).unwrap();
        let bytes = std::fs::read(tmp.path().join("input/fictional-demo-devices.knxprod")).unwrap();
        let report =
            knx_productdb::install_package(&products, "fictional-demo-devices.knxprod", &bytes)
                .unwrap();
        assert!(report.conflicts.is_empty());
        assert_eq!(report.unknown, 0, "{:?}", report.facts);
        assert_eq!(report.scheme, 11);
        assert_eq!(
            products
                .query_row("SELECT count(*) FROM product", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            bundle.products.len() as i64
        );
        for p in &bundle.products {
            let pid = format!("{}_P-1_R-1", p.app);
            let evaluated = knx_productdb::device_evaluation::evaluate_device(
                &products,
                &p.app,
                vec![(pid.clone(), "2".into())],
                &[],
            )
            .unwrap();
            assert!(evaluated.stale.is_empty(), "{}", p.key);
            assert!(evaluated.findings.is_empty(), "{}", p.key);
            assert_eq!(evaluated.activation.parameter_refs.len(), 1, "{}", p.key);
            assert_eq!(
                evaluated.activation.com_object_refs.len(),
                p.objects.len(),
                "{}",
                p.key
            );
            assert!(evaluated.ref_ids.contains(&pid));
        }
        let retry =
            knx_productdb::install_package(&products, "fictional-demo-devices.knxprod", &bytes)
                .unwrap();
        assert!(retry.skipped);
    }

    #[test]
    fn three_native_projects_keep_the_complete_authored_model() {
        let (tmp, bundle) = specifications();
        let products = bundle
            .products
            .iter()
            .map(|p| (p.key.as_str(), p))
            .collect();
        for spec in &bundle.projects {
            let project = build_project(spec, &products).unwrap();
            assert_eq!(
                project.devices.iter().count(),
                spec.devices.len(),
                "{}",
                spec.key
            );
            assert_eq!(
                project.installations[0].group_addresses.len(),
                spec.groups.len()
            );
            let conn = knx_store::open_and_migrate(&tmp.path().join(format!("{}.knxdb", spec.key)))
                .unwrap();
            knx_store::save_project(&conn, &project).unwrap();
            assert_eq!(
                knx_store::load_project(&conn).unwrap(),
                project,
                "{}",
                spec.key
            );
            knx_store::save_project(&conn, &project).unwrap();
            assert_eq!(knx_store::load_project(&conn).unwrap(), project);
        }
    }
}
