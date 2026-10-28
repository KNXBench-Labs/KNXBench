//! Template retention is scoped to explicit Background; old revision-1 evidence stays reproducible.
use super::{fixture, inactive_xml, policy, project_policy};
use knx_productdb::image::{
    build_experimental_system2_image, build_project_experimental_system2_image,
};

const OPTIONS: &str = r#"<Options DownloadInvisibleParameters="Background" />"#;

fn source(namespace: u8, options: &str) -> String {
    let xml = inactive_xml();
    assert_eq!(xml.matches(OPTIONS).count(), 1);
    xml.replace(OPTIONS, options)
        .replace("project/20", &format!("project/{namespace}"))
}

/// Refuses under revision 2 in both namespaces; `message` pins the refusal reason.
fn check_refusal(options: &str, message: &str) {
    for namespace in [20, 23] {
        let xml = source(namespace, options);
        let (_dir, conn, request) = fixture(&xml);
        let before = conn.total_changes();
        let mut bound = project_policy(&xml);
        bound.binding.schema_namespace = format!("http://knx.org/xml/project/{namespace}");
        let result = build_project_experimental_system2_image(&conn, &request, &bound);
        assert_eq!(conn.total_changes(), before, "read-only even on refusal");
        let error = result.expect_err("template retention must require explicit Background");
        assert!(
            error.to_string().contains(message),
            "namespace {namespace}, {options:?}: {error}"
        );
    }
}

const NOT_BACKGROUND: &str = "requires explicit DownloadInvisibleParameters=Background";

#[test]
fn revision2_refuses_default_value_instead_of_silently_retaining_template() {
    check_refusal(
        r#"<Options DownloadInvisibleParameters="DefaultValue" />"#,
        NOT_BACKGROUND,
    );
}

#[test]
fn revision2_refuses_an_absent_options_element() {
    check_refusal("", NOT_BACKGROUND);
}

#[test]
fn revision2_refuses_an_options_element_without_download_behavior() {
    check_refusal("<Options />", NOT_BACKGROUND);
}

/// Characterization: these values already refuse before any image interpretation.
#[test]
fn revision2_keeps_refusing_none_and_unknown_download_behaviors() {
    for value in ["None", "Never", "background", ""] {
        check_refusal(
            &format!(r#"<Options DownloadInvisibleParameters="{value}" />"#),
            "unrecognized DownloadInvisibleParameters value",
        );
    }
}

#[test]
fn revision2_explicit_background_retains_inactive_data_without_writing_to_database() {
    for namespace in [20, 23] {
        let xml = source(namespace, OPTIONS);
        let (_dir, conn, request) = fixture(&xml);
        let before = conn.total_changes();
        let mut bound = project_policy(&xml);
        bound.binding.schema_namespace = format!("http://knx.org/xml/project/{namespace}");
        let image = build_project_experimental_system2_image(&conn, &request, &bound).unwrap();
        assert_eq!(
            &image.image.segment("PAR").unwrap().octets[..4],
            &[0x12, 0x34, 0, 0]
        );
        assert_eq!(conn.total_changes(), before);
    }
}

/// Revision 1 (namespace 20 only) keeps reproducing superseded defaults evidence.
#[test]
fn revision1_default_value_and_omission_keep_historical_images_reproducible() {
    for options in [
        "",
        "<Options />",
        r#"<Options DownloadInvisibleParameters="DefaultValue" />"#,
    ] {
        let xml = source(20, options);
        let (_dir, conn, request) = fixture(&xml);
        let before = conn.total_changes();
        let image = build_experimental_system2_image(&conn, &request, &policy(&xml)).unwrap();
        assert_eq!(
            &image.image.segment("PAR").unwrap().octets[..4],
            &[0x12, 0x34, 1, 2]
        );
        assert_eq!(conn.total_changes(), before);
    }
}
