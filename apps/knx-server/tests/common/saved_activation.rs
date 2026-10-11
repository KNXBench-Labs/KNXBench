//! Independent, bounded saved-state oracle for synthetic and private tests.
//!
//! No domain mapping, parameter evaluation, or program-default inference.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct SavedDevice {
    pub tree_present: bool,
    pub objects: BTreeSet<String>,
    pub channel_ids: BTreeMap<String, String>,
    pub channels: Vec<(String, Option<String>, Option<bool>)>,
}

/// Namespace- and ancestry-aware source census, independent of production mapping.
pub fn saved_devices(xml: &[u8]) -> Result<BTreeMap<String, SavedDevice>, &'static str> {
    use quick_xml::events::Event;
    use quick_xml::name::ResolveResult;
    use quick_xml::{NsReader, XmlVersion};

    const NS: &str = "http://knx.org/xml/project/23";
    if xml.len() > 64 * 1024 * 1024 {
        return Err("source XML budget exceeded");
    }
    let mut reader = NsReader::from_reader(xml);
    let mut stack: Vec<Option<String>> = Vec::new();
    let mut devices = BTreeMap::new();
    let mut current: Option<(usize, String, SavedDevice)> = None;
    let mut roots = 0usize;
    let mut events = 0usize;
    loop {
        events += 1;
        if events > 1_000_000 || stack.len() > 128 {
            return Err("source XML work budget exceeded");
        }
        let (ns, event) = reader
            .read_resolved_event()
            .map_err(|_| "malformed source XML")?;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                let schema = matches!(ns, ResolveResult::Bound(ref bound) if bound.as_ref() == NS);
                let local = element.local_name();
                let name = local.as_ref();
                let mut attrs = BTreeMap::new();
                for attribute in element.attributes() {
                    let attribute = attribute.map_err(|_| "malformed source attribute")?;
                    let key = attribute.key.as_ref().to_owned();
                    let value = attribute
                        .normalized_value(XmlVersion::Implicit1_0)
                        .map_err(|_| "malformed source attribute value")?
                        .into_owned();
                    attrs.insert(key, value);
                }
                if stack.is_empty() {
                    roots += 1;
                    if roots != 1 || !schema || name != "KNX" {
                        return Err("source root is not schema 23");
                    }
                }
                let canonical = schema && stack.iter().all(Option::is_some);
                let names: Vec<_> = stack.iter().filter_map(|n| n.as_deref()).collect();
                if canonical
                    && name == "DeviceInstance"
                    && names.starts_with(&[
                        "KNX",
                        "Project",
                        "Installations",
                        "Installation",
                        "Topology",
                    ])
                    && matches!(
                        &names[5..],
                        ["Area", "Line", "Segment"] | ["Area", "Line"] | ["UnassignedDevices"]
                    )
                    && current.is_none()
                {
                    let id = required(&attrs, "Id")?.to_owned();
                    current = Some((stack.len(), id, SavedDevice::default()));
                }
                if let Some((depth, _, device)) = current.as_mut() {
                    let path: Vec<_> = stack
                        .iter()
                        .skip(*depth + 1)
                        .map(|n| n.as_deref())
                        .collect();
                    if canonical && name == "GroupObjectTree" && path.is_empty() {
                        if device.tree_present {
                            return Err("duplicate source object tree");
                        }
                        device.tree_present = true;
                        add_objects(device, attrs.get("GroupObjectInstances"))?;
                    } else if canonical
                        && name == "Node"
                        && path.first() == Some(&Some("GroupObjectTree"))
                        && path
                            .iter()
                            .skip(1)
                            .all(|n| matches!(n, Some("Node" | "Nodes")))
                    {
                        if !matches!(required(&attrs, "Type")?, "Channel" | "Folder") {
                            return Err("unsupported source tree node");
                        }
                        required(&attrs, "RefId")?;
                        add_objects(device, attrs.get("GroupObjectInstances"))?;
                    } else if canonical
                        && name == "ComObjectInstanceRef"
                        && path == [Some("ComObjectInstanceRefs")]
                    {
                        let object = required(&attrs, "RefId")?;
                        if let Some(channel) = attrs.get("ChannelId").filter(|v| !v.is_empty()) {
                            if device
                                .channel_ids
                                .insert(object.to_owned(), channel.clone())
                                .is_some()
                            {
                                return Err("duplicate source object channel claim");
                            }
                        }
                    } else if canonical
                        && name == "ChannelInstance"
                        && path == [Some("ChannelInstances")]
                    {
                        let id = required(&attrs, "Id")?.to_owned();
                        if device.channels.iter().any(|c| c.0 == id) {
                            return Err("duplicate source channel instance");
                        }
                        let active = attrs
                            .get("IsActive")
                            .map(|v| match v.as_str() {
                                "true" | "1" => Ok(true),
                                "false" | "0" => Ok(false),
                                _ => Err("invalid source channel activity"),
                            })
                            .transpose()?;
                        device
                            .channels
                            .push((id, attrs.get("RefId").cloned(), active));
                    }
                }
                if matches!(event, Event::Empty(_)) {
                    finish_device(&mut current, &mut devices, stack.len())?;
                } else {
                    stack.push(schema.then(|| name.to_owned()));
                }
            }
            Event::End(_) => {
                stack.pop().ok_or("unbalanced source XML")?;
                finish_device(&mut current, &mut devices, stack.len())?;
            }
            Event::DocType(_) => return Err("source DTD is not admitted"),
            Event::Eof => break,
            _ => {}
        }
    }
    if roots != 1 || !stack.is_empty() || current.is_some() {
        return Err("incomplete source XML");
    }
    Ok(devices)
}

fn required<'a>(attrs: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, &'static str> {
    attrs
        .get(key)
        .map(String::as_str)
        .filter(|v| !v.is_empty())
        .ok_or("required source identity is absent")
}

fn add_objects(device: &mut SavedDevice, value: Option<&String>) -> Result<(), &'static str> {
    for id in value.into_iter().flat_map(|v| v.split_ascii_whitespace()) {
        if !device.objects.insert(id.to_owned()) {
            return Err("duplicate saved-tree object");
        }
    }
    Ok(())
}

fn finish_device(
    current: &mut Option<(usize, String, SavedDevice)>,
    devices: &mut BTreeMap<String, SavedDevice>,
    depth: usize,
) -> Result<(), &'static str> {
    if current.as_ref().is_some_and(|c| c.0 == depth) {
        let (_, id, device) = current.take().ok_or("missing source device")?;
        if devices.insert(id, device).is_some() {
            return Err("duplicate source device");
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    Agreement,
    Difference,
    Undetermined,
    NotEvaluated,
}

/// Compare full and parent-relative spellings of the same program channel.
/// This is a test identity comparison, not an instance or activation inference.
pub fn channel_claim_matches(program: &str, stored: &str, evaluated: Option<&str>) -> bool {
    !stored.is_empty()
        && (evaluated == Some(stored) || evaluated == Some(format!("{program}_{stored}").as_str()))
}

#[test]
fn channel_claim_comparison_resolves_relative_program_refs_without_joining_instance_ids() {
    let program = "M-TEST_A-TEST";
    let actual = "M-TEST_A-TEST_CH-TEST";
    assert!(channel_claim_matches(program, "CH-TEST", Some(actual)));
    assert!(channel_claim_matches(program, actual, Some(actual)));
    assert!(!channel_claim_matches(
        program,
        "INSTANCE-TEST",
        Some(actual)
    ));
    assert!(!channel_claim_matches(
        program,
        "M-OTHER_A-OTHER_CH-TEST",
        Some(actual)
    ));
    assert!(!channel_claim_matches(program, "CH-TEST", None));
}

pub fn compare_activity(stored: bool, evaluated: &str) -> Result<Comparison, &'static str> {
    match evaluated {
        "Active" => Ok(if stored {
            Comparison::Agreement
        } else {
            Comparison::Difference
        }),
        "Inactive" => Ok(if stored {
            Comparison::Difference
        } else {
            Comparison::Agreement
        }),
        "Undetermined" => Ok(Comparison::Undetermined),
        "NotEvaluated" => Ok(Comparison::NotEvaluated),
        _ => Err("unknown evaluated activation token"),
    }
}

const REPORT_FLAGS: &[&str] = &[
    "object_agreement_seen",
    "object_difference_seen",
    "object_undetermined_seen",
    "object_not_evaluated_seen",
    "channel_activity_agreement_seen",
    "channel_activity_difference_seen",
    "channel_activity_undetermined_seen",
    "channel_activity_not_evaluated_seen",
    "channel_id_claim_seen",
    "channel_id_agreement_seen",
    "channel_id_difference_seen",
    "channel_id_not_adjudicated_seen",
    "explicit_channel_activity_seen",
    "source_override_outside_tree_seen",
    "source_tree_absent_seen",
    "extra_unscoped_activation_seen",
    "scoped_set_not_adjudicated_seen",
    "project_snapshot_unchanged",
    "input_unchanged",
    "nonempty_complete_object_comparison",
];

pub fn diagnostic_report() -> BTreeMap<&'static str, bool> {
    REPORT_FLAGS.iter().map(|&key| (key, false)).collect()
}

/// Completion requires integrity and actual evaluation, not agreement with ETS.
/// Unknown/unadjudicated observations never turn into a parity claim.
pub fn diagnostic_observed_agreement(report: &BTreeMap<&str, bool>) -> Result<bool, &'static str> {
    if report.len() != REPORT_FLAGS.len() || REPORT_FLAGS.iter().any(|k| !report.contains_key(k)) {
        return Err("diagnostic report schema is incomplete");
    }
    if !report["input_unchanged"]
        || !report["project_snapshot_unchanged"]
        || !report["nonempty_complete_object_comparison"]
        || !(report["object_agreement_seen"]
            || report["object_difference_seen"]
            || report["object_undetermined_seen"])
    {
        return Err("diagnostic integrity or evaluation coverage failed");
    }
    Ok(![
        "object_difference_seen",
        "extra_unscoped_activation_seen",
        "channel_activity_difference_seen",
        "channel_id_difference_seen",
        "object_undetermined_seen",
        "object_not_evaluated_seen",
        "channel_activity_undetermined_seen",
        "channel_activity_not_evaluated_seen",
        "channel_id_not_adjudicated_seen",
        "scoped_set_not_adjudicated_seen",
        "source_tree_absent_seen",
    ]
    .iter()
    .any(|k| report[k]))
}

#[test]
fn diagnostic_admission_distinguishes_findings_uncertainty_and_integrity_from_parity() {
    let mut report = diagnostic_report();
    report.insert("input_unchanged", true);
    report.insert("project_snapshot_unchanged", true);
    report.insert("nonempty_complete_object_comparison", true);
    report.insert("object_agreement_seen", true);
    assert_eq!(diagnostic_observed_agreement(&report), Ok(true));
    for finding in [
        "object_difference_seen",
        "extra_unscoped_activation_seen",
        "channel_activity_difference_seen",
        "channel_id_difference_seen",
        "object_undetermined_seen",
        "object_not_evaluated_seen",
        "channel_activity_not_evaluated_seen",
        "channel_id_not_adjudicated_seen",
        "scoped_set_not_adjudicated_seen",
        "source_tree_absent_seen",
    ] {
        report.insert(finding, true);
        assert_eq!(diagnostic_observed_agreement(&report), Ok(false));
        assert!(
            report[finding],
            "diagnostic admission must not erase a finding"
        );
        report.insert(finding, false);
    }
    for invariant in [
        "input_unchanged",
        "project_snapshot_unchanged",
        "nonempty_complete_object_comparison",
        "object_agreement_seen",
    ] {
        report.insert(invariant, false);
        assert!(diagnostic_observed_agreement(&report).is_err());
        report.insert(invariant, true);
    }
    report.remove("object_difference_seen");
    assert!(diagnostic_observed_agreement(&report).is_err());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_comparison_never_rounds_uncertainty_and_checks_both_difference_directions() {
        for stored in [false, true] {
            assert_eq!(
                compare_activity(stored, "Undetermined"),
                Ok(Comparison::Undetermined)
            );
            assert_eq!(
                compare_activity(stored, "NotEvaluated"),
                Ok(Comparison::NotEvaluated)
            );
            assert_eq!(
                compare_activity(stored, "Active"),
                Ok(if stored {
                    Comparison::Agreement
                } else {
                    Comparison::Difference
                })
            );
            assert_eq!(
                compare_activity(stored, "Inactive"),
                Ok(if stored {
                    Comparison::Difference
                } else {
                    Comparison::Agreement
                })
            );
        }
        assert!(compare_activity(true, "unknown").is_err());
    }

    #[test]
    fn saved_oracle_rejects_a_topology_lookalike_under_extensions() {
        let xml = SYNTHETIC
            .replace("<Topology>", "<Extensions><Topology>")
            .replace("</Topology>", "</Topology></Extensions>");
        assert!(saved_devices(xml.as_bytes())
            .expect("synthetic lookalike remains XML")
            .is_empty());
    }

    #[test]
    fn saved_oracle_distinguishes_absent_and_empty_trees() {
        let xml = r#"<KNX xmlns="http://knx.org/xml/project/23"><Project><Installations><Installation><Topology><UnassignedDevices><DeviceInstance Id="D-TEST-EMPTY"><GroupObjectTree/></DeviceInstance><DeviceInstance Id="D-TEST-ABSENT"/></UnassignedDevices></Topology></Installation></Installations></Project></KNX>"#;
        let devices = saved_devices(xml.as_bytes()).expect("synthetic source must parse");
        assert!(devices["D-TEST-EMPTY"].tree_present);
        assert!(!devices["D-TEST-ABSENT"].tree_present);
        assert!(devices.values().all(|d| d.objects.is_empty()));
    }

    #[test]
    fn saved_oracle_refuses_duplicates_malformed_xml_and_invalid_channel_claims() {
        for xml in [
            SYNTHETIC.replace("O-TEST-A\"><Nodes>", "O-TEST-A O-TEST-A\"><Nodes>"),
            SYNTHETIC.replace("IsActive=\"true\"", "IsActive=\"maybe\""),
            SYNTHETIC.replace("</KNX>", ""),
            format!("<!DOCTYPE KNX>{SYNTHETIC}"),
        ] {
            assert!(saved_devices(xml.as_bytes()).is_err());
        }
    }

    #[test]
    fn saved_oracle_ignores_foreign_namespace_lookalikes_and_refuses_depth_overflow() {
        let foreign = SYNTHETIC.replace(
            "<DeviceInstance Id=",
            "<DeviceInstance xmlns=\"urn:synthetic-foreign\" Id=",
        );
        assert!(saved_devices(foreign.as_bytes())
            .expect("foreign data stays XML")
            .is_empty());
        let deep = format!(
            "<KNX xmlns=\"http://knx.org/xml/project/23\">{}{}</KNX>",
            "<Wrapper>".repeat(130),
            "</Wrapper>".repeat(130)
        );
        assert!(saved_devices(deep.as_bytes()).is_err());
    }

    pub const SYNTHETIC: &str = r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><Installations><Installation InstallationId="0"><Topology><Area Id="A-TEST" Address="1"><Line Id="L-TEST" Address="1"><Segment Id="S-TEST" Number="1"><DeviceInstance Id="D-TEST"><ComObjectInstanceRefs><ComObjectInstanceRef RefId="O-TEST-A" ChannelId="M-TEST_A-TEST_CH-TEST"/><ComObjectInstanceRef RefId="O-TEST-ORPHAN" ChannelId="M-TEST_A-TEST_CH-TEST"/></ComObjectInstanceRefs><ChannelInstances><ChannelInstance Id="D-TEST_CI-TEST" RefId="CH-TEST" IsActive="true"/><ChannelInstance Id="D-TEST_CI-UNKNOWN" RefId="CH-UNKNOWN"/></ChannelInstances><GroupObjectTree GroupObjectInstances="O-TEST-INDEPENDENT"><Node Type="Channel" RefId="CH-TEST" GroupObjectInstances="O-TEST-A"><Nodes><Node Type="Folder" RefId="PB-TEST" GroupObjectInstances="O-TEST-B"/></Nodes></Node></GroupObjectTree><Extensions><GroupObjectTree GroupObjectInstances="O-TEST-LOOKALIKE"/></Extensions></DeviceInstance></Segment></Line></Area></Topology></Installation></Installations></Project></KNX>"#;

    #[test]
    fn saved_oracle_separates_tree_membership_channel_ids_and_explicit_channel_activity() {
        let devices = saved_devices(SYNTHETIC.as_bytes()).expect("synthetic oracle must parse");
        let device = &devices["D-TEST"];
        assert_eq!(
            device.objects,
            BTreeSet::from([
                "O-TEST-A".into(),
                "O-TEST-B".into(),
                "O-TEST-INDEPENDENT".into()
            ])
        );
        assert!(!device.objects.contains("O-TEST-ORPHAN"));
        assert!(!device.objects.contains("O-TEST-LOOKALIKE"));
        assert_eq!(device.channel_ids["O-TEST-A"], "M-TEST_A-TEST_CH-TEST");
        assert_eq!(
            device.channels[0],
            ("D-TEST_CI-TEST".into(), Some("CH-TEST".into()), Some(true))
        );
        assert_eq!(device.channels[1].2, None);
    }
}
