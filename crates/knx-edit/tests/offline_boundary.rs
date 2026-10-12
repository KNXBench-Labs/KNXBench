//! Direct offline-library consumer, with no App or importer dependency.
use knx_core::{GroupAddressId, Language, Project};
use knx_edit::name_recipe::{
    preview_name_recipe, NameRecipeError, NameRecipeIntent, NameRecipeRevision,
};
use knx_store::project_history::NativeSnapshot;

#[test]
fn direct_consumer_refuses_missing_selection_without_changing_the_snapshot() {
    let state = NativeSnapshot {
        project: Project::new(Language("en".into())),
        opaque: vec![],
        manufacturer_refs: vec![],
    };
    let before = state.clone();
    let revision = NameRecipeRevision {
        project_incarnation: "direct-consumer".into(),
        generation: 0,
    };
    let result = preview_name_recipe(
        &state,
        &revision,
        NameRecipeIntent {
            ids: vec![GroupAddressId(1)],
            prefix: "${literal}".into(),
            suffix: " 🛠 ".into(),
        },
    );
    assert!(matches!(
        result,
        Err(NameRecipeError::MissingTarget(GroupAddressId(1)))
    ));
    assert_eq!(state, before);
}
