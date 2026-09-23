//! Behavioral coverage for documentation composition at the product-data boundary.

use chrono::{TimeZone, Utc};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, ModuleInstance, ModuleInstanceId, ParameterInstance, ParameterInstanceId, Project,
    SourceRef, Topology,
};
use knx_report::{render_html, ReportLanguage, ReportSection};

fn source(id: &str) -> SourceRef {
    SourceRef {
        path: "fixture.xml".into(),
        ets_id: id.into(),
    }
}

fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.info.name = "Composition fixture".into();
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "Installation".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project
}

fn add_device(project: &mut Project, id: u32, product_ref: &str, program_ref: &str) {
    project.devices.insert(DeviceInstance {
        id: DeviceId(id),
        source: source(&format!("D-{id}")),
        name: format!("Device {id}"),
        description: None,
        address: None,
        product_ref: product_ref.into(),
        program_ref: program_ref.into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
}

fn add_parameter(project: &mut Project, id: u32, device: u32, reference: &str, raw: &str) {
    project.installations[0].parameters.push(ParameterInstance {
        id: ParameterInstanceId(id),
        device: DeviceId(device),
        source: source(reference),
        raw: raw.into(),
    });
}

fn add_module_argument(project: &mut Project, id: u32, device: u32, reference: &str, raw: &str) {
    project.devices.insert_module_instance(ModuleInstance {
        id: ModuleInstanceId(id),
        device: DeviceId(device),
        source: source(&format!("MODULE-{id}")),
        repeat_index: "1x1".into(),
        instance_ets_id: format!("MODULE-{id}_MI-1"),
        arguments: vec![(source(reference), raw.into())],
    });
}

fn render(project: &Project, products: &knx_productdb::Connection) -> knx_report::HtmlReport {
    let options = knx_app::documentation::report_options(
        project,
        Some(products),
        Utc.with_ymd_and_hms(2026, 9, 23, 12, 0, 0).unwrap(),
        ReportLanguage::English,
        [ReportSection::Devices].into_iter().collect(),
    );
    render_html(project, &options)
}

fn article(html: &str, device: u32) -> &str {
    let start = html
        .find(&format!("<article id=\"device-{device}\">"))
        .unwrap();
    let rest = &html[start..];
    let end = rest.find("</article>").unwrap() + "</article>".len();
    &rest[..end]
}

#[test]
fn mismatched_product_and_program_never_cross_attribute_another_devices_program_data() {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    products
        .execute_batch(
            "INSERT INTO manufacturer (id, name) VALUES ('M', 'Maker');
             INSERT INTO hardware (id, manufacturer_id, name, source_sha256)
               VALUES ('HW-A', 'M', 'Hardware A', 'x'), ('HW-B', 'M', 'Hardware B', 'x');
             INSERT INTO product (id, manufacturer_id, hardware_id, text, source_sha256)
               VALUES ('P-A', 'M', 'HW-A', 'Product A', 'x'),
                      ('P-B', 'M', 'HW-B', 'Product B', 'x');
             INSERT INTO application_program (id, manufacturer_id, name, source_sha256)
               VALUES ('APP-A', 'M', 'Program A', 'x'),
                      ('APP-B', 'M', 'Program B', 'x');
             INSERT INTO hardware2program
               (id, manufacturer_id, hardware_id, application_program_ref, source_sha256)
               VALUES ('H2P-A', 'M', 'HW-A', 'APP-A', 'x'),
                      ('H2P-B', 'M', 'HW-B', 'APP-B', 'x');
             INSERT INTO parameter_type (program_id, id, kind)
               VALUES ('APP-B', 'PT-B', 'Restriction');
             INSERT INTO parameter_type_enum
               (program_id, parameter_type_id, id, value, text)
               VALUES ('APP-B', 'PT-B', 'E-B', '1', 'Choice B');
             INSERT INTO parameter (program_id, id, name, text, parameter_type_id)
               VALUES ('APP-B', 'P-B-DECL', 'Parameter B name', 'Parameter B label', 'PT-B');
             INSERT INTO parameter_ref (program_id, id, parameter_id)
               VALUES ('APP-B', 'PR-B', 'P-B-DECL');",
        )
        .unwrap();

    let mut project = project();
    add_device(&mut project, 1, "P-A", "H2P-B");
    add_device(&mut project, 2, "P-B", "H2P-B");
    add_parameter(&mut project, 1, 1, "PR-B", "1");
    add_parameter(&mut project, 2, 2, "PR-B", "1");

    let report = render(&project, &products);
    let first = article(&report.html, 1);
    let second = article(&report.html, 2);

    assert!(first.contains("P-A"), "{first}");
    assert!(first.contains("H2P-B"), "{first}");
    assert!(!first.contains("Program B"), "{first}");
    assert!(!first.contains("Parameter B label"), "{first}");
    assert!(!first.contains("Choice B"), "{first}");
    assert!(second.contains("Program B"), "{second}");
    assert!(second.contains("Parameter B label"), "{second}");
    assert!(second.contains("Choice B"), "{second}");
    assert!(report.warnings.iter().any(|warning| {
        warning.location == "device 1"
            && warning.detail.contains("do not share hardware")
            && warning.detail.contains("P-A")
            && warning.detail.contains("H2P-B")
    }));
}

#[test]
fn blank_identity_parameter_and_module_argument_names_fall_back_to_raw_references() {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    products
        .execute_batch(
            "INSERT INTO manufacturer (id, name) VALUES ('M-blank', '   ');
             INSERT INTO hardware (id, manufacturer_id, name, source_sha256)
               VALUES ('HW-blank', 'M-blank', ' ', 'x');
             INSERT INTO product (id, manufacturer_id, hardware_id, text, source_sha256)
               VALUES ('P-blank', 'M-blank', 'HW-blank', '\n\t', 'x');
             INSERT INTO application_program (id, manufacturer_id, name, source_sha256)
               VALUES ('APP-blank', 'M-blank', '  ', 'x');
             INSERT INTO hardware2program
               (id, manufacturer_id, hardware_id, application_program_ref, source_sha256)
               VALUES ('H2P-blank', 'M-blank', 'HW-blank', 'APP-blank', 'x');
             INSERT INTO parameter_type (program_id, id, kind)
               VALUES ('APP-blank', 'PT-blank', 'Restriction');
             INSERT INTO parameter_type_enum
               (program_id, parameter_type_id, id, value, text)
               VALUES ('APP-blank', 'PT-blank', 'E-blank', '1', 'One');
             INSERT INTO parameter (program_id, id, name, text, parameter_type_id)
               VALUES ('APP-blank', 'P-blank-decl', ' ', '\t', 'PT-blank');
             INSERT INTO parameter_ref (program_id, id, parameter_id)
               VALUES ('APP-blank', 'PR-blank', 'P-blank-decl');
             INSERT INTO module_def_argument
               (program_id, module_def_id, id, name, arg_type, position)
               VALUES ('APP-blank', 'MD-blank', 'ARG-blank', '  ', 'Numeric', 0);",
        )
        .unwrap();

    let mut project = project();
    add_device(&mut project, 1, "P-blank", "H2P-blank");
    add_parameter(&mut project, 1, 1, "PR-blank", "1");
    add_module_argument(&mut project, 1, 1, "ARG-blank", "7");

    let assert_raw_fallbacks = |report: &knx_report::HtmlReport| {
        let html = article(&report.html, 1);
        for expected in [
            "<tr><th>Manufacturer</th><td>M-blank</td></tr>",
            "<tr><th>Product</th><td>P-blank</td></tr>",
            "<tr><th>Application program</th><td>H2P-blank</td></tr>",
            "<td>PR-blank</td><td>PR-blank</td>",
            "<td>ARG-blank</td><td>ARG-blank</td>",
        ] {
            assert!(html.contains(expected), "missing {expected:?}: {html}");
        }
        assert!(!html.contains("<td></td>"), "{html}");
        for expected in [
            "manufacturer name",
            "product name",
            "application-program name",
            "field name",
        ] {
            assert!(
                report
                    .warnings
                    .iter()
                    .any(|warning| warning.detail.contains(expected)),
                "missing {expected:?} warning: {:?}",
                report
                    .warnings
                    .iter()
                    .map(|warning| warning.detail.as_str())
                    .collect::<Vec<_>>()
            );
        }
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.location.contains("PR-blank")));
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.location.contains("ARG-blank")));
    };

    assert_raw_fallbacks(&render(&project, &products));

    products
        .execute_batch(
            "UPDATE manufacturer SET name = '' WHERE id = 'M-blank';
             UPDATE hardware SET name = '' WHERE id = 'HW-blank';
             UPDATE product SET text = '' WHERE id = 'P-blank';
             UPDATE application_program SET name = '' WHERE id = 'APP-blank';
             UPDATE parameter SET name = '', text = '' WHERE id = 'P-blank-decl';
             UPDATE module_def_argument SET name = '' WHERE id = 'ARG-blank';",
        )
        .unwrap();
    assert_raw_fallbacks(&render(&project, &products));
}

#[test]
fn unrenderable_parameter_kinds_and_allocator_refs_keep_raw_values_and_warn() {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    products
        .execute_batch(
            "INSERT INTO manufacturer (id, name) VALUES ('M', 'Maker');
             INSERT INTO hardware (id, manufacturer_id, name, source_sha256)
               VALUES ('HW', 'M', 'Hardware', 'x');
             INSERT INTO product (id, manufacturer_id, hardware_id, text, source_sha256)
               VALUES ('P', 'M', 'HW', 'Product', 'x');
             INSERT INTO application_program (id, manufacturer_id, name, source_sha256)
               VALUES ('APP', 'M', 'Program', 'x');
             INSERT INTO hardware2program
               (id, manufacturer_id, hardware_id, application_program_ref, source_sha256)
               VALUES ('H2P', 'M', 'HW', 'APP', 'x');
             INSERT INTO parameter_type (program_id, id, kind)
               VALUES ('APP', 'PT-R', 'Restriction'),
                      ('APP', 'PT-N', 'Number'),
                      ('APP', 'PT-O', 'Other');
             INSERT INTO parameter_type_enum
               (program_id, parameter_type_id, id, value, text)
               VALUES ('APP', 'PT-R', 'E-R', '1', 'One');
             INSERT INTO parameter (program_id, id, name, text, parameter_type_id)
               VALUES ('APP', 'P-R', 'Restriction name', 'Restriction label', 'PT-R'),
                      ('APP', 'P-N', 'Number name', 'Number label', 'PT-N'),
                      ('APP', 'P-O', 'Opaque name', 'Opaque label', 'PT-O');
             INSERT INTO parameter_ref (program_id, id, parameter_id)
               VALUES ('APP', 'PR-R', 'P-R'),
                      ('APP', 'PR-N', 'P-N'),
                      ('APP', 'PR-O', 'P-O');
             INSERT INTO module_def_argument
               (program_id, module_def_id, id, name, arg_type, position)
               VALUES ('APP', 'MD', 'ARG-A', 'Allocator', 'AllocatorRef', 0);",
        )
        .unwrap();

    let mut project = project();
    add_device(&mut project, 1, "P", "H2P");
    add_parameter(&mut project, 1, 1, "PR-R", "99");
    add_parameter(&mut project, 2, 1, "PR-N", "42");
    add_parameter(&mut project, 3, 1, "PR-O", "opaque-value");
    add_module_argument(&mut project, 1, 1, "ARG-A", "allocator-7");

    let report = render(&project, &products);
    let html = article(&report.html, 1);

    for raw in ["99", "42", "opaque-value", "allocator-7"] {
        assert!(html.contains(raw), "missing raw value {raw}: {html}");
    }
    for expected in [
        "restriction value '99' is not declared",
        "parameter kind 'Number' has no report formatter",
        "parameter kind 'Other' has no report formatter",
        "module argument kind 'AllocatorRef' is unsupported",
    ] {
        let warning = report
            .warnings
            .iter()
            .find(|warning| warning.detail.contains(expected))
            .unwrap_or_else(|| panic!("missing {expected:?}: {html}"));
        assert!(html.contains(&warning.detail), "{html}");
    }
    assert!(
        report
            .html
            .contains("AllocatorRef and unknown module argument kinds"),
        "{}",
        report.html
    );
}
