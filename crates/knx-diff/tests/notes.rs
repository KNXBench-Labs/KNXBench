//! Verifies native authored note changes without guessing imported note identities.
use knx_core::note::{ProjectNote, ProjectNoteTarget as Target};
use knx_core::{Language, Project, ProjectNoteId};
use knx_diff::diff_projects;
fn note(id: u32, text: &str) -> ProjectNote {
    ProjectNote::new(
        ProjectNoteId(id),
        Target::Project,
        "why".into(),
        text.into(),
    )
    .unwrap()
}
#[test]
fn authored_note_addition_removal_and_text_changes_are_visible() {
    let left = Project::new(Language("en".into()));
    let mut right = left.clone();
    right.notes.push(note(1, "literal Grüße🙂"));
    let diff = diff_projects(&left, &right);
    assert!(!diff.is_empty());
    assert_eq!(diff.info_changes[0].field, "notes");
    assert!(diff.info_changes[0].right.contains("literal Grüße🙂"));
    assert!(!diff_projects(&right, &left).is_empty());
    let mut changed = right.clone();
    changed.notes[0] = note(1, "changed");
    assert!(!diff_projects(&right, &changed).is_empty());
}
#[test]
fn note_order_target_and_documentation_flag_remain_separate_native_changes() {
    let mut left = Project::new(Language("en".into()));
    left.notes = vec![note(1, "one"), note(2, "two")];
    let mut reordered = left.clone();
    reordered.notes.reverse();
    assert!(!diff_projects(&left, &reordered).is_empty());
    let mut opted = left.clone();
    opted.notes[0] = opted.notes[0].clone().with_documentation(true);
    assert!(!diff_projects(&left, &opted).is_empty());
    let mut retargeted = left.clone();
    retargeted.notes[0] = ProjectNote::new(
        ProjectNoteId(1),
        Target::Installation(knx_core::InstallationId(0)),
        "why".into(),
        "one".into(),
    )
    .unwrap();
    assert!(!diff_projects(&left, &retargeted).is_empty());
}
#[test]
fn equal_note_state_and_allocator_only_changes_do_not_make_user_content_diff() {
    let mut left = Project::new(Language("en".into()));
    left.notes.push(note(1, "same"));
    let mut right = left.clone();
    right.ids = right.ids.with_project_note_count(9);
    assert!(diff_projects(&left, &right).is_empty());
}
