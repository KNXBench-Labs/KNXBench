//! Source-to-public declaration identity assertions for the final candidate.
use serde_json::Value;

// Eight producer-recognized declarations: collision at4, orphan at8. No
// manufacturer declaration participates in this internal encounter numbering.
pub(super) const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData>
<Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers>
<SpaceUsages><SpaceUsage Id="SU-1"/></SpaceUsages>
<FunctionTypes><FunctionType Id="FT-private"/><FunctionType Id="FT-2"/>
<FunctionType Id="FT-private"/><FunctionType Id="FT-3"/><FunctionType Id="FT-4"/>
<FunctionType Id="FT-5"/><FunctionPoint Id="FP-private"/></FunctionTypes>
</MasterData></KNX>"#;

pub(super) fn assert_public(rows: &[Value]) {
    let mut identities: Vec<_> = rows
        .iter()
        .filter(|r| {
            r["kind"] == "diagnostic"
                && matches!(
                    r["diagnosticKind"].as_str(),
                    Some("master-identity-collision" | "malformed-master-declaration")
                )
        })
        .map(|r| {
            assert_eq!(r["packageKey"], "0");
            assert_eq!(r["archivePath"], "knx_master.xml");
            assert_eq!(r["occurrences"], "1");
            assert!(r["detail"].is_null());
            assert_eq!(r["withheld"].as_array().unwrap().len(), 1);
            assert_eq!(r["withheld"][0]["field"], "detail");
            assert_eq!(r["withheld"][0]["reason"], "untrustedFreeText");
            (
                r["diagnosticKind"].as_str().unwrap(),
                r["xmlPath"].as_str().unwrap(),
            )
        })
        .collect();
    identities.sort();
    assert_eq!(
        identities,
        vec![
            (
                "malformed-master-declaration",
                "/KNX/MasterData/FunctionTypes/FunctionPoint[@master-ordinal=8]"
            ),
            (
                "master-identity-collision",
                "/KNX/MasterData/FunctionTypes/FunctionType[@master-ordinal=4]"
            ),
        ]
    );
    let public = serde_json::to_string(rows).unwrap();
    assert!(!public.contains("FT-private"));
    assert!(!public.contains("FP-private"));
}

pub(super) fn assert_raw_unchanged(directory: &std::path::Path) {
    for stem in ["preview", "result"] {
        let raw = std::fs::read_to_string(directory.join(format!("{stem}.raw"))).unwrap();
        let public = std::fs::read_to_string(directory.join(format!("{stem}.public"))).unwrap();
        let raw: Vec<Value> = raw
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        let public: Vec<Value> = public
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(raw.len(), public.len());
        assert_public(&public);
        for (raw, public) in raw
            .iter()
            .zip(&public)
            .filter(|(r, _)| r["kind"] == "diagnostic")
        {
            assert_eq!(raw["ordinal"], public["ordinal"]);
            for key in [
                "kind",
                "packageKey",
                "diagnosticKind",
                "archivePath",
                "xmlPath",
                "occurrences",
            ] {
                assert_eq!(raw[key], public[key]);
            }
            let detail = raw["detail"].as_str().unwrap();
            assert_eq!(
                public["withheld"][0]["rawUtf8Bytes"],
                detail.len().to_string()
            );
            assert!(public["detail"].is_null());
            if raw["diagnosticKind"] == "master-identity-collision" {
                assert!(detail.starts_with(
                    "function_type Some(\"FT-private\"): identity collision; source "
                ));
                assert!(detail.contains(" declaration 4 bytes ["));
                assert!(detail.contains("; kept source "));
                assert!(detail.ends_with(" declaration 2"));
            } else if raw["diagnosticKind"] == "malformed-master-declaration" {
                assert!(detail
                    .starts_with("function_point Some(\"FP-private\"): orphan parent; source "));
                assert!(detail.contains(" declaration 8 bytes ["));
            }
        }
    }
}
