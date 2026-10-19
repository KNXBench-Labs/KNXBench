//! Verifies normalized instruction integrity with synthetic data.
use knx_core::instruction::*;

fn run(text: &str, bold: bool) -> InstructionRun {
    InstructionRun {
        text: text.into(),
        style: InstructionStyle {
            bold,
            ..Default::default()
        },
    }
}
#[test]
fn empty_document_is_a_present_value_distinct_from_no_override() {
    let empty = InstructionDocument::new(vec![]).unwrap();
    assert_eq!(empty, InstructionDocument::default());
    assert_eq!(empty.runs(), &[]);
    assert_eq!(empty.plain_text(), "");
    assert_ne!(Some(empty), None);
}
#[test]
fn literal_unicode_whitespace_and_markup_remain_exact() {
    let original = "  Grüße 😀\r\n\t<script>literal</script>\\{}\0";
    let doc = InstructionDocument::new(vec![run(original, false)]).unwrap();
    assert_eq!(doc.plain_text(), original);
    assert_eq!(doc.runs()[0].text, original);
}
#[test]
fn empty_runs_are_rejected_not_silently_removed() {
    assert_eq!(
        InstructionDocument::new(vec![run("", false)]),
        Err(InstructionError::EmptyRun)
    );
}
#[test]
fn adjacent_equal_styles_are_not_accepted_as_distinct_semantic_states() {
    assert_eq!(
        InstructionDocument::new(vec![run("a", false), run("b", false)]),
        Err(InstructionError::AdjacentStyle)
    );
    let doc = InstructionDocument::new(vec![run("a", false), run("b", true)]).unwrap();
    assert_eq!(doc.plain_text(), "ab");
    assert_eq!(doc.runs().len(), 2);
}
#[test]
fn byte_limit_is_utf8_bytes_not_characters() {
    let text = "😀".repeat(MAX_DOCUMENT_BYTES / 4);
    assert_eq!(
        InstructionDocument::new(vec![run(&text, false)])
            .unwrap()
            .plain_text(),
        text
    );
    assert_eq!(
        InstructionDocument::new(vec![run(&(text + "a"), false)]),
        Err(InstructionError::ByteLimit)
    );
}
#[test]
fn run_budget_refuses_before_traversing_adversarial_input() {
    let accepted: Vec<_> = (0..MAX_DOCUMENT_RUNS)
        .map(|index| run("x", index % 2 == 0))
        .collect();
    assert_eq!(
        InstructionDocument::new(accepted).unwrap().runs().len(),
        MAX_DOCUMENT_RUNS
    );
    let oversized = vec![run("", false); MAX_DOCUMENT_RUNS + 1];
    assert_eq!(
        InstructionDocument::new(oversized),
        Err(InstructionError::RunLimit)
    );
}
#[test]
fn all_supported_style_combinations_are_exact_and_ordered() {
    let runs: Vec<_> = (0..8)
        .map(|mask| InstructionRun {
            text: format!("{mask}"),
            style: InstructionStyle {
                bold: mask & 1 != 0,
                italic: mask & 2 != 0,
                underline: mask & 4 != 0,
            },
        })
        .collect();
    let doc = InstructionDocument::new(runs.clone()).unwrap();
    assert_eq!(doc.runs(), runs);
    assert_eq!(doc.plain_text(), "01234567");
}
