//! Independent synthetic caption-span adversarial controls; no proprietary inputs.
use super::*;
fn doc(body: &str) -> Vec<u8> {
    format!("<KNX xmlns=\"http://knx.org/xml/project/20\"><ManufacturerData><Manufacturer RefId=\"M\"><ApplicationPrograms>{body}</ApplicationPrograms></Manufacturer></ManufacturerData></KNX>").into_bytes()
}
fn owner(id: &str, value: &str) -> String {
    format!("<ApplicationProgram Id=\"{id}\" Name=\"{value}\"/>")
}
fn good(value: &str) -> Vec<u8> {
    doc(&owner("A", value))
}
#[test]
fn review_selected_owner_not_equal_value_decoys() {
    let old = doc(&format!(
        "<!-- Name=\"old\" --><?probe Name=\"old\"?>{}{}<Translation Text=\"old\"/>",
        owner("C", "old"),
        owner("A", "old")
    ));
    let new = doc(&format!(
        "<!-- Name=\"old\" --><?probe Name=\"old\"?>{}{}<Translation Text=\"old\"/>",
        owner("C", "old"),
        owner("A", "new longer")
    ));
    caption_value_only(&old, &new, "A").unwrap();
}
#[test]
fn review_wrong_owner_not_selected() {
    assert!(caption_value_only(&doc(&owner("C", "old")), &doc(&owner("C", "new")), "A").is_err());
}
#[test]
fn review_all_outside_prefix_bytes_exact() {
    let new = String::from_utf8(good("new"))
        .unwrap()
        .replace("RefId=\"M\"", "RefId=\"N\"");
    assert!(caption_value_only(&good("old"), new.as_bytes(), "A").is_err());
}
#[test]
fn review_all_outside_suffix_bytes_exact() {
    let old = doc(&format!("{}<Translation Text=\"old\"/>", owner("A", "old")));
    let new = doc(&format!(
        "{}<Translation Text=\"changed\"/>",
        owner("A", "new")
    ));
    assert!(caption_value_only(&old, &new, "A").is_err());
}
#[test]
fn review_comments_and_pi_cannot_supply_or_change_span() {
    for decoration in ["<!-- Name=\"new\" -->", "<?probe Name=\"new\"?>"] {
        let new = doc(&format!("{decoration}{}", owner("A", "old")));
        assert!(caption_value_only(&good("old"), &new, "A").is_err());
    }
}
#[test]
fn review_duplicate_selected_owner_and_attributes_refuse() {
    for body in [
        format!("{}{}", owner("A", "new"), owner("A", "other")),
        "<ApplicationProgram Id=\"A\" Id=\"C\" Name=\"new\"/>".into(),
        "<ApplicationProgram Id=\"A\" Name=\"new\" Name=\"other\"/>".into(),
    ] {
        assert!(caption_value_only(&good("old"), &doc(&body), "A").is_err());
    }
}
#[test]
fn review_absent_qualified_and_prefixed_targets_refuse() {
    for body in [
        "<ApplicationProgram Id=\"A\"/>",
        "<ApplicationProgram Name=\"new\"/>",
        "<ApplicationProgram Id=\"A\" x:Name=\"new\" xmlns:x=\"urn:x\"/>",
        "<x:ApplicationProgram xmlns:x=\"http://knx.org/xml/project/20\" Id=\"A\" Name=\"new\"/>",
    ] {
        assert!(caption_value_only(&good("old"), &doc(body), "A").is_err());
    }
}
#[test]
fn review_lexical_old_and_new_both_bounded() {
    for value in [
        "".to_string(),
        "x".repeat(1025),
        "&amp;".into(),
        "&#65;".into(),
        "\t".into(),
        "\r".into(),
        "\u{7f}".into(),
        "é".into(),
    ] {
        assert!(caption_value_only(&good("old"), &good(&value), "A").is_err());
        assert!(caption_value_only(&good(&value), &good("new"), "A").is_err());
    }
    caption_value_only(&good(&"x".repeat(1024)), &good("!"), "A").unwrap();
}
#[test]
fn review_input_and_event_budgets_refuse() {
    let mut huge = good("new");
    huge.resize(16 * 1024 * 1024 + 1, b' ');
    assert!(caption_value_only(&good("old"), &huge, "A").is_err());
    let many = doc(&format!(
        "{}{}",
        owner("A", "new"),
        "<!--x-->".repeat(250_000)
    ));
    assert!(caption_value_only(&good("old"), &many, "A").is_err());
}
#[test]
fn review_dtd_refuses() {
    let mut new = b"<!DOCTYPE KNX>".to_vec();
    new.extend(good("new"));
    assert!(caption_value_only(&good("old"), &new, "A").is_err());
}
