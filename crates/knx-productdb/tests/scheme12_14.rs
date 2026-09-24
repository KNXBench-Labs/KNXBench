//! Scheme-12/14 acceptance preserves every observed semantic delta or reports it.

use std::collections::BTreeMap;
use std::io::{Cursor, Write};

use knx_productdb::report::{UnknownConstruct, UnknownKind};
use knx_productdb::{install_package, open_and_migrate, InstallFacts, PackageError};
use rusqlite::Connection;
use zip::write::SimpleFileOptions;

fn db() -> (tempfile::TempDir, Connection) {
    let directory = tempfile::tempdir().unwrap();
    let connection = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    (directory, connection)
}

fn archive(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in members {
        writer
            .start_file(*path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn master(scheme: u32) -> Vec<u8> {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/{scheme}"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#
    )
    .into_bytes()
}

fn program(scheme: u32) -> Vec<u8> {
    let deltas = if scheme == 12 {
        r#"<Property Occurrence="2"/><LoadProcedures><LdCtrlWriteProp AppliesTo="LoadProcedure"/></LoadProcedures></Static><Dynamic><ParameterBlock Id="PB-12"><ParameterSeparator Id="S-12" Text="Advanced" UIHint="Group" HorizontalRuler="true" Access="Read"/></ParameterBlock></Dynamic>"#
    } else {
        r#"<Property Occurrence="3"/><LoadProcedures><LoadProcedure><LdCtrlDeclarePropDesc ObjIdx="1" PropId="2" PropType="3" MaxElements="4" ReadAccess="5" WriteAccess="6" Writable="false"/></LoadProcedure></LoadProcedures></Static><Dynamic><ParameterBlock Id="PB-14"><ParameterSeparator Id="S-14" Text="Advanced" UIHint="Text"/></ParameterBlock></Dynamic>"#
    };
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/{scheme}"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-{scheme}" Name="Program"><Static>{deltas}</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
    .into_bytes()
}

fn package(scheme: u32) -> (Vec<u8>, Vec<u8>) {
    let program = program(scheme);
    let master = master(scheme);
    let bytes = archive(&[
        ("knx_master.xml", master.as_slice()),
        ("M-0001/A.xml", program.as_slice()),
    ]);
    (bytes, program)
}

fn database_counts(connection: &Connection) -> BTreeMap<String, i64> {
    let tables = connection
        .prepare(
            "SELECT name FROM sqlite_schema
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    tables
        .into_iter()
        .map(|table| {
            let quoted = table.replace('"', "\"\"");
            let count = connection
                .query_row(&format!("SELECT count(*) FROM \"{quoted}\""), [], |row| {
                    row.get(0)
                })
                .unwrap();
            (table, count)
        })
        .collect()
}

fn unknown<'a>(
    facts: &'a InstallFacts,
    kind: UnknownKind,
    name: &str,
    path_suffix: &str,
) -> &'a UnknownConstruct {
    facts
        .unknown_constructs
        .iter()
        .find(|item| item.kind == kind && item.name == name && item.xpath.ends_with(path_suffix))
        .unwrap_or_else(|| {
            panic!(
                "missing {kind:?} {path_suffix}/@{name}; observed: {:?}",
                facts.unknown_constructs
            )
        })
}

fn unknown_rows(
    connection: &Connection,
    package_sha256: &str,
) -> Vec<(String, String, String, i64, Option<String>)> {
    connection
        .prepare(
            "SELECT xpath, kind, name, occurrences, sample
             FROM package_install_unknown
             WHERE package_sha256 = ?1
             ORDER BY xpath, kind, name",
        )
        .unwrap()
        .query_map([package_sha256], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

#[test]
fn schemes_12_and_14_install_with_explicit_loss_accounting() {
    for scheme in [12, 14] {
        let (_directory, connection) = db();
        let (bytes, program) = package(scheme);

        let first =
            install_package(&connection, &format!("scheme-{scheme}.knxprod"), &bytes).unwrap();
        assert_eq!(first.scheme, scheme);
        assert!(!first.skipped);
        let facts = first.facts.clone().expect("measured install evidence");

        assert_eq!(
            connection
                .query_row(
                    "SELECT bytes FROM source_file WHERE source_path = 'M-0001/A.xml'",
                    [],
                    |row| row.get::<_, Vec<u8>>(0),
                )
                .unwrap(),
            program
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM dynamic_node
                     WHERE program_id = ?1 AND kind = 'ParameterSeparator'",
                    [format!("A-{scheme}")],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            1
        );
        assert_eq!(
            unknown(
                &facts,
                UnknownKind::Attribute,
                "Occurrence",
                "/Static/Property"
            )
            .sample
            .as_deref(),
            Some(if scheme == 12 { "2" } else { "3" })
        );
        assert_eq!(
            unknown(
                &facts,
                UnknownKind::Attribute,
                "UIHint",
                "/Dynamic/ParameterBlock/ParameterSeparator"
            )
            .sample
            .as_deref(),
            Some(if scheme == 12 { "Group" } else { "Text" })
        );
        for name in ["Occurrence", "UIHint"] {
            assert_eq!(
                facts
                    .unknown_constructs
                    .iter()
                    .filter(|item| item.name == name)
                    .count(),
                1,
                "scheme {scheme} duplicated {name} evidence"
            );
        }

        if scheme == 12 {
            assert_eq!(
                unknown(
                    &facts,
                    UnknownKind::Attribute,
                    "AppliesTo",
                    "/LoadProcedures/LdCtrlWriteProp"
                )
                .sample
                .as_deref(),
                Some("LoadProcedure")
            );
            for (name, sample) in [("HorizontalRuler", "true"), ("Access", "Read")] {
                assert_eq!(
                    unknown(
                        &facts,
                        UnknownKind::Attribute,
                        name,
                        "/Dynamic/ParameterSeparator"
                    )
                    .sample
                    .as_deref(),
                    Some(sample)
                );
            }
            for name in ["AppliesTo", "HorizontalRuler", "Access"] {
                assert_eq!(
                    facts
                        .unknown_constructs
                        .iter()
                        .filter(|item| item.name == name)
                        .count(),
                    1,
                    "scheme 12 duplicated {name} evidence"
                );
            }
        } else {
            assert!(facts.unknown_constructs.iter().any(|item| {
                item.kind == UnknownKind::Element && item.name == "LdCtrlDeclarePropDesc"
            }));
            assert_eq!(
                facts
                    .unknown_constructs
                    .iter()
                    .filter(|item| {
                        item.kind == UnknownKind::Element && item.name == "LdCtrlDeclarePropDesc"
                    })
                    .count(),
                1
            );
            for (name, sample) in [
                ("ObjIdx", "1"),
                ("PropId", "2"),
                ("PropType", "3"),
                ("MaxElements", "4"),
                ("ReadAccess", "5"),
                ("WriteAccess", "6"),
                ("Writable", "false"),
            ] {
                assert_eq!(
                    unknown(
                        &facts,
                        UnknownKind::Attribute,
                        name,
                        "/LoadProcedure/LdCtrlDeclarePropDesc"
                    )
                    .sample
                    .as_deref(),
                    Some(sample)
                );
                assert_eq!(
                    facts
                        .unknown_constructs
                        .iter()
                        .filter(|item| item.name == name)
                        .count(),
                    1,
                    "scheme 14 duplicated {name} evidence"
                );
            }
        }

        let mut expected_unknowns = facts
            .unknown_constructs
            .iter()
            .map(|item| {
                (
                    item.xpath.clone(),
                    format!("{:?}", item.kind),
                    item.name.clone(),
                    i64::from(item.occurrences),
                    item.sample.clone(),
                )
            })
            .collect::<Vec<_>>();
        expected_unknowns.sort();
        assert_eq!(unknown_rows(&connection, &first.sha256), expected_unknowns);
        let persisted_report = connection
            .query_row(
                "SELECT unknown_distinct, unknown_occurrences
                 FROM package_install_report
                 WHERE package_sha256 = ?1",
                [&first.sha256],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .unwrap();
        assert_eq!(
            persisted_report,
            (
                i64::try_from(facts.unknown_constructs.len()).unwrap(),
                facts
                    .unknown_constructs
                    .iter()
                    .map(|item| i64::from(item.occurrences))
                    .sum()
            )
        );
        assert!(facts
            .unknown_constructs
            .iter()
            .all(|item| item.occurrences == 1));

        let retry = install_package(&connection, "renamed.knxprod", &bytes).unwrap();
        assert!(retry.skipped);
        assert_eq!(retry.scheme, scheme);
        assert_eq!(retry.facts, Some(facts));
    }
}

#[test]
fn existing_supported_schemes_do_not_gain_pdb5_evidence() {
    for scheme in [11, 20] {
        let (_directory, connection) = db();
        let master = master(scheme);
        let program = format!(
            r#"<KNX xmlns="http://knx.org/xml/project/{scheme}"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-{scheme}" Name="Program"><Static/><Dynamic><ParameterSeparator Id="S-{scheme}" Text="Existing" UIHint="Text"/></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
        );
        let bytes = archive(&[
            ("knx_master.xml", master.as_slice()),
            ("M-0001/A.xml", program.as_bytes()),
        ]);

        let report = install_package(&connection, "existing.knxprod", &bytes).unwrap();

        assert_eq!(report.scheme, scheme);
        let facts = report.facts.expect("measured install evidence");
        assert!(facts.unknown_constructs.iter().all(|item| {
            !matches!(
                item.name.as_str(),
                "AppliesTo"
                    | "Occurrence"
                    | "Access"
                    | "HorizontalRuler"
                    | "UIHint"
                    | "LdCtrlDeclarePropDesc"
            )
        }));
        assert!(unknown_rows(&connection, &report.sha256).iter().all(|row| {
            !matches!(
                row.2.as_str(),
                "AppliesTo"
                    | "Occurrence"
                    | "Access"
                    | "HorizontalRuler"
                    | "UIHint"
                    | "LdCtrlDeclarePropDesc"
            )
        }));
        let retry = install_package(&connection, "existing-renamed.knxprod", &bytes).unwrap();
        assert!(retry.skipped);
        assert_eq!(retry.scheme, scheme);
        assert_eq!(retry.facts, Some(facts));
    }
}

#[test]
fn prefixed_dynamic_lookalikes_are_persisted_as_qualified_unknowns() {
    let (_directory, connection) = db();
    let master = master(12);
    let program = br#"<KNX xmlns="http://knx.org/xml/project/12" xmlns:e="urn:example"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-EXT" Name="Extension evidence"><Static><e:Property Occurrence="7"/><LdCtrlDeclarePropDesc xmlns="" ObjIdx="9"/></Static><e:Dynamic Foo="root-foreign"><ParameterSeparator Id="S-1" Text="Official" e:UIHint="qualified-attribute"/><e:ParameterSeparator Id="E-1" UIHint="qualified-element"/><ParameterSeparator xmlns="urn:default-extension" Id="D-1" UIHint="default-qualified-element"/><e:ParameterBlock Id="E-BLOCK"><e:ParameterSeparator Id="E-SEP" UIHint="foreign-tree" Access="Read" HorizontalRuler="true"/></e:ParameterBlock></e:Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
    let bytes = archive(&[
        ("knx_master.xml", master.as_slice()),
        ("M-0001/A.xml", program.as_slice()),
    ]);

    let report = install_package(&connection, "extension.knxprod", &bytes).unwrap();
    let facts = report.facts.expect("measured extension evidence");
    assert!(facts.unknown_constructs.iter().any(|item| {
        item.name == "{http://knx.org/xml/project/12}ParameterSeparator/@{urn:example}UIHint"
            && item.sample.as_deref() == Some("qualified-attribute")
    }));
    for name in [
        "{urn:example}Property",
        "{urn:example}Dynamic",
        "{urn:example}ParameterBlock",
        "{urn:example}ParameterSeparator",
    ] {
        assert!(facts
            .unknown_constructs
            .iter()
            .any(|item| item.kind == UnknownKind::Element && item.name == name));
    }
    assert!(!facts.unknown_constructs.iter().any(|item| {
        (item.kind == UnknownKind::Element && item.name == "Property")
            || (item.kind == UnknownKind::Attribute
                && item.name == "Occurrence"
                && item.sample.as_deref() == Some("7"))
    }));
    assert!(facts.unknown_constructs.iter().any(|item| {
        item.kind == UnknownKind::Element && item.name == "{}LdCtrlDeclarePropDesc"
    }));
    assert!(!facts
        .unknown_constructs
        .iter()
        .any(|item| { item.kind == UnknownKind::Element && item.name == "LdCtrlDeclarePropDesc" }));
    for (namespace, sample) in [
        ("urn:example", "qualified-element"),
        ("urn:default-extension", "default-qualified-element"),
    ] {
        let element_name = format!("{{{namespace}}}ParameterSeparator");
        let path_name = if namespace == "urn:example" {
            "e:ParameterSeparator"
        } else {
            "ParameterSeparator"
        };
        let attribute_name = format!("{element_name}/@UIHint");
        assert!(facts
            .unknown_constructs
            .iter()
            .any(|item| { item.kind == UnknownKind::Element && item.name == element_name }));
        assert!(facts.unknown_constructs.iter().any(|item| {
            item.xpath.ends_with(&format!("/{path_name}"))
                && item.name == attribute_name
                && item.sample.as_deref() == Some(sample)
        }));
    }
    let foreign_path = "/e:Dynamic/e:ParameterBlock/e:ParameterSeparator";
    assert!(facts.unknown_constructs.iter().any(|item| {
        item.xpath.ends_with("/e:Dynamic")
            && item.name == "{urn:example}Dynamic/@Foo"
            && item.sample.as_deref() == Some("root-foreign")
    }));
    assert!(!facts
        .unknown_constructs
        .iter()
        .any(|item| { item.xpath.ends_with("/Dynamic/Dynamic") && item.name == "Foo" }));
    for (name, sample) in [("Access", "Read"), ("HorizontalRuler", "true")] {
        assert!(facts.unknown_constructs.iter().any(|item| {
            item.xpath.ends_with(foreign_path)
                && item.name == format!("{{urn:example}}ParameterSeparator/@{name}")
                && item.sample.as_deref() == Some(sample)
        }));
        assert!(!facts.unknown_constructs.iter().any(|item| {
            item.xpath.ends_with("/Dynamic/ParameterSeparator") && item.name == name
        }));
    }
    assert!(unknown_rows(&connection, &report.sha256)
        .iter()
        .any(|row| row.2
            == "{http://knx.org/xml/project/12}ParameterSeparator/@{urn:example}UIHint"
            && row.4.as_deref() == Some("qualified-attribute")));
    assert!(unknown_rows(&connection, &report.sha256)
        .iter()
        .any(|row| row.0.ends_with("/e:ParameterSeparator")
            && row.2 == "{urn:example}ParameterSeparator/@UIHint"
            && row.4.as_deref() == Some("qualified-element")));
    assert!(unknown_rows(&connection, &report.sha256)
        .iter()
        .any(|row| row.0.ends_with("/ParameterSeparator")
            && row.2 == "{urn:default-extension}ParameterSeparator/@UIHint"
            && row.4.as_deref() == Some("default-qualified-element")));
}

#[test]
fn namespace_collisions_and_wrapped_modules_reconcile_to_parser_semantics() {
    let (_directory, connection) = db();
    let master = master(12);
    let program = br#"<KNX xmlns="http://knx.org/xml/project/12" xmlns:e="urn:example"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-RECONCILE" Name="Reconcile"><Static/><Dynamic e:RootOnly="root-qualified"><when default="true" e:default="false"/><when e:default="false" default="true"/><when e:default="true"/><when default="false"/><choose><when e:default="false"/></choose></Dynamic><ModuleDefs><Wrapper><ModuleDef Id="MD-1"><Wrapper><e:Dynamic><e:ParameterSeparator Id="S-1" Weird="module-value"/></e:Dynamic></Wrapper></ModuleDef><Wrapper><ModuleDef Id="MD-2"><Wrapper><Dynamic><choose><when e:ModuleOnly="module-canonical"/></choose></Dynamic></Wrapper></ModuleDef></Wrapper></Wrapper></ModuleDefs></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
    let bytes = archive(&[
        ("knx_master.xml", master.as_slice()),
        ("M-0001/A.xml", program.as_slice()),
    ]);

    let report = install_package(&connection, "reconcile.knxprod", &bytes).unwrap();
    let facts = report.facts.expect("measured reconciliation evidence");

    assert!(facts.unknown_constructs.iter().any(|item| {
        item.name == "default" && item.occurrences == 1 && item.sample.as_deref() == Some("false")
    }));
    assert!(!facts
        .unknown_constructs
        .iter()
        .any(|item| item.name == "{}default"));
    let qualified_defaults = facts
        .unknown_constructs
        .iter()
        .filter(|item| item.name == "{urn:example}default")
        .collect::<Vec<_>>();
    assert_eq!(
        qualified_defaults
            .iter()
            .map(|item| item.occurrences)
            .sum::<u32>(),
        4
    );
    assert!(qualified_defaults
        .iter()
        .any(|item| item.sample.as_deref() == Some("false")));
    assert!(!facts
        .unknown_constructs
        .iter()
        .any(|item| item.name == "e:default"));
    assert!(facts.unknown_constructs.iter().any(|item| {
        item.xpath.ends_with("/ApplicationProgram/Dynamic")
            && item.name == "{urn:example}RootOnly"
            && item.sample.as_deref() == Some("root-qualified")
    }));
    assert!(!facts
        .unknown_constructs
        .iter()
        .any(|item| item.name == "e:RootOnly"));
    assert!(facts.unknown_constructs.iter().any(|item| {
        item.xpath.ends_with("/e:Dynamic/e:ParameterSeparator")
            && item.name == "{urn:example}ParameterSeparator/@Weird"
            && item.sample.as_deref() == Some("module-value")
    }));
    assert!(!facts.unknown_constructs.iter().any(|item| {
        item.xpath
            .ends_with("/ModuleDefs/ModuleDef/Dynamic/ParameterSeparator")
            && item.name == "Weird"
    }));
    assert!(facts.unknown_constructs.iter().any(|item| {
        item.xpath
            .ends_with("/ModuleDef/Wrapper/Dynamic/choose/when")
            && item.name == "{urn:example}ModuleOnly"
            && item.sample.as_deref() == Some("module-canonical")
    }));
    assert!(!facts
        .unknown_constructs
        .iter()
        .any(|item| item.name == "e:ModuleOnly"));
}

#[test]
fn excessive_evidence_nesting_rolls_back_the_whole_package() {
    let (_directory, connection) = db();
    let master = master(12);
    let mut program = String::from(
        r#"<KNX xmlns="http://knx.org/xml/project/12"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-DEEP" Name="Deep"><Static>"#,
    );
    for _ in 0..1_100 {
        program.push_str("<Nested>");
    }
    for _ in 0..1_100 {
        program.push_str("</Nested>");
    }
    program.push_str(
        "</Static><Dynamic/></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>",
    );
    let bytes = archive(&[
        ("knx_master.xml", master.as_slice()),
        ("M-0001/A.xml", program.as_bytes()),
    ]);
    let before = database_counts(&connection);

    let error = install_package(&connection, "deep.knxprod", &bytes)
        .expect_err("the evidence depth budget must reject this package");

    assert!(error.to_string().contains("nesting exceeds evidence limit"));
    assert_eq!(database_counts(&connection), before);
}

#[test]
fn schemes_12_and_14_roll_back_after_prior_member_writes() {
    for scheme in [12, 14] {
        let (_directory, connection) = db();
        connection
            .execute_batch(&format!(
                "CREATE TRIGGER force_late_scheme_{scheme}_failure
                 BEFORE INSERT ON application_program
                 WHEN NEW.id = 'A-{scheme}'
                 BEGIN
                     SELECT RAISE(ABORT, 'forced late scheme failure');
                 END;"
            ))
            .unwrap();
        let before = database_counts(&connection);
        let (bytes, _) = package(scheme);

        assert!(matches!(
            install_package(
                &connection,
                &format!("late-failure-{scheme}.knxprod"),
                &bytes
            ),
            Err(PackageError::Database(_))
        ));
        assert_eq!(database_counts(&connection), before, "scheme {scheme}");
    }
}

#[test]
fn lookalike_and_unapproved_namespaces_remain_rejected_without_rows() {
    let (_directory, connection) = db();
    let before = database_counts(&connection);
    for namespace in [
        "http://knx.org/xml/project/120",
        "http://knx.org/xml/project/140",
        "https://knx.org/xml/project/12",
        "https://knx.org/xml/project/14",
        "http://knx.org/xml/project/15",
        "urn:example:project:14",
    ] {
        let master = format!("<KNX xmlns=\"{namespace}\"><MasterData/></KNX>");
        let result = install_package(
            &connection,
            "unsupported.knxprod",
            &archive(&[("knx_master.xml", master.as_bytes())]),
        );
        assert!(matches!(
            result,
            Err(PackageError::UnsupportedNamespace { namespace: actual }) if actual == namespace
        ));
        assert_eq!(database_counts(&connection), before);
    }
}
