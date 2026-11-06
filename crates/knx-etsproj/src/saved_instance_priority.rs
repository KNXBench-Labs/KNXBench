//! Captures bounded schema-23 saved declarations after original source retention is complete.

use crate::{
    opaque::{OpaqueEntry, OpaqueKind},
    source::{SourceComObjectInstance, SourceDevice, SourceDocument},
    xpath,
};
use knx_core::{
    Project, SavedInstancePriority as Fact, SavedPriorityAbsent, SavedPriorityBindings,
    SavedPriorityDeclared, SavedPriorityOmission as Omission, SavedPrioritySource,
    SavedPriorityUnavailable as Reason, SourceRef, MAX_SAVED_PRIORITY_CAPTURE_BYTES,
    MAX_SAVED_PRIORITY_OBJECTS, MAX_SAVED_PRIORITY_STRING_BYTES,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

// Conservative charge includes hash-table buckets, borrowed keys, output slots and boxed facts.
const INDEX_ENTRY_BYTES: usize = 512;
const FACT_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptureLimit {
    Objects,
    StringBytes,
    WorkBytes,
}

#[derive(Default, Debug)]
struct Work {
    bytes: usize,
    source_objects: usize,
    normalized_objects: usize,
    visits: usize,
}
impl Work {
    fn charge(&mut self, bytes: usize) -> Result<(), CaptureLimit> {
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or(CaptureLimit::WorkBytes)?;
        if self.bytes > MAX_SAVED_PRIORITY_CAPTURE_BYTES {
            return Err(CaptureLimit::WorkBytes);
        }
        Ok(())
    }
    fn string(&mut self, text: &str) -> Result<(), CaptureLimit> {
        if text.len() > MAX_SAVED_PRIORITY_STRING_BYTES {
            return Err(CaptureLimit::StringBytes);
        }
        self.charge(text.len())
    }
    fn visit(&mut self) {
        self.visits += 1;
    }
}

fn source_devices(source: &SourceDocument) -> impl Iterator<Item = (&SourceDevice, bool)> {
    source.installations.iter().flat_map(|installation| {
        installation
            .areas
            .iter()
            .flat_map(|area| &area.lines)
            .flat_map(|line| &line.devices)
            .map(|d| (d, false))
            .chain(installation.unassigned_devices.iter().map(|d| (d, true)))
    })
}

fn is_module(com: &knx_core::ComObjectInstance) -> bool {
    com.module_instance.is_some() || com.source.ets_id.starts_with("MD-")
}
fn unavailable(reason: Reason) -> Fact {
    Fact::Unavailable { reason }
}

/// Pre-admits every index, copied source string and expanded locator before allocating them.
fn preflight(
    project: &Project,
    source: &SourceDocument,
    member: &str,
    bytes: &[u8],
    opaque: &[OpaqueEntry],
) -> Result<Work, CaptureLimit> {
    // Count before index/payload admission so byte exhaustion cannot mask the count guard.
    let mut work = Work::default();
    let normalized = project
        .devices
        .com_objects()
        .take(MAX_SAVED_PRIORITY_OBJECTS + 1)
        .count();
    work.visits += normalized;
    let mut declarations = 0usize;
    let mut memberships = 0usize;
    for (device, _) in source_devices(source) {
        work.visit();
        declarations = declarations
            .checked_add(device.com_objects.len())
            .ok_or(CaptureLimit::Objects)?;
        memberships = memberships
            .checked_add(device.group_object_tree.len())
            .ok_or(CaptureLimit::Objects)?;
        if declarations > MAX_SAVED_PRIORITY_OBJECTS || memberships > MAX_SAVED_PRIORITY_OBJECTS {
            return Err(CaptureLimit::Objects);
        }
    }
    if normalized > MAX_SAVED_PRIORITY_OBJECTS {
        return Err(CaptureLimit::Objects);
    }
    work.string(member)?;
    work.charge(bytes.len().checked_mul(2).ok_or(CaptureLimit::WorkBytes)?)?; // Digest and exact-byte comparison; no diagnostic copy.
    let line_base = xpath::line_device(23, "").len();
    let unassigned_base = xpath::unassigned_device("").len();
    let object_base = xpath::com_object("", "").len();
    for (device, unassigned) in source_devices(source) {
        work.visit();
        work.string(&device.id)?;
        work.charge(INDEX_ENTRY_BYTES)?;
        let owner_len = (if unassigned {
            unassigned_base
        } else {
            line_base
        })
        .checked_add(device.id.len())
        .ok_or(CaptureLimit::WorkBytes)?;
        if owner_len > MAX_SAVED_PRIORITY_STRING_BYTES {
            return Err(CaptureLimit::StringBytes);
        }
        for ref_id in &device.group_object_tree {
            work.visit();
            work.string(ref_id)?;
            work.charge(INDEX_ENTRY_BYTES)?;
        }
        for com in &device.com_objects {
            work.visit();
            work.source_objects = work
                .source_objects
                .checked_add(1)
                .ok_or(CaptureLimit::WorkBytes)?;
            if work.source_objects > MAX_SAVED_PRIORITY_OBJECTS {
                return Err(CaptureLimit::Objects);
            }
            work.string(&com.ref_id)?;
            work.charge(INDEX_ENTRY_BYTES)?;
            let locator_len = owner_len
                .checked_add(object_base)
                .and_then(|n| n.checked_add(com.ref_id.len()))
                .ok_or(CaptureLimit::WorkBytes)?;
            if locator_len > MAX_SAVED_PRIORITY_STRING_BYTES {
                return Err(CaptureLimit::StringBytes);
            }
            for attr in &com.other {
                work.visit();
                work.charge(1)?;
                if attr.name == "Priority" {
                    work.string(&attr.value)?;
                }
            }
        }
    }
    for device in project.devices.iter() {
        work.visit();
        work.string(&device.source.path)?;
        work.string(&device.source.ets_id)?;
        work.charge(INDEX_ENTRY_BYTES)?;
        for _ in &device.com_objects {
            work.visit();
            work.charge(INDEX_ENTRY_BYTES)?;
        }
    }
    for com in project.devices.com_objects() {
        work.visit();
        work.normalized_objects = work
            .normalized_objects
            .checked_add(1)
            .ok_or(CaptureLimit::WorkBytes)?;
        if work.normalized_objects > MAX_SAVED_PRIORITY_OBJECTS {
            return Err(CaptureLimit::Objects);
        }
        work.charge(INDEX_ENTRY_BYTES + FACT_BYTES)?;
        work.string(&com.source.path)?;
        work.string(&com.source.ets_id)?;
        if let Some(owner) = project.devices.get(com.device) {
            // Charge all owned payload strings before any clone/qualified locator expansion.
            let locator_len = line_base
                .max(unassigned_base)
                .checked_add(owner.source.ets_id.len())
                .and_then(|n| n.checked_add(object_base))
                .and_then(|n| n.checked_add(com.source.ets_id.len()))
                .ok_or(CaptureLimit::WorkBytes)?;
            if locator_len > MAX_SAVED_PRIORITY_STRING_BYTES {
                return Err(CaptureLimit::StringBytes);
            }
            work.charge(locator_len)?;
            work.string(&owner.source.path)?;
            work.string(&owner.source.ets_id)?;
        }
        work.string(member)?;
        work.charge(64)?;
    }
    for entry in opaque {
        work.visit();
        // All entries are visited, but only candidate rows are indexed or hashed.
        work.charge(1)?;
        if entry.name == "Priority"
            || (entry.source_path == member && entry.xpath.is_empty() && entry.name.is_empty())
        {
            work.string(&entry.source_path)?;
            work.string(&entry.xpath)?;
            work.string(&entry.name)?;
            work.string(&entry.sha256)?;
            work.charge(INDEX_ENTRY_BYTES)?;
            if entry.name == "Priority" {
                if entry.bytes.len() > MAX_SAVED_PRIORITY_STRING_BYTES {
                    return Err(CaptureLimit::StringBytes);
                }
                // UTF-8 raw is also cloned into each fact. Duplicates are overcharged conservatively.
                work.charge(
                    entry
                        .bytes
                        .len()
                        .checked_mul(2)
                        .ok_or(CaptureLimit::WorkBytes)?,
                )?;
            }
        }
    }
    Ok(work)
}

pub(crate) fn capture_saved_instance_priorities(
    project: &mut Project,
    source: &SourceDocument,
    member: &str,
    original: &[u8],
    opaque: &[OpaqueEntry],
) {
    capture(project, source, member, original, opaque);
}

fn capture(
    project: &mut Project,
    source: &SourceDocument,
    member: &str,
    original: &[u8],
    opaque: &[OpaqueEntry],
) -> Option<Work> {
    let admitted = if source.schema_version == 23 {
        preflight(project, source, member, original, opaque).ok()
    } else {
        None
    };
    let initial = if source.schema_version != 23 {
        Some(Reason::UnsupportedSourceSchema)
    } else if admitted.is_none() {
        Some(Reason::ResourceLimit)
    } else {
        None
    };
    if let Some(reason) = initial {
        // No indexes, raw clones or admitted prefix. Module scope stays explicit at the cap.
        for com in project.devices.com_objects_mut() {
            com.saved_instance_priority = unavailable(
                if reason != Reason::UnsupportedSourceSchema && is_module(com) {
                    Reason::ModuleScope
                } else {
                    reason
                },
            );
        }
        return None;
    }
    let mut work = admitted?;
    let mut owners: HashMap<&str, (&SourceDevice, bool, usize)> = HashMap::new();
    let mut memberships: HashMap<(&str, &str), usize> = HashMap::new();
    let mut declarations: HashMap<(&str, &str), (&SourceComObjectInstance, usize)> = HashMap::new();
    for (device, unassigned) in source_devices(source) {
        work.visit();
        let owner = owners.entry(&device.id).or_insert((device, unassigned, 0));
        owner.2 += 1;
        for ref_id in &device.group_object_tree {
            work.visit();
            *memberships.entry((&device.id, ref_id)).or_default() += 1;
        }
        for com in &device.com_objects {
            work.visit();
            let item = declarations
                .entry((&device.id, &com.ref_id))
                .or_insert((com, 0));
            item.1 += 1;
        }
    }
    let mut attrs: HashMap<(&str, &str), (&OpaqueEntry, usize)> = HashMap::new();
    let mut whole: Option<&OpaqueEntry> = None;
    let mut whole_count = 0;
    let mut unkeyed = false;
    for entry in opaque {
        work.visit();
        if entry.source_path == member && entry.xpath.is_empty() && entry.name.is_empty() {
            whole = Some(entry);
            whole_count += 1;
        }
        if entry.name == "Priority" {
            let item = attrs
                .entry((&entry.source_path, &entry.xpath))
                .or_insert((entry, 0));
            item.1 += 1;
            unkeyed |= entry.source_path == member && entry.xpath.is_empty();
        }
    }
    let digest = format!("{:x}", Sha256::digest(original));
    let source_ok = whole_count == 1
        && whole.is_some_and(|entry| {
            entry.kind == OpaqueKind::ContainerEntry
                && entry.bytes == original
                && entry.sha256 == digest
        });
    let bindings = SavedPriorityBindings::new(project);
    work.visits += project
        .devices
        .iter()
        .map(|d| 1 + d.com_objects.len())
        .sum::<usize>()
        + project.devices.com_objects().count();
    let results: Vec<_> = project
        .devices
        .com_objects()
        .map(|com| {
            work.visit();
            let value = (|| {
                if is_module(com) {
                    return unavailable(Reason::ModuleScope);
                }
                let Some(device) = project.devices.get(com.device) else {
                    return unavailable(Reason::AmbiguousOwner);
                };
                if !bindings.unique_owner(com, com.device)
                    || member.is_empty()
                    || device.source.ets_id.is_empty()
                    || com.source.ets_id.is_empty()
                    || device.source.path != member
                    || com.source.path != member
                    || crate::values::com_object_number(&com.source.ets_id)
                        .or_else(|_| {
                            crate::values::device_local_com_object_number(&com.source.ets_id)
                        })
                        .is_err()
                {
                    return unavailable(Reason::AmbiguousOwner);
                }
                let Some((_, unassigned, 1)) = owners.get(device.source.ets_id.as_str()) else {
                    return unavailable(Reason::AmbiguousOwner);
                };
                if memberships.get(&(device.source.ets_id.as_str(), com.source.ets_id.as_str()))
                    != Some(&1)
                {
                    return unavailable(Reason::AmbiguousOwner);
                }
                let declaration =
                    declarations.get(&(device.source.ets_id.as_str(), com.source.ets_id.as_str()));
                if declaration.is_some_and(|(_, count)| *count != 1) {
                    return unavailable(Reason::AmbiguousDeclaration);
                }
                let mut priority = declaration
                    .into_iter()
                    .flat_map(|(element, _)| element.other.iter())
                    .filter(|attr| attr.name == "Priority");
                let raw = priority.next().map(|attr| attr.value.as_str());
                if priority.next().is_some() {
                    return unavailable(Reason::AmbiguousDeclaration);
                }
                let owner_path = if *unassigned {
                    xpath::unassigned_device(&device.source.ets_id)
                } else {
                    xpath::line_device(23, &device.source.ets_id)
                };
                let locator = xpath::com_object(&owner_path, &com.source.ets_id);
                let retained = attrs.get(&(member, locator.as_str()));
                let attr_ok = match raw {
                    Some(raw) => retained.is_some_and(|(entry, count)| {
                        *count == 1
                            && entry.kind == OpaqueKind::RetainedAttribute
                            && entry.bytes == raw.as_bytes()
                            && entry.sha256 == format!("{:x}", Sha256::digest(raw.as_bytes()))
                    }),
                    None => retained.is_none() && !unkeyed,
                };
                if !source_ok || !attr_ok {
                    return unavailable(Reason::SourceIntegrity);
                }
                let source = SavedPrioritySource {
                    source_schema: 23,
                    device_source: device.source.clone(),
                    object_source: SourceRef {
                        path: member.into(),
                        ets_id: com.source.ets_id.clone(),
                    },
                    owner_xpath: locator,
                    topology_sha256: digest.clone(),
                };
                match raw {
                    Some(raw) => Fact::Declared(Box::new(SavedPriorityDeclared {
                        raw: raw.into(),
                        source,
                    })),
                    None => Fact::Absent(Box::new(SavedPriorityAbsent {
                        omission: if declaration.is_some() {
                            Omission::AttributeOmitted
                        } else {
                            Omission::InstanceElementOmitted
                        },
                        source,
                    })),
                }
            })();
            value
        })
        .collect();
    for (com, value) in project.devices.com_objects_mut().zip(results) {
        com.saved_instance_priority = value;
    }
    Some(work)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{RetainedAttribute, SourceInstallation};
    const MEMBER: &str = "P-TEST/0.xml";
    const ORIGINAL: &[u8] = b"synthetic original source octets";

    fn seed(count: usize) -> (Project, SourceDocument, Vec<OpaqueEntry>) {
        let mut device = SourceDevice {
            id: "P-TEST-0_DI-1".into(),
            ..Default::default()
        };
        for n in 0..count {
            let ref_id = format!("O-{n}_R-1");
            device.group_object_tree.push(ref_id.clone());
            device.com_objects.push(SourceComObjectInstance {
                ref_id,
                other: vec![RetainedAttribute {
                    xpath: String::new(),
                    name: "Priority".into(),
                    value: "High".into(),
                }],
                ..Default::default()
            });
        }
        let source = SourceDocument {
            schema_version: 23,
            project_id: "P-TEST".into(),
            installations: vec![SourceInstallation {
                unassigned_devices: vec![device],
                ..Default::default()
            }],
            ..Default::default()
        };
        let mapped = crate::map::map(&source, MEMBER).unwrap();
        let mut opaque = vec![OpaqueEntry {
            source_path: MEMBER.into(),
            xpath: String::new(),
            name: String::new(),
            kind: OpaqueKind::ContainerEntry,
            bytes: ORIGINAL.to_vec(),
            sha256: crate::opaque::sha256_hex(ORIGINAL),
        }];
        opaque.extend(
            mapped
                .retained
                .iter()
                .map(|a| crate::opaque::from_retained_attribute(MEMBER, a)),
        );
        (mapped.project, source, opaque)
    }
    fn assert_reason(project: &Project, reason: Reason) {
        assert!(project.devices.com_objects().next().is_some());
        for com in project.devices.com_objects() {
            assert_eq!(com.saved_instance_priority, unavailable(reason));
        }
    }

    #[test]
    fn saved_instance_priority_independent_original_and_attribute_integrity_checks() {
        let (model, source, original) = seed(1);
        let mut good = model.clone();
        capture(&mut good, &source, MEMBER, ORIGINAL, &original).unwrap();
        assert!(
            matches!(&good.devices.com_objects().next().unwrap().saved_instance_priority,Fact::Declared(v) if v.raw=="High")
        );
        // Each tamper starts from a valid bounded control. Hash mismatches cannot be masked by lengths.
        for case in 0..15 {
            let mut entries = original.clone();
            let attribute = entries.iter().position(|e| e.name == "Priority").unwrap();
            match case {
                0 => {
                    entries.remove(0);
                }
                1 => entries.push(entries[0].clone()),
                2 => entries[0].kind = OpaqueKind::RetainedElement,
                3 => entries[0].source_path = "other/0.xml".into(),
                4 => entries[0].sha256 = "0".repeat(64),
                5 => entries[0].bytes[0] ^= 1,
                6 => {
                    entries.remove(attribute);
                }
                7 => entries.push(entries[attribute].clone()),
                8 => entries[attribute].kind = OpaqueKind::ContainerEntry,
                9 => entries[attribute].source_path = "other/0.xml".into(),
                10 => entries[attribute].xpath.clear(),
                11 => entries[attribute].xpath = entries[attribute].xpath.replace("DI-1", "DI-2"),
                12 => entries[attribute].sha256 = "0".repeat(64),
                13 => entries[attribute].bytes = b"high".to_vec(),
                14 => {
                    entries[attribute].name = "priority".into();
                }
                _ => unreachable!(),
            }
            let preserved = entries.clone();
            let mut project = model.clone();
            capture(&mut project, &source, MEMBER, ORIGINAL, &entries);
            assert_reason(&project, Reason::SourceIntegrity);
            assert_eq!(entries, preserved);
        }
        // Rehashing different source/attribute octets still fails exact-byte comparison.
        let mut entries = original.clone();
        entries[0].bytes = b"different original member".to_vec();
        entries[0].sha256 = crate::opaque::sha256_hex(&entries[0].bytes);
        let mut project = model.clone();
        capture(&mut project, &source, MEMBER, ORIGINAL, &entries);
        assert_reason(&project, Reason::SourceIntegrity);
        let mut entries = original.clone();
        entries[1].bytes = b"Alert".to_vec();
        entries[1].sha256 = crate::opaque::sha256_hex(&entries[1].bytes);
        let mut project = model.clone();
        capture(&mut project, &source, MEMBER, ORIGINAL, &entries);
        assert_reason(&project, Reason::SourceIntegrity);
        let mut absent_source = source.clone();
        absent_source.installations[0].unassigned_devices[0].com_objects[0]
            .other
            .clear();
        let mut project = model.clone();
        capture(&mut project, &absent_source, MEMBER, ORIGINAL, &original);
        assert_reason(&project, Reason::SourceIntegrity);
        let mut project = model.clone();
        capture(
            &mut project,
            &absent_source,
            MEMBER,
            ORIGINAL,
            &original[..1],
        );
        assert!(
            matches!(&project.devices.com_objects().next().unwrap().saved_instance_priority,Fact::Absent(v) if v.omission==Omission::AttributeOmitted)
        );
        let mut source = source;
        let duplicate =
            source.installations[0].unassigned_devices[0].com_objects[0].other[0].clone();
        source.installations[0].unassigned_devices[0].com_objects[0]
            .other
            .push(duplicate);
        let mut project = model;
        capture(&mut project, &source, MEMBER, ORIGINAL, &original);
        assert_reason(&project, Reason::AmbiguousDeclaration);
    }

    #[test]
    fn saved_instance_priority_normalized_owner_list_record_and_module_refusals() {
        let (model, source, entries) = seed(1);
        let id = model.devices.com_objects().next().unwrap().id;
        let owner = model.devices.com_objects().next().unwrap().device;
        for case in 0..9 {
            let mut project = model.clone();
            match case {
                0 => project.devices.get_mut(owner).unwrap().com_objects.push(id),
                1 => project.devices.get_mut(owner).unwrap().com_objects.clear(),
                2 => project.devices.com_object_mut(id).unwrap().device = knx_core::DeviceId(900),
                3 => project.devices.com_object_mut(id).unwrap().source.path = "other/0.xml".into(),
                4 => {
                    project.devices.com_object_mut(id).unwrap().id =
                        knx_core::ComObjectInstanceId(900)
                }
                5 => project.devices.com_object_mut(id).unwrap().source.ets_id = "broken".into(),
                6 => {
                    let mut other = project.devices.get(owner).unwrap().clone();
                    other.id = knx_core::DeviceId(900);
                    other.source.ets_id = "DI-OTHER".into();
                    project.devices.insert(other);
                }
                7 => {
                    let mut other = project.devices.com_object(id).unwrap().clone();
                    other.id = knx_core::ComObjectInstanceId(900);
                    project.devices.insert_com_object(other);
                }
                8 => {
                    let mut hidden = project.devices.com_object(id).unwrap().clone();
                    hidden.id = knx_core::ComObjectInstanceId(900);
                    hidden.source.ets_id = "O-8_R-1".into();
                    project.devices.insert_com_object(hidden);
                    project
                        .devices
                        .com_object_mut(knx_core::ComObjectInstanceId(900))
                        .unwrap()
                        .id = id;
                }
                _ => unreachable!(),
            }
            capture(&mut project, &source, MEMBER, ORIGINAL, &entries);
            assert_reason(&project, Reason::AmbiguousOwner);
        }
        for resolved in [false, true] {
            let mut project = model.clone();
            let object = project.devices.com_object_mut(id).unwrap();
            object.source.ets_id = "MD-1_M-1_MI-1_O-1-0_R-1".into();
            if resolved {
                object.module_instance = Some(knx_core::ModuleInstanceId(1));
            }
            capture(&mut project, &source, MEMBER, ORIGINAL, &entries);
            assert_reason(&project, Reason::ModuleScope);
        }
        let mut project = model;
        let mut legacy = source;
        legacy.schema_version = 21;
        capture(&mut project, &legacy, MEMBER, ORIGINAL, &entries);
        assert_reason(&project, Reason::UnsupportedSourceSchema);
    }

    #[test]
    fn saved_instance_priority_resource_meter_exact_limits_and_linear_counted_work() {
        let mut work = Work::default();
        work.charge(MAX_SAVED_PRIORITY_CAPTURE_BYTES).unwrap();
        assert!(work.charge(1).is_err());
        let mut work = Work::default();
        assert!(work.charge(usize::MAX).is_err());
        let mut work = Work::default();
        work.string(&"x".repeat(MAX_SAVED_PRIORITY_STRING_BYTES))
            .unwrap();
        assert!(work
            .string(&"x".repeat(MAX_SAVED_PRIORITY_STRING_BYTES + 1))
            .is_err());
        let (mut boundary, mut boundary_source, mut boundary_entries) = seed(1);
        let raw = "ü".repeat(2048);
        boundary_source.installations[0].unassigned_devices[0].com_objects[0].other[0].value =
            raw.clone();
        boundary_entries[1].bytes = raw.as_bytes().to_vec();
        boundary_entries[1].sha256 = crate::opaque::sha256_hex(raw.as_bytes());
        capture(
            &mut boundary,
            &boundary_source,
            MEMBER,
            ORIGINAL,
            &boundary_entries,
        )
        .unwrap();
        assert!(
            matches!(&boundary.devices.com_objects().next().unwrap().saved_instance_priority,Fact::Declared(v) if v.raw==raw)
        );
        let (mut project, source, entries) = seed(128);
        let original = entries.clone();
        let a = capture(&mut project, &source, MEMBER, ORIGINAL, &entries).unwrap();
        let (mut project, source, entries) = seed(256);
        let b = capture(&mut project, &source, MEMBER, ORIGINAL, &entries).unwrap();
        eprintln!("saved_instance_priority counted-work: n128 visits={} charged={} objects={}; n256 visits={} charged={} objects={}", a.visits,a.bytes,a.normalized_objects,b.visits,b.bytes,b.normalized_objects);
        assert!(a.visits > 128);
        assert!(b.visits <= 2 * a.visits + 10);
        assert!(b.bytes <= 2 * a.bytes + 1024);
        assert_eq!(original.len(), 129);
        for variant in 0..6 {
            let (mut project, mut source, mut entries) = seed(2);
            let id = project.devices.com_objects().next().unwrap().id;
            let owner = project.devices.com_objects().next().unwrap().device;
            match variant {
                0 => {
                    source.installations[0].unassigned_devices[0].com_objects[0].other[0].value =
                        "ü".repeat(2049)
                }
                1 => project.devices.get_mut(owner).unwrap().source.ets_id = "x".repeat(4097),
                2 => project.devices.com_object_mut(id).unwrap().source.path = "x".repeat(4097),
                3 => project.devices.get_mut(owner).unwrap().source.ets_id = "x".repeat(4000), // locator alone exceeds the cap
                4 => entries[1].xpath = "x".repeat(4097),
                5 => {
                    let extra = entries[1].clone();
                    for _ in 0..30_000 {
                        entries.push(extra.clone());
                    }
                } // index work, not raw count
                _ => unreachable!(),
            }
            let retained = entries.clone();
            assert!(capture(&mut project, &source, MEMBER, ORIGINAL, &entries).is_none());
            assert_reason(&project, Reason::ResourceLimit);
            assert_eq!(entries, retained);
        }
        // Count guard is independent: tiny source/project, but too many original override occurrences.
        let (mut project, mut source, entries) = seed(1);
        let one = source.installations[0].unassigned_devices[0].com_objects[0].clone();
        source.installations[0].unassigned_devices[0].com_objects =
            vec![one; MAX_SAVED_PRIORITY_OBJECTS + 1];
        assert_eq!(
            preflight(&project, &source, MEMBER, ORIGINAL, &entries).unwrap_err(),
            CaptureLimit::Objects
        );
        assert!(capture(&mut project, &source, MEMBER, ORIGINAL, &entries).is_none());
        assert_reason(&project, Reason::ResourceLimit);
        let (mut project, source, entries) = seed(1);
        assert!(capture(
            &mut project,
            &source,
            MEMBER,
            &vec![b'x'; MAX_SAVED_PRIORITY_CAPTURE_BYTES + 1],
            &entries
        )
        .is_none());
        assert_reason(&project, Reason::ResourceLimit);
    }
}
