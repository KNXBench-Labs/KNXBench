//! Verifies explicit note documentation inclusion with literal escaped text.
use knx_core::note::{ProjectNote, ProjectNoteTarget};
use knx_core::{Language, Project, ProjectNoteId};
use knx_report::{render_html, ReportLanguage, ReportOptions};
fn options() -> ReportOptions {
    ReportOptions::new(
        chrono::DateTime::parse_from_rfc3339("2026-10-11T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
    )
}
fn note(id: u32, title: &str, body: &str) -> ProjectNote {
    ProjectNote::new(
        ProjectNoteId(id),
        ProjectNoteTarget::Project,
        title.into(),
        body.into(),
    )
    .unwrap()
}
#[test]
fn unselected_scratch_notes_do_not_leak_into_any_report_output() {
    let mut project = Project::new(Language("en".into()));
    let before = render_html(&project, &options());
    project
        .notes
        .push(note(1, "private-title-marker", "private-body-marker"));
    let after = render_html(&project, &options());
    assert_eq!(after.html, before.html);
    assert!(!after.html.contains("private-title-marker"));
    assert_eq!(
        after
            .warnings
            .iter()
            .map(|w| (&w.location, &w.detail))
            .collect::<Vec<_>>(),
        before
            .warnings
            .iter()
            .map(|w| (&w.location, &w.detail))
            .collect::<Vec<_>>()
    );
}
#[test]
fn selected_notes_keep_order_linebreaks_and_are_escaped_not_executed() {
    let mut project = Project::new(Language("en".into()));
    project.notes.push(
        note(
            9,
            "First <script>",
            "Literal\r\n<img src=\"https://untrusted.invalid/never\"> & Grüße🙂",
        )
        .with_documentation(true),
    );
    project
        .notes
        .push(note(2, "Second", "after").with_documentation(true));
    project
        .notes
        .push(note(1, "private-title-marker", "private-body-marker"));
    let mut options = options();
    options.language = ReportLanguage::German;
    let report = render_html(&project, &options);
    assert!(report.html.contains("Projektnotizen"));
    assert!(report.html.contains("First &lt;script&gt;"));
    assert!(report.html.contains("Literal\r\n&lt;img"));
    assert!(report.html.contains("&amp; Grüße🙂"));
    assert!(!report.html.contains("<img"));
    assert!(!report.html.contains("private-body-marker"));
    assert!(
        report.html.find("First &lt;script&gt;").unwrap() < report.html.find("Second").unwrap()
    );
    assert!(report.html.contains("white-space:pre-wrap"));
    assert_eq!(report.html, render_html(&project, &options).html);
}
