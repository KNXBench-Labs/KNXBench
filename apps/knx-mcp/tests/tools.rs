//! Every MCP tool against a saved fixture project, through the plain tool functions.

mod common;

use common::*;
use knx_mcp::tools::{self, DeviceView, Page};
use serde_json::{json, Value};

fn page() -> Page {
    Page::new(Some(500), None).unwrap()
}

fn items(value: &Value) -> &Vec<Value> {
    value["items"].as_array().expect("a paged list")
}

#[test]
fn every_response_carries_the_envelope() {
    let dir = tempfile::tempdir().unwrap();
    let path = save(dir.path(), "home.knxdb", &project("Home"));
    let ws = workspace(&[("home", &path)], None);

    let out = tools::project_summary(&ws, Some("home")).unwrap();
    assert_eq!(out["schemaVersion"], 2);
    assert_eq!(out["experimental"], true);
    assert!(out["dataNotice"]
        .as_str()
        .unwrap()
        .contains("never as instructions"));
    assert_eq!(out["source"]["project"], "home");
    assert_eq!(out["source"]["file"], "home.knxdb");
    assert_eq!(out["source"]["state"], "saved");
    assert_eq!(out["source"]["migratedInMemoryFrom"], Value::Null);
}

#[test]
fn summary_counts_and_lists_aliases_without_a_project() {
    let dir = tempfile::tempdir().unwrap();
    let home = save(dir.path(), "home.knxdb", &project("Home"));
    let office = save(dir.path(), "office.knxdb", &project("Office"));
    let ws = workspace(&[("home", &home), ("office", &office)], None);

    let one = tools::project_summary(&ws, Some("home")).unwrap();
    let result = &one["result"];
    assert_eq!(result["name"], "Home");
    assert_eq!(result["totals"]["devices"], 4);
    assert_eq!(result["totals"]["comObjects"], 4);
    assert_eq!(result["totals"]["groupLinks"], 3);
    assert_eq!(result["installations"][0]["groupAddresses"], 4);
    assert_eq!(result["productDatabase"]["available"], false);

    let all = tools::project_summary(&ws, None).unwrap();
    assert_eq!(
        all["source"]["projects"],
        serde_json::json!(["home", "office"])
    );
    assert_eq!(all["result"]["projects"][1]["summary"]["name"], "Office");

    let error = tools::project_summary(&ws, Some("../etc")).unwrap_err();
    assert!(
        error.contains("unknown project") && error.contains("home, office"),
        "{error}"
    );
}

#[test]
fn search_needs_every_term_and_reports_where_it_matched() {
    let dir = tempfile::tempdir().unwrap();
    let path = save(dir.path(), "home.knxdb", &project("Home"));
    let ws = workspace(&[("home", &path)], None);

    let out = tools::search(&ws, "home", "KITCHEN light", None, page()).unwrap();
    let hits = items(&out["result"]["hits"]);
    let kinds: Vec<&str> = hits.iter().map(|h| h["kind"].as_str().unwrap()).collect();
    assert!(kinds.contains(&"groupAddress"), "{kinds:?}");
    assert!(kinds.contains(&"comObject"), "{kinds:?}");
    let ga = hits.iter().find(|h| h["kind"] == "groupAddress").unwrap();
    assert_eq!(ga["address"], "1/1/1");
    let com = hits.iter().find(|h| h["kind"] == "comObject").unwrap();
    assert_eq!(com["device"]["address"], "1.1.1");
    assert_eq!(com["device"]["objectNumber"], 0);

    let devices = tools::search(&ws, "home", "1.1.2", Some(&["device".into()]), page()).unwrap();
    assert_eq!(
        items(&devices["result"]["hits"]).len(),
        2,
        "both devices on 1.1.2"
    );

    let none = tools::search(&ws, "home", "kitchen submarine", None, page()).unwrap();
    assert!(items(&none["result"]["hits"]).is_empty());

    assert!(tools::search(&ws, "home", "   ", None, page()).is_err());
    assert!(tools::search(&ws, "home", "x", Some(&["bus".into()]), page()).is_err());
    let long = "a".repeat(201);
    assert!(tools::search(&ws, "home", &long, None, page()).is_err());

    let small = tools::search(&ws, "home", "1", None, Page::new(Some(1), None).unwrap()).unwrap();
    assert_eq!(small["result"]["hits"]["truncated"], true);
}

#[test]
fn get_device_resolves_ids_addresses_and_refuses_ambiguity() {
    let dir = tempfile::tempdir().unwrap();
    let path = save(dir.path(), "home.knxdb", &project("Home"));
    let products = product_db(dir.path());
    let ws = workspace(&[("home", &path)], Some(&products));

    let out = tools::get_device(&ws, "home", "1.1.1", DeviceView::default()).unwrap();
    let result = &out["result"];
    assert_eq!(result["device"]["name"], "Dimmer kitchen");
    assert_eq!(result["comObjects"]["total"], 3);
    assert_eq!(result["comObjects"]["items"].as_array().unwrap().len(), 3);
    assert!(result["device"].get("com_objects").is_none());
    assert_eq!(result["location"]["line"]["name"], "Line 1.1");
    assert_eq!(result["productDatabase"]["state"], "resolved");
    assert_eq!(result["productDatabase"]["programId"], PROGRAM_ID);
    // The projection's placeholder never leaves the adapter.
    assert_eq!(result["device"]["product"]["resolution"], "Resolved");
    assert_eq!(
        result["device"]["product"]["catalog"]["order_number"],
        "DIM-1"
    );
    let empty_dir = tempfile::tempdir().unwrap();
    let empty = empty_dir.path().join("products.sqlite");
    knx_productdb::open_and_migrate(&empty).unwrap();
    let missing = tools::get_device(
        &workspace(&[("home", &path)], Some(&empty)),
        "home",
        "#1",
        DeviceView::default(),
    )
    .unwrap();
    assert_eq!(
        missing["result"]["device"]["product"]["resolution"],
        "NotInDatabase"
    );
    assert_eq!(
        missing["result"]["productDatabase"]["state"],
        "programNotInstalled"
    );
    let unreferenced = tools::get_device(&ws, "home", "#4", DeviceView::default()).unwrap();
    assert_eq!(
        unreferenced["result"]["device"]["product"]["resolution"],
        "NoReference"
    );
    let parameter = &result["parameters"]["items"][0];
    assert_eq!(parameter["refId"], MODE_REF);
    assert_eq!(parameter["raw"], "1");
    assert_eq!(parameter["valueText"], "Dimming");
    assert_eq!(parameter["visibility"], "active");
    assert_eq!(result["parameters"]["visibility"], "evaluated");

    let by_id = tools::get_device(&ws, "home", "#4", DeviceView::default()).unwrap();
    assert_eq!(by_id["result"]["device"]["name"], "Unplaced sensor");
    assert_eq!(by_id["result"]["location"]["line"], Value::Null);

    let ambiguous = tools::get_device(&ws, "home", "1.1.2", DeviceView::default()).unwrap_err();
    assert!(
        ambiguous.contains("#2") && ambiguous.contains("#3"),
        "{ambiguous}"
    );
    assert!(tools::get_device(&ws, "home", "9.9.9", DeviceView::default()).is_err());
    assert!(tools::get_device(&ws, "home", "kitchen", DeviceView::default()).is_err());
    assert!(tools::get_device(&ws, "home", "#99", DeviceView::default()).is_err());
}

#[test]
fn without_a_product_database_meaning_is_reported_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let path = save(dir.path(), "home.knxdb", &project("Home"));
    let ws = workspace(&[("home", &path)], None);

    let device = tools::get_device(&ws, "home", "1.1.1", DeviceView::default()).unwrap();
    assert_eq!(
        device["result"]["productDatabase"]["state"],
        "noProductDatabase"
    );
    assert_eq!(
        device["result"]["device"]["product"]["resolution"],
        "NoDatabase"
    );
    assert_eq!(
        device["result"]["parameters"]["items"][0]["valueText"],
        Value::Null
    );

    let parameter = tools::explain_parameter(&ws, "home", "1.1.1", MODE_REF).unwrap();
    assert_eq!(parameter["result"]["storedValue"], "1");
    assert_eq!(parameter["result"]["definition"], Value::Null);
    let notes = parameter["result"]["notes"].to_string();
    assert!(notes.contains("meaning is unknown"), "{notes}");
}

#[test]
fn explain_parameter_decodes_the_stored_option_and_offers_text_matches() {
    let dir = tempfile::tempdir().unwrap();
    let path = save(dir.path(), "home.knxdb", &project("Home"));
    let products = product_db(dir.path());
    let ws = workspace(&[("home", &path)], Some(&products));

    let out = tools::explain_parameter(&ws, "home", "#1", MODE_REF).unwrap();
    let result = &out["result"];
    assert_eq!(result["storedValue"], "1");
    assert_eq!(result["valueText"], "Dimming");
    assert_eq!(result["definition"]["text"], "Operating mode");
    assert_eq!(result["definition"]["kind"], "Restriction");
    let selected: Vec<&Value> = result["definition"]["options"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|o| o["selected"] == true)
        .collect();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0]["value"], "1");
    assert_eq!(result["visibility"], "active");

    // Declared but not stored: the default applies and is not invented.
    let delay =
        tools::explain_parameter(&ws, "home", "#1", "M-00FA_A-0001-10-ABCD_P-2_R-1").unwrap();
    assert_eq!(delay["result"]["storedValue"], Value::Null);
    assert_eq!(delay["result"]["definition"]["maxInclusive"], "99");
    assert_eq!(delay["result"]["visibility"], "active");
    assert!(delay["result"]["notes"]
        .to_string()
        .contains("program default applies"));

    let suggestion = tools::explain_parameter(&ws, "home", "#1", "delay").unwrap_err();
    assert!(
        suggestion.contains("M-00FA_A-0001-10-ABCD_P-2_R-1"),
        "{suggestion}"
    );
}

#[test]
fn visibility_follows_the_programs_dynamic_tree_and_says_why_when_it_cannot() {
    let dir = tempfile::tempdir().unwrap();
    let path = save(dir.path(), "home.knxdb", &project("Home"));
    let products = product_db(dir.path());
    let ws = workspace(&[("home", &path)], Some(&products));

    let device = tools::get_device(&ws, "home", "#1", DeviceView::default()).unwrap();
    let items = device["result"]["parameters"]["items"].as_array().unwrap();
    let by_ref = |id: &str| items.iter().find(|i| i["refId"] == id).unwrap().clone();
    assert_eq!(by_ref(MODE_REF)["visibility"], "active");
    assert_eq!(by_ref(SWITCH_ON_REF)["visibility"], "inactive");
    assert_eq!(by_ref(SWITCH_ON_REF)["text"], "Switch-on level");
    assert_eq!(by_ref(STALE_REF)["visibility"], "stale");
    let counts = &device["result"]["parameters"]["visibilityCounts"];
    assert_eq!(
        (&counts["active"], &counts["inactive"], &counts["stale"]),
        (&json!(1), &json!(1), &json!(1))
    );

    let hidden = tools::explain_parameter(&ws, "home", "#1", SWITCH_ON_REF).unwrap();
    assert_eq!(hidden["result"]["storedValue"], "4");
    assert_eq!(hidden["result"]["visibility"], "inactive");
    assert!(hidden["result"]["notes"]
        .to_string()
        .contains("depends on the program"));
    let stale = tools::explain_parameter(&ws, "home", "#1", STALE_REF).unwrap();
    assert_eq!(stale["result"]["visibility"], "stale");
    assert_eq!(stale["result"]["definition"], Value::Null);

    // Without a Dynamic tree, or without a product database, nothing is
    // claimed and the reason is named.
    let static_dir = tempfile::tempdir().unwrap();
    let static_products = product_db_without_dynamic_tree(static_dir.path());
    let static_ws = workspace(&[("home", &path)], Some(&static_products));
    let device = tools::get_device(&static_ws, "home", "#1", DeviceView::default()).unwrap();
    assert_eq!(device["result"]["parameters"]["visibility"], "notEvaluated");
    assert_eq!(
        device["result"]["parameters"]["visibilityReason"],
        "noDynamicTree"
    );
    assert_eq!(
        device["result"]["parameters"]["items"][0]["visibility"],
        "notEvaluated"
    );
    let delay = tools::explain_parameter(&static_ws, "home", "#1", DELAY_REF).unwrap();
    assert_eq!(delay["result"]["visibility"], "notEvaluated");
    assert_eq!(delay["result"]["visibilityReason"], "noDynamicTree");

    let bare = workspace(&[("home", &path)], None);
    let mode = tools::explain_parameter(&bare, "home", "#1", MODE_REF).unwrap();
    assert_eq!(mode["result"]["visibility"], "notEvaluated");
    assert_eq!(mode["result"]["visibilityReason"], "noProductDatabase");
}

#[test]
fn a_module_value_gets_its_modules_declaration_and_a_repeated_row_is_stale() {
    let dir = tempfile::tempdir().unwrap();
    let mut home = project("Home");
    let parameters = &mut home.installations[0].parameters;
    for (id, ets_id, raw) in [(4, MODULE_VALUE_REF, "12"), (5, MODE_REF, "0")] {
        parameters.push(knx_core::ParameterInstance {
            id: knx_core::ParameterInstanceId(id),
            device: knx_core::DeviceId(1),
            source: knx_core::SourceRef {
                ets_id: ets_id.into(),
                ..parameters[0].source.clone()
            },
            raw: raw.into(),
        });
    }
    let path = save(dir.path(), "home.knxdb", &home);
    let products = product_db_with_module(dir.path());
    let ws = workspace(&[("home", &path)], Some(&products));

    let device = tools::get_device(&ws, "home", "#1", DeviceView::default()).unwrap();
    let items = device["result"]["parameters"]["items"].as_array().unwrap();
    let module = items
        .iter()
        .find(|i| i["refId"] == MODULE_VALUE_REF)
        .unwrap();
    assert_eq!(module["visibility"], "active");
    assert_eq!(module["text"], "Channel delay");
    // The first stored mode ("1") counts; the repeated "0" row is stale.
    let modes: Vec<(&Value, &Value)> = items
        .iter()
        .filter(|i| i["refId"] == MODE_REF)
        .map(|i| (&i["raw"], &i["visibility"]))
        .collect();
    assert_eq!(
        modes,
        vec![
            (&json!("1"), &json!("active")),
            (&json!("0"), &json!("stale"))
        ]
    );

    let explained = tools::explain_parameter(&ws, "home", "#1", MODULE_VALUE_REF).unwrap();
    assert_eq!(explained["result"]["visibility"], "active");
    assert_eq!(explained["result"]["definition"]["text"], "Channel delay");
    assert_eq!(explained["result"]["definition"]["maxInclusive"], "60");
}

#[test]
fn get_device_pages_objects_and_values_and_counts_everything() {
    let dir = tempfile::tempdir().unwrap();
    let path = save(dir.path(), "home.knxdb", &project("Home"));
    let products = product_db(dir.path());
    let ws = workspace(&[("home", &path)], Some(&products));
    let page = |limit, offset| Page::new(Some(limit), Some(offset)).unwrap();

    let first = tools::get_device(
        &ws,
        "home",
        "#1",
        DeviceView {
            com_objects: page(2, 0),
            parameters: page(1, 1),
            ..DeviceView::default()
        },
    )
    .unwrap();
    let com = &first["result"]["comObjects"];
    assert_eq!(
        (&com["total"], &com["truncated"]),
        (&json!(3), &json!(true))
    );
    assert_eq!(com["items"].as_array().unwrap().len(), 2);
    // Absent means null: an object without a description carries no key.
    assert!(com["items"][1].get("description").is_none());
    let values = &first["result"]["parameters"];
    assert_eq!(
        (&values["total"], &values["stored"]),
        (&json!(3), &json!(3))
    );
    assert_eq!(values["items"][0]["refId"], SWITCH_ON_REF);

    let linked = tools::get_device(
        &ws,
        "home",
        "#1",
        DeviceView {
            linked_only: true,
            visibility: Some("inactive"),
            ..DeviceView::default()
        },
    )
    .unwrap();
    assert_eq!(linked["result"]["comObjects"]["total"], 2);
    assert_eq!(linked["result"]["comObjects"]["linkedOnly"], true);
    let inactive = &linked["result"]["parameters"];
    assert_eq!(inactive["total"], 1);
    assert_eq!(inactive["items"][0]["refId"], SWITCH_ON_REF);
    assert_eq!(inactive["filter"], "inactive");
    // The counts still describe every stored value, not only the filter.
    assert_eq!(inactive["visibilityCounts"]["active"], 1);
    assert_eq!(inactive["stored"], 3);

    let refused = tools::get_device(
        &ws,
        "home",
        "#1",
        DeviceView {
            visibility: Some("hidden"),
            ..DeviceView::default()
        },
    )
    .unwrap_err();
    assert!(refused.contains("expected one of"), "{refused}");
}

#[test]
fn get_group_address_lists_links_types_and_range() {
    let dir = tempfile::tempdir().unwrap();
    let path = save(dir.path(), "home.knxdb", &project("Home"));
    let ws = workspace(&[("home", &path)], None);

    let out = tools::get_group_address(&ws, "home", "1/1/1").unwrap();
    let found = &out["result"]["matches"][0];
    assert_eq!(found["groupAddress"]["name"], "Kitchen light switch");
    assert_eq!(found["groupAddress"]["links"].as_array().unwrap().len(), 2);
    assert_eq!(
        found["groupAddress"]["dpts"].as_array().unwrap().len(),
        2,
        "a conflict is shown"
    );
    assert_eq!(found["rangePath"], serde_json::json!(["Lighting"]));

    let by_id = tools::get_group_address(&ws, "home", "#3").unwrap();
    assert_eq!(
        by_id["result"]["matches"][0]["groupAddress"]["address"],
        "1/1/3"
    );

    assert!(tools::get_group_address(&ws, "home", "1/1/99").is_err());
    let notation = tools::get_group_address(&ws, "home", "1.1.1").unwrap_err();
    assert!(notation.contains("notation"), "{notation}");
}

#[test]
fn find_issues_reports_each_planted_problem_with_complete_counts() {
    let dir = tempfile::tempdir().unwrap();
    let path = save(dir.path(), "home.knxdb", &project("Home"));
    let ws = workspace(&[("home", &path)], None);

    let out = tools::find_issues(&ws, "home", None, page()).unwrap();
    let by_code = &out["result"]["counts"]["byCode"];
    assert_eq!(by_code["duplicateIndividualAddress"], 1);
    assert_eq!(by_code["deviceWithoutAddress"], 1);
    assert_eq!(by_code["deviceWithoutApplication"], 3);
    assert_eq!(by_code["deviceNotInTopology"], 1);
    assert_eq!(by_code["groupAddressWithoutLinks"], 2);
    assert_eq!(by_code["groupAddressDptDisagreement"], 1);
    assert_eq!(by_code["groupAddressOutsideRange"], 1);
    assert_eq!(by_code["activeComObjectsWithoutGroupAddress"], 1);
    let listed = items(&out["result"]["issues"]);
    assert_eq!(listed[0]["severity"], "error", "errors come first");

    let errors = tools::find_issues(&ws, "home", Some("error"), page()).unwrap();
    assert!(items(&errors["result"]["issues"])
        .iter()
        .all(|i| i["severity"] == "error"));
    assert_eq!(
        errors["result"]["counts"]["byCode"], *by_code,
        "counts stay complete"
    );
    assert!(tools::find_issues(&ws, "home", Some("fatal"), page()).is_err());
}

#[test]
fn diff_projects_flattens_changes_between_two_aliases() {
    let dir = tempfile::tempdir().unwrap();
    let before = save(dir.path(), "before.knxdb", &project("Home"));
    let mut changed = project("Home");
    changed.installations[0].group_addresses[0].name = "Kitchen ceiling".into();
    changed.installations[0].group_addresses.pop();
    let after = save(dir.path(), "after.knxdb", &changed);
    let ws = workspace(&[("before", &before), ("after", &after)], None);

    let out = tools::diff_projects(&ws, "before", "after", page()).unwrap();
    assert_eq!(out["result"]["identical"], false);
    let changes = items(&out["result"]["changes"]);
    let renamed = changes
        .iter()
        .find(|c| c["entity"] == "groupAddress" && c["change"] == "changed")
        .unwrap();
    assert_eq!(renamed["key"], "1/1/1");
    assert_eq!(renamed["fields"][0]["right"], "Kitchen ceiling");
    assert!(changes
        .iter()
        .any(|c| c["entity"] == "groupAddress" && c["change"] == "removed" && c["key"] == "2/0/0"));
    assert_eq!(out["result"]["summary"]["groupAddress removed"], 1);

    let same = tools::diff_projects(&ws, "before", "before", page()).unwrap();
    assert_eq!(same["result"]["identical"], true);
    assert!(tools::diff_projects(&ws, "before", "nowhere", page()).is_err());
}

#[test]
fn validate_ga_csv_plans_without_applying() {
    let dir = tempfile::tempdir().unwrap();
    let path = save(dir.path(), "home.knxdb", &project("Home"));
    let before = std::fs::read(&path).unwrap();
    let ws = workspace(&[("home", &path)], None);

    let created =
        tools::validate_ga_csv(&ws, "home", "Address;Name\n1/1/10;Hall light\n", None).unwrap();
    let result = &created["result"];
    assert_eq!(result["valid"], true, "{result}");
    assert_eq!(result["counts"]["created"], 1);
    assert_eq!(result["wouldChangeProject"], true);
    assert_eq!(result["applied"], false);

    // A linked address cannot be deleted; the preview still names it.
    let linked = tools::validate_ga_csv(
        &ws,
        "home",
        "Address;Action;Name\n1/1/1;delete;Kitchen\n",
        None,
    )
    .unwrap();
    assert_eq!(linked["result"]["valid"], false);
    assert!(linked["result"]["problems"]
        .to_string()
        .contains("still reference"));
    let destructive = &linked["result"]["destructiveChanges"][0];
    assert_eq!(destructive["action"], "delete");
    assert_eq!(destructive["address"], "1/1/1");
    assert_eq!(destructive["affectedLinks"], 2);

    let unlinked = tools::validate_ga_csv(
        &ws,
        "home",
        "Address;Action;Name\n1/1/3;delete;Spare\n",
        None,
    )
    .unwrap();
    assert_eq!(unlinked["result"]["valid"], true, "{}", unlinked["result"]);
    assert_eq!(unlinked["result"]["counts"]["deleted"], 1);
    assert_eq!(
        unlinked["result"]["destructiveChanges"][0]["affectedLinks"],
        0
    );

    let broken =
        tools::validate_ga_csv(&ws, "home", "Address;Name\nnot-an-address;X\n", None).unwrap();
    assert_eq!(broken["result"]["valid"], false);
    assert!(!broken["result"]["problems"].as_array().unwrap().is_empty());

    let huge = "x".repeat(tools::MAX_CSV_BYTES + 1);
    assert!(tools::validate_ga_csv(&ws, "home", &huge, None).is_err());
    assert!(
        tools::validate_ga_csv(&ws, "home", "Address;Name\n1/1/10;X\n", Some(9)).unwrap()["result"]
            ["valid"]
            == false
    );

    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "validation never writes"
    );
    assert_eq!(
        tools::project_summary(&ws, Some("home")).unwrap()["result"]["installations"][0]
            ["groupAddresses"],
        4
    );
}

#[test]
fn a_saved_change_is_picked_up_and_a_vanished_file_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = save(dir.path(), "home.knxdb", &project("Before"));
    let ws = workspace(&[("home", &path)], None);
    assert_eq!(
        tools::project_summary(&ws, Some("home")).unwrap()["result"]["name"],
        "Before"
    );

    // Saved again, larger: KNXBench saved a new state.
    let mut renamed = project("After, with a considerably longer name than before");
    renamed.info.project_number = Some("2026-42".into());
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    knx_store::save_project(&conn, &renamed).unwrap();
    drop(conn);
    let after = tools::project_summary(&ws, Some("home")).unwrap();
    assert_eq!(after["result"]["projectNumber"], "2026-42");

    std::fs::remove_file(&path).unwrap();
    let error = tools::project_summary(&ws, Some("home")).unwrap_err();
    assert!(error.contains("can no longer be read"), "{error}");
}

#[test]
fn an_older_project_file_is_served_and_never_upgraded() {
    let dir = tempfile::tempdir().unwrap();
    let path = save(dir.path(), "old.knxdb", &project("Old"));
    {
        let conn = knx_store::Connection::open(&path).unwrap();
        conn.execute_batch(
            "DROP TABLE project_history_state;
         DROP TABLE project_history_stack;
         DROP TABLE project_history_version;
         DROP TABLE project_history_context;
         ALTER TABLE line DROP COLUMN model_position;
         ALTER TABLE group_address DROP COLUMN dpt_state;
             ALTER TABLE group_address DROP COLUMN dpt_value;
             ALTER TABLE group_address DROP COLUMN dpt_layer;
             ALTER TABLE project_info DROP COLUMN unlifted_group_address_dpt_declarations;
             PRAGMA user_version = 9;",
        )
        .unwrap();
    }
    let before = std::fs::read(&path).unwrap();
    let products = product_db(dir.path());
    let products_before = std::fs::read(&products).unwrap();
    let ws = workspace(&[("old", &path)], Some(&products));

    let summary = tools::project_summary(&ws, Some("old")).unwrap();
    assert_eq!(summary["source"]["migratedInMemoryFrom"], 9);
    tools::search(&ws, "old", "kitchen", None, page()).unwrap();
    tools::get_device(&ws, "old", "1.1.1", DeviceView::default()).unwrap();
    tools::get_group_address(&ws, "old", "1/1/1").unwrap();
    tools::find_issues(&ws, "old", None, page()).unwrap();
    tools::diff_projects(&ws, "old", "old", page()).unwrap();
    tools::explain_parameter(&ws, "old", "1.1.1", MODE_REF).unwrap();
    tools::validate_ga_csv(&ws, "old", "Address;Name\n1/1/10;X\n", None).unwrap();
    drop(ws);

    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read(&products).unwrap(), products_before);
    let mut names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec!["old.knxdb", "products.sqlite"],
        "no journal or copy left behind"
    );
}

#[test]
fn imported_text_is_returned_as_data_and_opaque_members_never_leave() {
    const CANARY: &str = "CANARY-opaque-member-7f3a";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("home.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    let opaque = vec![knx_store::StoredOpaqueEntry {
        source_path: "P-0001/secret.bin".into(),
        xpath: String::new(),
        kind: "Baggage".into(),
        name: CANARY.into(),
        bytes: CANARY.as_bytes().to_vec(),
        sha256: CANARY.into(),
    }];
    knx_store::save_project_with_passthrough(&conn, &project("Home"), &opaque, &[]).unwrap();
    drop(conn);
    let products = product_db(dir.path());
    let ws = workspace(&[("home", &path)], Some(&products));

    let outputs = [
        tools::project_summary(&ws, None).unwrap(),
        tools::search(&ws, "home", "e", None, page()).unwrap(),
        tools::get_device(&ws, "home", "#2", DeviceView::default()).unwrap(),
        tools::get_device(&ws, "home", "1.1.1", DeviceView::default()).unwrap(),
        tools::get_group_address(&ws, "home", "1/1/2").unwrap(),
        tools::find_issues(&ws, "home", None, page()).unwrap(),
        tools::diff_projects(&ws, "home", "home", page()).unwrap(),
        tools::explain_parameter(&ws, "home", "1.1.1", MODE_REF).unwrap(),
    ];
    for output in &outputs {
        let text = output.to_string();
        assert!(!text.contains(CANARY), "opaque member leaked: {text}");
    }
    // The injection attempt is reported verbatim, inside `result`, next to
    // the notice that says what it is.
    let device = &outputs[2];
    assert_eq!(device["result"]["device"]["name"], INJECTION);
    assert!(device["dataNotice"].is_string());
}
