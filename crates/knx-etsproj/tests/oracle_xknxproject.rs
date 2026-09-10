//! Cross-checks the import against `xknxproject`'s own reading of the
//! reference project (`project_dump.json`, committed at the workspace
//! root). `xknxproject` is a good reader and a lossy one (RESEARCH §7.1):
//! used here as a second opinion on the parts it does read, with the parts
//! it is known to lose asserted as differences rather than ignored, so a
//! future version of the oracle that starts reading them makes this test
//! fail rather than stay silently comparable to less than it could be.

mod support;
use support::*;

use knx_etsproj::import_knxproj;

fn oracle() -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(oracle_dump_path()).unwrap()).unwrap()
}

#[test]
fn group_addresses_agree_with_the_oracle_by_address_and_name() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let out = import_knxproj(&reference_ets4_path()).unwrap();
    let oracle = oracle();
    let theirs = oracle["group_addresses"].as_object().unwrap();
    assert_eq!(theirs.len(), 514);

    let style = out.project.info.group_address_style;
    for ga in &out.project.installations[0].group_addresses {
        let key = ga.address.format(style);
        let entry = theirs
            .get(&key)
            .unwrap_or_else(|| panic!("oracle lacks {key}"));
        assert_eq!(entry["name"].as_str().unwrap(), ga.name);
    }
}

#[test]
fn devices_agree_with_the_oracle_except_for_the_one_it_loses() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let out = import_knxproj(&reference_ets4_path()).unwrap();
    let oracle = oracle();
    let theirs = oracle["devices"].as_object().unwrap();
    assert_eq!(theirs.len(), 35);
    assert_eq!(out.project.devices.iter().count(), 36);

    let addressed: Vec<_> = out
        .project
        .devices
        .iter()
        .filter(|d| d.address.is_some())
        .collect();
    assert_eq!(addressed.len(), 35);
    for d in addressed {
        let key = d.address.unwrap().to_string();
        assert!(theirs.contains_key(&key), "oracle lacks device {key}");
    }
}

#[test]
fn the_oracle_still_loses_exactly_what_research_measured() {
    // If any of these starts holding data, RESEARCH §7.1 needs updating and
    // the comparison above can be widened.
    let oracle = oracle();
    assert!(oracle.get("parameters").is_none());
    assert_eq!(oracle["functions"].as_object().map(|f| f.len()), Some(0));

    // The oracle exposes only communication objects that carry a group
    // address link: 570 of the 907 instances in the file. The 337 unlinked
    // instances are exactly the ones a parameter editor needs.
    assert_eq!(
        oracle["communication_objects"].as_object().unwrap().len(),
        570
    );

    // Group ranges are exposed as the 7 main ranges; nesting is not.
    assert_eq!(oracle["group_ranges"].as_object().unwrap().len(), 7);
}

#[test]
fn the_project_metadata_agrees_with_the_oracle() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let out = import_knxproj(&reference_ets4_path()).unwrap();
    let oracle = oracle();
    assert_eq!(oracle["info"]["schema_version"].as_str().unwrap(), "11");
    assert_eq!(
        oracle["info"]["project_id"].as_str().unwrap(),
        out.project.info.project_id
    );
    assert_eq!(
        oracle["info"]["name"].as_str().unwrap(),
        out.project.info.name
    );
    assert_eq!(
        oracle["info"]["group_address_style"].as_str().unwrap(),
        "ThreeLevel"
    );
}

#[test]
fn our_linked_communication_objects_match_the_oracle_count() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let out = import_knxproj(&reference_ets4_path()).unwrap();
    let linked = out
        .project
        .devices
        .iter()
        .flat_map(|d| d.com_objects.clone())
        .filter_map(|id| out.project.devices.com_object(id))
        .filter(|c| !c.links.is_empty())
        .count();
    assert_eq!(linked, 570);
}
