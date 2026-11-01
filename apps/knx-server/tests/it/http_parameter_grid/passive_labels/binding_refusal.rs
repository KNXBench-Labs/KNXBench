//! A refused passive-label binding is named by one panel diagnostic of its own.

use super::*;

const KIND: &str = "passiveLabelsUnbound";
/// The stored-tree drift mutation also used by the ProductDB binding tests.
const DRIFT: &str = "UPDATE dynamic_node SET position=position+100 WHERE program_id='A-FIX' AND module_def_id='' AND kind='Button'";
/// Changes the retained source bytes, so the hash check refuses them.
const CORRUPT: &str = "UPDATE source_file SET bytes=CAST(replace(CAST(bytes AS TEXT),'Fictional source label','Fictional source lab3l') AS BLOB)";
/// `PR-MODE` 0 closes the branch holding `B-ONLY`.
const OPEN: &str = r#"Text="Fictional selector" ParameterType="PT-MODE" Value="1""#;
const CLOSED: &str = r#"Text="Fictional selector" ParameterType="PT-MODE" Value="0""#;

fn unbound(dto: &Value) -> Vec<&Value> {
    dto["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["kind"] == KIND)
        .collect()
}

async fn refused(program: &str, mutation: Option<&str>) -> Value {
    let (_dir, state) = state_with(program, mutation);
    panel_at(&state, PARAMETERS).await
}

/// Exactly one program-level warning whose detail leads with `cause`.
fn assert_one(dto: &Value, cause: &str, reached: usize) {
    let found = unbound(dto);
    assert_eq!(found.len(), 1, "{dto}");
    let diagnostic = found[0];
    assert!(diagnostic["scope"].is_null(), "{diagnostic}");
    assert_eq!(diagnostic["severity"], "warning");
    assert_eq!(
        diagnostic["message"],
        "Source button labels of this program are not shown: the retained product source could not be matched to the installed catalogue."
    );
    let detail = diagnostic["detail"].as_str().unwrap();
    assert!(detail.starts_with(&format!("{cause}: ")), "{detail}");
    assert!(
        detail.ends_with(&format!(
            "{reached} reached Button(s) show no label (program 'A-FIX')."
        )),
        "{detail}"
    );
    assert_eq!(passive_ids(dto), Vec::<String>::new(), "no label at all");
}

#[tokio::test]
async fn a_bound_program_reports_no_binding_diagnostic() {
    let dto = panel(EN).await;
    assert_eq!(passive_ids(&dto), ["B-ONLY", "B-MIX"]);
    assert_eq!(unbound(&dto), Vec::<&Value>::new());
}

#[tokio::test]
async fn a_duplicate_button_id_is_named_and_the_generic_issue_stays() {
    let dto = refused(&EN.replace(r#"Id="B-MIX""#, r#"Id="B-ONLY""#), None).await;
    assert_one(&dto, "duplicateButtonId", 2);
    let section = device_section(&dto);
    // A Button-only block was an area only through its label, so it is gone;
    // the mixed grid keeps its generic issue for the unshown Button.
    assert!(layout(section, "PB-ONLY").is_none(), "{dto}");
    let mixed = layout(section, "PB-MIX").expect("the mixed grid stays");
    assert_eq!(
        mixed["issues"],
        json!([{ "kind": "unshownContent", "element": "Button", "reference": "B-ONLY", "cell": "1,2" }])
    );
}

#[tokio::test]
async fn an_unreadable_source_is_named() {
    assert_one(&refused(EN, Some(CORRUPT)).await, "sourceUnavailable", 2);
}

#[tokio::test]
async fn stored_structure_drift_is_named() {
    assert_one(&refused(EN, Some(DRIFT)).await, "structuralDrift", 2);
}

#[tokio::test]
async fn a_refusal_without_a_reached_button_stays_silent() {
    // Only `B-ONLY` remains; its branch decides whether anything is reached.
    let single = EN.replace(
        r#"<Button Id="B-MIX" Text="Fictional mixed label" EventHandlerOnline="ConnectionOriented" EventHandler="fictionalCallback" EventHandlerParameters="{ &quot;fixture&quot;: 7 }" Cell="1,2"/>"#,
        "",
    );
    assert_ne!(single, EN, "the fixture still declares B-MIX verbatim");
    assert_one(
        &refused(&single, Some(CORRUPT)).await,
        "sourceUnavailable",
        1,
    );
    let closed = refused(&single.replace(OPEN, CLOSED), Some(CORRUPT)).await;
    assert_eq!(unbound(&closed), Vec::<&Value>::new(), "{closed}");
}
