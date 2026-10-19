//! Verifies normalized instruction integrity with synthetic data.
#[path = "../../src/testutil.rs"]
#[allow(dead_code)]
mod testutil;
use knx_core::instruction::{InstructionDocument, InstructionRun, InstructionStyle};
use knx_diff::{diff_projects, FieldDiff};
fn document(text: &str, bold: bool) -> InstructionDocument {
    InstructionDocument::new(vec![InstructionRun {
        text: text.into(),
        style: InstructionStyle {
            bold,
            ..Default::default()
        },
    }])
    .unwrap()
}
#[test]
fn normalized_instruction_edits_are_visible_without_claiming_source_changes() {
    let mut left = testutil::project();
    let id = testutil::add_device(&mut left, 1, None);
    let mut right = left.clone();
    right
        .instructions
        .insert(id, document("edited Grüße", false));
    let difference = diff_projects(&left, &right);
    assert!(
        !difference.is_empty(),
        "normalized instructions must not be invisible to native comparison"
    );
    let change = &difference.installations[0].devices.changed[0];
    assert_eq!(change.changed_fields, vec!["instructions"]);
    let values = change.left.field_changes(&change.right);
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].field, "instructions");
    assert!(values[0].right.contains("edited Grüße"));
    assert_eq!(
        left.devices.get(id).unwrap().source,
        right.devices.get(id).unwrap().source
    );
}
#[test]
fn absent_and_explicit_empty_are_distinct_comparison_states() {
    let mut left = testutil::project();
    let id = testutil::add_device(&mut left, 1, None);
    let mut right = left.clone();
    right
        .instructions
        .insert(id, InstructionDocument::default());
    assert!(!diff_projects(&left, &right).is_empty());
}
#[test]
fn style_only_edits_are_not_collapsed_to_plain_text() {
    let mut left = testutil::project();
    let id = testutil::add_device(&mut left, 1, None);
    left.instructions.insert(id, document("same text", false));
    let mut right = left.clone();
    right.instructions.insert(id, document("same text", true));
    assert!(!diff_projects(&left, &right).is_empty());
}
#[test]
fn equal_normalized_documents_do_not_create_a_diff() {
    let mut left = testutil::project();
    let id = testutil::add_device(&mut left, 1, None);
    left.instructions.insert(id, document("same text", true));
    assert!(diff_projects(&left, &left.clone()).is_empty());
}
