//! F02/F03: a module Button keeps its own translation and provenance, per exact scope.

use super::*;

const GRID: &str = r#"<Rows><Row Id="R"/></Rows><Columns><Column Id="C1" Width="50%"/><Column Id="C2" Width="50%"/></Columns>"#;

/// The program Button and both module Buttons sit at the same node number of their own
/// tree; `MD-1` is instantiated twice, `MD-2` once.
fn program() -> String {
    let defs = format!(
        r#"<ModuleDefs><ModuleDef Id="MD-1" Name="First"><Static/><Dynamic><ParameterBlock Id="MD-1_PB" Layout="Grid">{GRID}<Button Id="MD-1_B" Text="First source" Cell="1,2"/></ParameterBlock></Dynamic></ModuleDef><ModuleDef Id="MD-2" Name="Second"><Static/><Dynamic><ParameterBlock Id="MD-2_PB" Layout="Grid">{GRID}<Button Id="MD-2_B" Text="Second source" Cell="1,2"/></ParameterBlock></Dynamic></ModuleDef></ModuleDefs>"#
    );
    let modules = r#"<Module Id="MOD-1_M-1" RefId="MD-1"/><Module Id="MOD-1_M-2" RefId="MD-1"/><Module Id="MOD-2_M-3" RefId="MD-2"/>"#;
    // The program's own tree holds one `Dynamic`; the module trees are added afterwards.
    ONLY.replacen("</Dynamic>", &format!("{modules}</Dynamic>{defs}"), 1)
}

const TRANSLATE: &str =
    "INSERT INTO translation (scope,scope_id,language,ref_id,attribute_name,text) VALUES
 ('Program','A-FIX','de-DE','B-ONLY','Text','Programm übersetzt'),
 ('Program','A-FIX','de-DE','MD-1_B','Text','Erstes Modul übersetzt'),
 ('Program','A-FIX','de-DE','MD-2_B','Text','Zweites Modul übersetzt')";

fn buttons(dto: &Value) -> Vec<(Value, String, Value)> {
    dto["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|section| {
            section["passiveLabels"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|l| l["kind"] == "buttonLiteral")
                .map(|l| {
                    (
                        section["scope"].clone(),
                        l["text"].as_str().unwrap().to_owned(),
                        l.get("textFill").cloned().unwrap_or(Value::Null),
                    )
                })
        })
        .collect()
}

#[tokio::test]
async fn each_module_button_keeps_its_own_translation_without_a_fill() {
    let (_dir, state) = state_with(&program(), Some(TRANSLATE));
    let dto = panel_at(&state, &format!("{PARAMETERS}?language=de-DE")).await;
    let found = buttons(&dto);
    let texts: Vec<&str> = found.iter().map(|(_, t, _)| t.as_str()).collect();
    assert_eq!(
        texts,
        [
            "Programm übersetzt",
            "Erstes Modul übersetzt",
            "Erstes Modul übersetzt",
            "Zweites Modul übersetzt"
        ],
        "{dto}"
    );
    // The two instances of one definition are two distinct scopes.
    assert_ne!(found[1].0, found[2].0);
    // A plain translation substitutes no stored value: no fill is claimed.
    assert!(found.iter().all(|(_, _, fill)| fill.is_null()), "{found:?}");
}

#[tokio::test]
async fn an_untranslated_module_button_never_borrows_the_program_buttons_translation() {
    let only_program =
        "INSERT INTO translation (scope,scope_id,language,ref_id,attribute_name,text) VALUES
 ('Program','A-FIX','de-DE','B-ONLY','Text','Programm übersetzt')";
    let (_dir, state) = state_with(&program(), Some(only_program));
    let dto = panel_at(&state, &format!("{PARAMETERS}?language=de-DE")).await;
    let texts: Vec<String> = buttons(&dto).into_iter().map(|(_, t, _)| t).collect();
    assert_eq!(
        texts,
        [
            "Programm übersetzt",
            "First source",
            "First source",
            "Second source"
        ],
        "{dto}"
    );
}

/// WP-48.3 lead 4: a Button row ingested before `@Id` was stored has no `element_id`. Its
/// literal label is still bound from the retained source (no re-import needed); only the
/// requested-language overlay cannot find it and falls back to the literal.
#[tokio::test]
async fn a_historical_button_row_without_element_id_keeps_its_literal_label() {
    let (_dir, state) = state_with_all(
        ONLY,
        &[
            "INSERT INTO translation (scope,scope_id,language,ref_id,attribute_name,text) VALUES
 ('Program','A-FIX','de-DE','B-ONLY','Text','Programm übersetzt')",
            "UPDATE dynamic_node SET element_id=NULL WHERE kind='Button'",
        ],
    );
    let dto = panel_at(&state, &format!("{PARAMETERS}?language=de-DE")).await;
    let found = buttons(&dto);
    assert_eq!(found.len(), 1, "{dto}");
    assert_eq!(found[0].1, "Fictional source label");
    assert!(found[0].2.is_null());
}
