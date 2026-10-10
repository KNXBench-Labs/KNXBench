//! KL-106 audit: retained ETS source evidence never reaches an MCP tool response.
//!
//! The planted values are synthetic (RFC 5737 addresses, a locally
//! administered MAC, `.invalid` host names) and stand in for the kinds of
//! subtree an ETS import keeps verbatim: interface settings, IP
//! configuration, additional addresses and project traces.

mod common;

use common::*;
use knx_mcp::tools::{self, DeviceView, Page};

const PLANTED: &[&str] = &[
    "tunnel-gw-q7.example.invalid",
    "198.51.100.23",
    "192.0.2.77",
    "02:00:5e:10:00:01",
    "planted-user-zq",
    "planted extra address",
];

fn retained(name: &str, body: &str) -> knx_store::StoredOpaqueEntry {
    knx_store::StoredOpaqueEntry {
        source_path: "P-0001/0.xml".into(),
        xpath: format!("/KNX/Project/{name}"),
        kind: "RetainedElement".into(),
        name: name.into(),
        bytes: body.as_bytes().to_vec(),
        sha256: "0".repeat(64),
    }
}

#[test]
fn no_tool_response_carries_retained_source_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = save(dir.path(), "home.knxdb", &project("Home"));
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::insert_opaque(
        &conn,
        &[
            retained(
                "BusAccess",
                r#"<BusAccess Parameter="Target=tunnel-gw-q7.example.invalid;Peer=198.51.100.23"/>"#,
            ),
            retained(
                "IPConfig",
                r#"<IPConfig IPAddress="192.0.2.77" MACAddress="02:00:5e:10:00:01"/>"#,
            ),
            retained(
                "AdditionalAddresses",
                r#"<AdditionalAddresses><Address Name="planted extra address"/></AdditionalAddresses>"#,
            ),
            retained(
                "ProjectTraces",
                r#"<ProjectTraces><ProjectTrace UserName="planted-user-zq"/></ProjectTraces>"#,
            ),
        ],
    )
    .unwrap();
    drop(conn);
    let ws = workspace(&[("home", &path)], None);
    let page = || Page::new(Some(500), None).unwrap();

    let mut responses = vec![
        tools::project_summary(&ws, Some("home")).unwrap(),
        tools::find_issues(&ws, "home", None, page()).unwrap(),
        tools::diff_projects(&ws, "home", "home", page()).unwrap(),
        tools::get_group_address(&ws, "home", "1/1/1").unwrap(),
    ];
    for device in ["#1", "#2", "#3", "#4"] {
        responses.push(tools::get_device(&ws, "home", device, DeviceView::default()).unwrap());
    }
    for planted in PLANTED {
        // The response echoes the query, so only its hits are inspected.
        let search = tools::search(&ws, "home", planted, None, page()).unwrap();
        assert_eq!(search["result"]["hits"]["total"], 0, "{planted}: {search}");
        responses.push(search["result"]["hits"].clone());
    }
    for response in &responses {
        let text = response.to_string();
        for planted in PLANTED {
            assert!(!text.contains(planted), "{planted} leaked: {text}");
        }
    }
}
