//! Fixed typed expectations for the independently checked fresh migration chain.
//!
//! Source: c012f714 full-chain oracle, SQLite 3.53.2; historical DDL is not admitted.
//! Schema rowids, rootpages and incidental index-list sequence are not constants.

use rusqlite::types::ValueRef::{self, Integer, Null, Text};

#[rustfmt::skip]
pub(super) const OBJECTS: &[[ValueRef<'static>; 4]] = &[
    [Text("table".as_bytes()), Text("application_program".as_bytes()), Text("application_program".as_bytes()), Text("CREATE TABLE application_program (\n             id                    TEXT PRIMARY KEY,\n             manufacturer_id       TEXT NOT NULL,\n             name                  TEXT,\n             application_number    TEXT,\n             application_version   TEXT,\n             program_type          TEXT,\n             mask_version          TEXT,\n             pei_type              TEXT,\n             load_procedure_style  TEXT,\n             default_language      TEXT,\n             hash                  TEXT,\n             linkable              INTEGER,\n             original_manufacturer TEXT,\n             source_sha256         TEXT NOT NULL\n         , is_secure_enabled TEXT, max_security_group_key_table_entries TEXT, max_security_individual_address_entries TEXT, max_security_p2p_key_table_entries TEXT, max_tunneling_user_entries TEXT, max_user_entries TEXT, min_ets_version TEXT, replaces_versions TEXT, write_authority_recorded INTEGER NOT NULL DEFAULT 0) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("application_program_source".as_bytes()), Text("application_program".as_bytes()), Text("CREATE INDEX application_program_source ON application_program (source_sha256)".as_bytes())],
    [Text("table".as_bytes()), Text("catalog_item".as_bytes()), Text("catalog_item".as_bytes()), Text("CREATE TABLE catalog_item (\n             id                      TEXT PRIMARY KEY,\n             manufacturer_id         TEXT NOT NULL,\n             section_id              TEXT NOT NULL,\n             name                    TEXT,\n             number                  TEXT,\n             visible_description     TEXT,\n             product_ref_id          TEXT,\n             hardware2program_ref_id TEXT,\n             default_language        TEXT,\n             source_sha256           TEXT NOT NULL\n         ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("catalog_item_section".as_bytes()), Text("catalog_item".as_bytes()), Text("CREATE INDEX catalog_item_section ON catalog_item (section_id)".as_bytes())],
    [Text("index".as_bytes()), Text("catalog_item_source".as_bytes()), Text("catalog_item".as_bytes()), Text("CREATE INDEX catalog_item_source ON catalog_item (source_sha256)".as_bytes())],
    [Text("table".as_bytes()), Text("catalog_section".as_bytes()), Text("catalog_section".as_bytes()), Text("CREATE TABLE catalog_section (\n             id                  TEXT PRIMARY KEY,\n             manufacturer_id     TEXT NOT NULL,\n             parent_id           TEXT,\n             name                TEXT,\n             number              TEXT,\n             visible_description TEXT,\n             default_language    TEXT,\n             source_sha256       TEXT NOT NULL\n         ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("catalog_section_source".as_bytes()), Text("catalog_section".as_bytes()), Text("CREATE INDEX catalog_section_source ON catalog_section (source_sha256)".as_bytes())],
    [Text("table".as_bytes()), Text("com_object".as_bytes()), Text("com_object".as_bytes()), Text("CREATE TABLE com_object (\n             program_id          TEXT NOT NULL,\n             id                  TEXT NOT NULL,\n             number              INTEGER,\n             name                TEXT,\n             text                TEXT,\n             function_text       TEXT,\n             visible_description TEXT,\n             object_size         TEXT,\n             priority            TEXT,\n             dpt_list            TEXT,\n             read_flag           TEXT,\n             write_flag          TEXT,\n             transmit_flag       TEXT,\n             update_flag         TEXT,\n             communication_flag  TEXT,\n             read_on_init_flag   TEXT,\n             PRIMARY KEY (program_id, id)\n         ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("com_object_ref".as_bytes()), Text("com_object_ref".as_bytes()), Text("CREATE TABLE com_object_ref (\n             program_id          TEXT NOT NULL,\n             id                  TEXT NOT NULL,\n             com_object_id       TEXT NOT NULL,\n             tag                 TEXT,\n             text                TEXT,\n             function_text       TEXT,\n             visible_description TEXT,\n             object_size         TEXT,\n             priority            TEXT,\n             dpt_list            TEXT,\n             read_flag           TEXT,\n             write_flag          TEXT,\n             transmit_flag       TEXT,\n             update_flag         TEXT,\n             communication_flag  TEXT,\n             read_on_init_flag   TEXT, text_parameter_ref_id TEXT,\n             PRIMARY KEY (program_id, id)\n         ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("com_object_ref_object".as_bytes()), Text("com_object_ref".as_bytes()), Text("CREATE INDEX com_object_ref_object ON com_object_ref (program_id, com_object_id)".as_bytes())],
    [Text("index".as_bytes()), Text("com_object_ref_program".as_bytes()), Text("com_object_ref".as_bytes()), Text("CREATE INDEX com_object_ref_program ON com_object_ref (program_id)".as_bytes())],
    [Text("table".as_bytes()), Text("datapoint_type".as_bytes()), Text("datapoint_type".as_bytes()), Text("CREATE TABLE datapoint_type (\n             id   TEXT PRIMARY KEY,\n             main INTEGER NOT NULL,\n             sub  INTEGER,\n             name TEXT,\n             text TEXT\n         ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("dynamic_node".as_bytes()), Text("dynamic_node".as_bytes()), Text("CREATE TABLE dynamic_node (\n            program_id    TEXT NOT NULL,\n            module_def_id TEXT NOT NULL,\n            node_id       INTEGER NOT NULL,\n            parent_id     INTEGER,\n            position      INTEGER NOT NULL,\n            kind          TEXT NOT NULL,\n            element_id    TEXT,\n            ref_id        TEXT,\n            test          TEXT,\n            is_default    INTEGER,\n            text          TEXT,\n            extra         TEXT, value TEXT, name TEXT, number TEXT, text_parameter_ref_id TEXT, access TEXT, ui_hint TEXT,\n            PRIMARY KEY (program_id, module_def_id, node_id)\n        ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("dynamic_node_sibling".as_bytes()), Text("dynamic_node".as_bytes()), Text("CREATE UNIQUE INDEX dynamic_node_sibling\n            ON dynamic_node (program_id, module_def_id, parent_id, position)".as_bytes())],
    [Text("table".as_bytes()), Text("function_point".as_bytes()), Text("function_point".as_bytes()), Text("CREATE TABLE function_point (\n             id               TEXT PRIMARY KEY,\n             function_type_id TEXT NOT NULL,\n             datapoint_type   TEXT,\n             role             TEXT,\n             characteristics  TEXT,\n             text             TEXT\n         ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("function_point_function_type".as_bytes()), Text("function_point".as_bytes()), Text("CREATE INDEX function_point_function_type ON function_point (function_type_id)".as_bytes())],
    [Text("table".as_bytes()), Text("function_type".as_bytes()), Text("function_type".as_bytes()), Text("CREATE TABLE function_type (\n             id     TEXT PRIMARY KEY,\n             number INTEGER,\n             text   TEXT,\n             status TEXT\n         ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("hardware".as_bytes()), Text("hardware".as_bytes()), Text("CREATE TABLE hardware (\n             id                      TEXT PRIMARY KEY,\n             manufacturer_id         TEXT NOT NULL,\n             name                    TEXT,\n             serial_number           TEXT,\n             version_number          TEXT,\n             bus_current             TEXT,\n             has_individual_address  INTEGER,\n             has_application_program INTEGER,\n             is_accessory            INTEGER,\n             is_coupler              INTEGER,\n             is_power_supply         INTEGER,\n             is_ip_enabled           INTEGER,\n             is_power_line_repeater  INTEGER,\n             original_manufacturer   TEXT,\n             source_sha256           TEXT NOT NULL\n         ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("hardware2program".as_bytes()), Text("hardware2program".as_bytes()), Text("CREATE TABLE hardware2program (\n             id                      TEXT PRIMARY KEY,\n             manufacturer_id         TEXT NOT NULL,\n             hardware_id             TEXT NOT NULL,\n             application_program_ref TEXT,\n             medium_types            TEXT,\n             hash                    TEXT,\n             registration_number     TEXT,\n             registration_status     TEXT,\n             registration_signature  TEXT,\n             source_sha256           TEXT NOT NULL\n         ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("hardware2program_program".as_bytes()), Text("hardware2program".as_bytes()), Text("CREATE INDEX hardware2program_program ON hardware2program (application_program_ref)".as_bytes())],
    [Text("index".as_bytes()), Text("hardware2program_source".as_bytes()), Text("hardware2program".as_bytes()), Text("CREATE INDEX hardware2program_source ON hardware2program (source_sha256)".as_bytes())],
    [Text("index".as_bytes()), Text("hardware_source".as_bytes()), Text("hardware".as_bytes()), Text("CREATE INDEX hardware_source ON hardware (source_sha256)".as_bytes())],
    [Text("table".as_bytes()), Text("ingest_unknown".as_bytes()), Text("ingest_unknown".as_bytes()), Text("CREATE TABLE ingest_unknown (\n             id            INTEGER PRIMARY KEY,\n             source_sha256 TEXT NOT NULL,\n             program_id    TEXT,\n             xpath         TEXT NOT NULL,\n             kind          TEXT NOT NULL,\n             name          TEXT NOT NULL,\n             occurrences   INTEGER NOT NULL,\n             sample        TEXT\n         ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("ingest_unknown_conflict_identity".as_bytes()), Text("ingest_unknown".as_bytes()), Text("CREATE UNIQUE INDEX ingest_unknown_conflict_identity ON ingest_unknown\n         (source_sha256, xpath, name, occurrences, ifnull(sample, '')) WHERE kind = 'IdConflict'".as_bytes())],
    [Text("index".as_bytes()), Text("ingest_unknown_construct_identity".as_bytes()), Text("ingest_unknown".as_bytes()), Text("CREATE UNIQUE INDEX ingest_unknown_construct_identity ON ingest_unknown\n         (source_sha256, ifnull(program_id, ''), xpath, kind, name) WHERE kind <> 'IdConflict'".as_bytes())],
    [Text("index".as_bytes()), Text("ingest_unknown_source".as_bytes()), Text("ingest_unknown".as_bytes()), Text("CREATE INDEX ingest_unknown_source ON ingest_unknown (source_sha256)".as_bytes())],
    [Text("table".as_bytes()), Text("legacy_diagnostic".as_bytes()), Text("legacy_diagnostic".as_bytes()), Text("CREATE TABLE legacy_diagnostic (\n             payload_sha256 TEXT NOT NULL REFERENCES legacy_source (payload_sha256),\n             ordinal INTEGER NOT NULL,\n             kind TEXT NOT NULL,\n             detail TEXT NOT NULL,\n             PRIMARY KEY (payload_sha256, ordinal)\n         ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("legacy_program".as_bytes()), Text("legacy_program".as_bytes()), Text("CREATE TABLE legacy_program (\n             program_id TEXT PRIMARY KEY,\n             payload_sha256 TEXT NOT NULL REFERENCES legacy_source (payload_sha256),\n             exim_program_id TEXT NOT NULL\n         ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("legacy_source".as_bytes()), Text("legacy_source".as_bytes()), Text("CREATE TABLE legacy_source (\n             payload_sha256 TEXT PRIMARY KEY,\n             namespace TEXT NOT NULL UNIQUE,\n             member_name TEXT NOT NULL,\n             member_kind TEXT NOT NULL,\n             format_version TEXT,\n             exported_at TEXT,\n             producer TEXT,\n             charset TEXT NOT NULL\n         ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("legacy_source_file".as_bytes()), Text("legacy_source_file".as_bytes()), Text("CREATE TABLE legacy_source_file (\n             payload_sha256 TEXT NOT NULL REFERENCES legacy_source (payload_sha256),\n             original_sha256 TEXT NOT NULL,\n             source_name TEXT NOT NULL,\n             encrypted INTEGER NOT NULL,\n             PRIMARY KEY (payload_sha256, original_sha256, source_name)\n         ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("manufacturer".as_bytes()), Text("manufacturer".as_bytes()), Text("CREATE TABLE manufacturer (\n             id   TEXT PRIMARY KEY,\n             name TEXT\n         ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("master_declaration".as_bytes()), Text("master_declaration".as_bytes()), Text("CREATE TABLE master_declaration (\n        source_sha256 TEXT NOT NULL REFERENCES master_source(source_sha256),\n        ordinal INTEGER NOT NULL CHECK(ordinal>0),\n        family TEXT NOT NULL CHECK(family IN ('datapoint_type','function_type','function_point','space_usage')),\n        element TEXT NOT NULL CHECK(element IN ('DatapointType','DatapointSubtype','FunctionType','FunctionPoint','SpaceUsage')),\n        logical_id TEXT, xml_path TEXT NOT NULL,\n        byte_start INTEGER NOT NULL CHECK(byte_start>=0),\n        byte_end INTEGER NOT NULL CHECK(byte_end>byte_start),\n        main_number INTEGER, function_type_id TEXT,\n        missing_id INTEGER NOT NULL CHECK(missing_id IN (0,1)),\n        orphan_parent INTEGER NOT NULL CHECK(orphan_parent IN (0,1)),\n        observed_write INTEGER CHECK(observed_write IN (0,1)),\n        CHECK(missing_id=(logical_id IS NULL)),\n        CHECK(observed_write IS NULL OR (missing_id=0 AND orphan_parent=0)),\n        PRIMARY KEY(source_sha256,ordinal)\n    ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("master_declaration_identity".as_bytes()), Text("master_declaration".as_bytes()), Text("CREATE INDEX master_declaration_identity ON master_declaration(family,logical_id,source_sha256,ordinal)".as_bytes())],
    [Text("table".as_bytes()), Text("master_source".as_bytes()), Text("master_source".as_bytes()), Text("CREATE TABLE master_source (\n        source_sha256 TEXT PRIMARY KEY,\n        definition_version INTEGER NOT NULL CHECK(definition_version=1),\n        history TEXT NOT NULL CHECK(history IN ('observed','historical-unavailable')),\n        status TEXT NOT NULL CHECK(status IN ('complete','failed','unexamined')),\n        reason TEXT,\n        CHECK((status='complete')=(reason IS NULL))\n    ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("master_winner".as_bytes()), Text("master_winner".as_bytes()), Text("CREATE TABLE master_winner (\n        family TEXT NOT NULL CHECK(family IN ('datapoint_type','function_type','function_point','space_usage')),\n        logical_id TEXT NOT NULL,\n        source_sha256 TEXT, ordinal INTEGER,\n        CHECK((source_sha256 IS NULL)=(ordinal IS NULL)),\n        FOREIGN KEY(source_sha256,ordinal) REFERENCES master_declaration(source_sha256,ordinal),\n        PRIMARY KEY(family,logical_id)\n    ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("module_def_argument".as_bytes()), Text("module_def_argument".as_bytes()), Text("CREATE TABLE module_def_argument (\n            program_id    TEXT NOT NULL,\n            module_def_id TEXT NOT NULL,\n            id            TEXT NOT NULL,\n            name          TEXT,\n            arg_type      TEXT,\n            allocates     INTEGER,\n            position      INTEGER NOT NULL,\n            extra         TEXT,\n            PRIMARY KEY (program_id, module_def_id, id)\n        ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("module_def_argument_scope".as_bytes()), Text("module_def_argument".as_bytes()), Text("CREATE INDEX module_def_argument_scope\n            ON module_def_argument (program_id, module_def_id)".as_bytes())],
    [Text("table".as_bytes()), Text("package".as_bytes()), Text("package".as_bytes()), Text("CREATE TABLE package (\n            sha256 TEXT PRIMARY KEY,\n            source_name TEXT NOT NULL,\n            scheme INTEGER NOT NULL,\n            size INTEGER NOT NULL,\n            bytes BLOB NOT NULL,\n            unknown_count INTEGER NOT NULL\n        , translation_program_count INTEGER NOT NULL DEFAULT 0, translation_catalog_count INTEGER NOT NULL DEFAULT 0, translation_hardware_count INTEGER NOT NULL DEFAULT 0, translation_master_count INTEGER NOT NULL DEFAULT 0, dropped_datapoint_type_count INTEGER NOT NULL DEFAULT 0) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("package_baggage_declaration".as_bytes()), Text("package_baggage_declaration".as_bytes()), Text("CREATE TABLE package_baggage_declaration (\n            package_sha256 TEXT NOT NULL REFERENCES package_baggage_inventory(package_sha256),\n            index_path TEXT NOT NULL,\n            ordinal INTEGER NOT NULL CHECK (ordinal >= 0),\n            baggage_id TEXT,\n            name TEXT,\n            target_path TEXT,\n            install_on_import TEXT,\n            time_info TEXT,\n            file_version TEXT,\n            resolution TEXT NOT NULL CHECK (resolution IN ('resolved','missing','invalid')),\n            member_path TEXT,\n            detail TEXT,\n            PRIMARY KEY (package_sha256, index_path, ordinal),\n            CHECK ((resolution = 'resolved') = (member_path IS NOT NULL)),\n            CHECK ((resolution = 'resolved') = (detail IS NULL))\n        ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("package_baggage_inventory".as_bytes()), Text("package_baggage_inventory".as_bytes()), Text("CREATE TABLE package_baggage_inventory (\n            package_sha256 TEXT PRIMARY KEY REFERENCES package(sha256),\n            status TEXT NOT NULL CHECK (status IN ('measured','unavailable'))\n        ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("package_baggage_payload".as_bytes()), Text("package_baggage_payload".as_bytes()), Text("CREATE TABLE package_baggage_payload (\n            package_sha256 TEXT NOT NULL REFERENCES package_baggage_inventory(package_sha256),\n            member_path TEXT NOT NULL,\n            sha256 TEXT NOT NULL REFERENCES source_file(sha256),\n            size INTEGER NOT NULL CHECK (size >= 0),\n            media_class TEXT NOT NULL CHECK (media_class IN ('empty','png','jpeg','gif','bmp','pdf','zip','pe-executable','ole2-compound','xml','unknown')),\n            declarations INTEGER NOT NULL CHECK (declarations >= 0),\n            nested_status TEXT NOT NULL CHECK (nested_status IN ('not-archive','unreadable','read')),\n            nested_entries INTEGER CHECK (nested_entries >= 0),\n            nested_expanded_size INTEGER CHECK (nested_expanded_size >= 0),\n            nested_encrypted_entries INTEGER CHECK (nested_encrypted_entries >= 0),\n            nested_archive_names INTEGER CHECK (nested_archive_names >= 0),\n            PRIMARY KEY (package_sha256, member_path),\n            CHECK ((nested_status = 'read') = (nested_entries IS NOT NULL)),\n            CHECK ((nested_entries IS NULL) = (nested_expanded_size IS NULL)),\n            CHECK ((nested_entries IS NULL) = (nested_encrypted_entries IS NULL)),\n            CHECK ((nested_entries IS NULL) = (nested_archive_names IS NULL)),\n            CHECK ((media_class = 'zip') = (nested_status <> 'not-archive'))\n        ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("package_conflict".as_bytes()), Text("package_conflict".as_bytes()), Text("CREATE TABLE package_conflict (\n            package_sha256 TEXT NOT NULL REFERENCES package(sha256),\n            ordinal INTEGER NOT NULL,\n            table_name TEXT NOT NULL,\n            logical_id TEXT NOT NULL,\n            kept_sha256 TEXT NOT NULL,\n            other_sha256 TEXT NOT NULL, occurrence INTEGER NOT NULL DEFAULT 1,\n            PRIMARY KEY (package_sha256, ordinal)\n        ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("package_install_count".as_bytes()), Text("package_install_count".as_bytes()), Text("CREATE TABLE \"package_install_count\" (\n            package_sha256 TEXT NOT NULL REFERENCES package_install_report(package_sha256),\n            ordinal INTEGER NOT NULL CHECK (ordinal >= 0),\n            category TEXT NOT NULL CHECK (category IN ('archive_member','product','application_program','parameter','communication_object','dynamic_node','module','baggage_index','baggage','unknown_construct','master_section','master_subtree','datapoint_type')),\n            disposition TEXT NOT NULL CHECK (disposition IN ('read','stored','deduplicated','retained-but-uninterpreted','unsupported','dropped')),\n            count INTEGER NOT NULL CHECK (count >= 0),\n            PRIMARY KEY (package_sha256, ordinal),\n            UNIQUE (package_sha256, category, disposition)\n        ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("package_install_diagnostic".as_bytes()), Text("package_install_diagnostic".as_bytes()), Text("CREATE TABLE \"package_install_diagnostic\" (\n        package_sha256 TEXT NOT NULL REFERENCES package_install_report(package_sha256),\n        ordinal INTEGER NOT NULL CHECK(ordinal>=0),\n        kind TEXT NOT NULL CHECK(kind IN ('unsupported-master-section','unsupported-master-subtree','unresolved-baggage-declaration','undeclared-baggage-payload','master-identity-collision','malformed-master-declaration')),\n        archive_path TEXT NOT NULL,xml_path TEXT NOT NULL,detail TEXT NOT NULL,\n        occurrences INTEGER NOT NULL CHECK(occurrences>0),\n        PRIMARY KEY(package_sha256,ordinal),\n        UNIQUE(package_sha256,kind,archive_path,xml_path,detail)\n    ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("package_install_report".as_bytes()), Text("package_install_report".as_bytes()), Text("CREATE TABLE package_install_report (\n            package_sha256 TEXT PRIMARY KEY REFERENCES package(sha256),\n            report_version INTEGER NOT NULL CHECK (report_version = 1),\n            status TEXT NOT NULL CHECK (status IN ('measured','unavailable')),\n            unknown_distinct INTEGER NOT NULL CHECK (unknown_distinct >= 0),\n            unknown_occurrences INTEGER NOT NULL CHECK (unknown_occurrences >= 0),\n            CHECK (status = 'measured' OR (unknown_distinct = 0 AND unknown_occurrences = 0))\n        ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("package_install_unknown".as_bytes()), Text("package_install_unknown".as_bytes()), Text("CREATE TABLE package_install_unknown (\n            package_sha256 TEXT NOT NULL REFERENCES package_install_report(package_sha256),\n            ordinal INTEGER NOT NULL CHECK (ordinal >= 0),\n            xpath TEXT NOT NULL,\n            kind TEXT NOT NULL CHECK (kind IN ('Element','Attribute')),\n            name TEXT NOT NULL,\n            occurrences INTEGER NOT NULL CHECK (occurrences > 0),\n            sample TEXT,\n            PRIMARY KEY (package_sha256, ordinal),\n            UNIQUE (package_sha256, xpath, kind, name)\n        ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("package_member".as_bytes()), Text("package_member".as_bytes()), Text("CREATE TABLE package_member (\n            package_sha256 TEXT NOT NULL REFERENCES package(sha256),\n            ordinal INTEGER NOT NULL,\n            path TEXT NOT NULL,\n            role TEXT NOT NULL,\n            source_sha256 TEXT NOT NULL REFERENCES source_file(sha256),\n            size INTEGER NOT NULL,\n            PRIMARY KEY (package_sha256, path),\n            UNIQUE (package_sha256, ordinal)\n        ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("package_source_name".as_bytes()), Text("package_source_name".as_bytes()), Text("CREATE TABLE package_source_name (\n            package_sha256 TEXT NOT NULL REFERENCES package(sha256),\n            source_name    TEXT NOT NULL,\n            PRIMARY KEY (package_sha256, source_name)\n        ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("parameter".as_bytes()), Text("parameter".as_bytes()), Text("CREATE TABLE parameter (\n             program_id        TEXT NOT NULL,\n             id                TEXT NOT NULL,\n             name              TEXT,\n             text              TEXT,\n             parameter_type_id TEXT,\n             access            TEXT,\n             value             TEXT,\n             suffix            TEXT,\n             code_segment      TEXT,\n             offset            INTEGER,\n             bit_offset        INTEGER,\n             union_id          INTEGER,\n             union_size_in_bit INTEGER, own_offset INTEGER, own_bit_offset INTEGER, property_object_index INTEGER, property_object_type INTEGER, property_id INTEGER, property_offset INTEGER, property_bit_offset INTEGER, property_occurrence INTEGER, suffix_text TEXT,\n             PRIMARY KEY (program_id, id)\n         ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("parameter_calculation_ref".as_bytes()), Text("parameter_calculation_ref".as_bytes()), Text("CREATE TABLE parameter_calculation_ref (\n             program_id       TEXT NOT NULL,\n             calculation_id   TEXT NOT NULL,\n             side             TEXT NOT NULL CHECK (side IN ('L', 'R')),\n             parameter_ref_id TEXT NOT NULL,\n             PRIMARY KEY (program_id, calculation_id, side, parameter_ref_id)\n         ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("parameter_calculation_ref_member".as_bytes()), Text("parameter_calculation_ref".as_bytes()), Text("CREATE INDEX parameter_calculation_ref_member\n             ON parameter_calculation_ref (program_id, parameter_ref_id)".as_bytes())],
    [Text("table".as_bytes()), Text("parameter_ref".as_bytes()), Text("parameter_ref".as_bytes()), Text("CREATE TABLE parameter_ref (\n             program_id    TEXT NOT NULL,\n             id            TEXT NOT NULL,\n             parameter_id  TEXT NOT NULL,\n             display_order INTEGER,\n             tag           TEXT,\n             text          TEXT,\n             value         TEXT, access TEXT, suffix_text TEXT,\n             PRIMARY KEY (program_id, id)\n         ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("parameter_ref_program".as_bytes()), Text("parameter_ref".as_bytes()), Text("CREATE INDEX parameter_ref_program ON parameter_ref (program_id)".as_bytes())],
    [Text("table".as_bytes()), Text("parameter_type".as_bytes()), Text("parameter_type".as_bytes()), Text("CREATE TABLE parameter_type (\n             program_id     TEXT NOT NULL,\n             id             TEXT NOT NULL,\n             name           TEXT,\n             kind           TEXT NOT NULL,\n             size_in_bit    INTEGER,\n             base           TEXT,\n             min_inclusive  TEXT,\n             max_inclusive  TEXT,\n             number_type    TEXT,\n             PRIMARY KEY (program_id, id)\n         ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("parameter_type_enum".as_bytes()), Text("parameter_type_enum".as_bytes()), Text("CREATE TABLE parameter_type_enum (\n             program_id        TEXT NOT NULL,\n             parameter_type_id TEXT NOT NULL,\n             id                TEXT NOT NULL,\n             value             TEXT,\n             text              TEXT,\n             display_order     INTEGER, binary_value TEXT, name TEXT, icon TEXT, picture_alignment TEXT,\n             PRIMARY KEY (program_id, id)\n         ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("parameter_type_enum_type".as_bytes()), Text("parameter_type_enum".as_bytes()), Text("CREATE INDEX parameter_type_enum_type ON parameter_type_enum (program_id, parameter_type_id)".as_bytes())],
    [Text("table".as_bytes()), Text("product".as_bytes()), Text("product".as_bytes()), Text("CREATE TABLE product (\n             id                  TEXT PRIMARY KEY,\n             manufacturer_id     TEXT NOT NULL,\n             hardware_id         TEXT NOT NULL,\n             text                TEXT,\n             order_number        TEXT,\n             is_rail_mounted     INTEGER,\n             width_in_millimeter TEXT,\n             default_language    TEXT,\n             hash                TEXT,\n             registration_status TEXT,\n             source_sha256       TEXT NOT NULL\n         ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("product_hardware".as_bytes()), Text("product".as_bytes()), Text("CREATE INDEX product_hardware ON product (hardware_id)".as_bytes())],
    [Text("index".as_bytes()), Text("product_source".as_bytes()), Text("product".as_bytes()), Text("CREATE INDEX product_source ON product (source_sha256)".as_bytes())],
    [Text("table".as_bytes()), Text("schema_meta".as_bytes()), Text("schema_meta".as_bytes()), Text("CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("source_file".as_bytes()), Text("source_file".as_bytes()), Text("CREATE TABLE source_file (\n             sha256          TEXT PRIMARY KEY,\n             source_path     TEXT NOT NULL,\n             manufacturer_id TEXT,\n             len             INTEGER NOT NULL,\n             bytes           BLOB NOT NULL\n         ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("source_file_manufacturer".as_bytes()), Text("source_file".as_bytes()), Text("CREATE INDEX source_file_manufacturer ON source_file (manufacturer_id)".as_bytes())],
    [Text("table".as_bytes()), Text("source_identity".as_bytes()), Text("source_identity".as_bytes()), Text("CREATE TABLE source_identity (\n            source_sha256 TEXT NOT NULL REFERENCES source_identity_scan(source_sha256),\n            table_name    TEXT NOT NULL CHECK (table_name IN ('catalog_section','catalog_item','hardware','product','hardware2program','application_program')),\n            logical_id    TEXT NOT NULL,\n            occurrence    INTEGER NOT NULL CHECK (occurrence >= 1),\n            digest        TEXT NOT NULL CHECK (length(digest) = 64),\n            PRIMARY KEY (source_sha256, table_name, logical_id, occurrence)\n        ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("source_identity_by_id".as_bytes()), Text("source_identity".as_bytes()), Text("CREATE INDEX source_identity_by_id ON source_identity (table_name, logical_id)".as_bytes())],
    [Text("table".as_bytes()), Text("source_identity_scan".as_bytes()), Text("source_identity_scan".as_bytes()), Text("CREATE TABLE source_identity_scan (\n            source_sha256 TEXT PRIMARY KEY REFERENCES source_file(sha256),\n            status        TEXT NOT NULL CHECK (status IN ('measured','unavailable')),\n            reason        TEXT,\n            scanner       INTEGER NOT NULL,\n            CHECK ((status = 'unavailable') = (reason IS NOT NULL))\n        ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("source_parse_evidence".as_bytes()), Text("source_parse_evidence".as_bytes()), Text("CREATE TABLE source_parse_evidence (\n            sha256 TEXT PRIMARY KEY REFERENCES source_file(sha256)\n        ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("source_producer".as_bytes()), Text("source_producer".as_bytes()), Text("CREATE TABLE source_producer (\n            source_sha256  TEXT PRIMARY KEY REFERENCES source_file(sha256),\n            root_namespace TEXT,\n            created_by     TEXT,\n            tool_version   TEXT\n        ) STRICT".as_bytes())],
    [Text("table".as_bytes()), Text("space_usage".as_bytes()), Text("space_usage".as_bytes()), Text("CREATE TABLE space_usage (\n             id     TEXT PRIMARY KEY,\n             number INTEGER,\n             text   TEXT\n         ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("sqlite_autoindex_application_program_1".as_bytes()), Text("application_program".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_catalog_item_1".as_bytes()), Text("catalog_item".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_catalog_section_1".as_bytes()), Text("catalog_section".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_com_object_1".as_bytes()), Text("com_object".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_com_object_ref_1".as_bytes()), Text("com_object_ref".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_datapoint_type_1".as_bytes()), Text("datapoint_type".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_dynamic_node_1".as_bytes()), Text("dynamic_node".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_function_point_1".as_bytes()), Text("function_point".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_function_type_1".as_bytes()), Text("function_type".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_hardware2program_1".as_bytes()), Text("hardware2program".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_hardware_1".as_bytes()), Text("hardware".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_legacy_diagnostic_1".as_bytes()), Text("legacy_diagnostic".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_legacy_program_1".as_bytes()), Text("legacy_program".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_legacy_source_1".as_bytes()), Text("legacy_source".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_legacy_source_2".as_bytes()), Text("legacy_source".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_legacy_source_file_1".as_bytes()), Text("legacy_source_file".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_manufacturer_1".as_bytes()), Text("manufacturer".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_master_declaration_1".as_bytes()), Text("master_declaration".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_master_source_1".as_bytes()), Text("master_source".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_master_winner_1".as_bytes()), Text("master_winner".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_module_def_argument_1".as_bytes()), Text("module_def_argument".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_package_1".as_bytes()), Text("package".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_package_baggage_declaration_1".as_bytes()), Text("package_baggage_declaration".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_package_baggage_inventory_1".as_bytes()), Text("package_baggage_inventory".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_package_baggage_payload_1".as_bytes()), Text("package_baggage_payload".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_package_conflict_1".as_bytes()), Text("package_conflict".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_package_install_count_1".as_bytes()), Text("package_install_count".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_package_install_count_2".as_bytes()), Text("package_install_count".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_package_install_diagnostic_1".as_bytes()), Text("package_install_diagnostic".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_package_install_diagnostic_2".as_bytes()), Text("package_install_diagnostic".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_package_install_report_1".as_bytes()), Text("package_install_report".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_package_install_unknown_1".as_bytes()), Text("package_install_unknown".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_package_install_unknown_2".as_bytes()), Text("package_install_unknown".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_package_member_1".as_bytes()), Text("package_member".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_package_member_2".as_bytes()), Text("package_member".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_package_source_name_1".as_bytes()), Text("package_source_name".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_parameter_1".as_bytes()), Text("parameter".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_parameter_calculation_ref_1".as_bytes()), Text("parameter_calculation_ref".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_parameter_ref_1".as_bytes()), Text("parameter_ref".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_parameter_type_1".as_bytes()), Text("parameter_type".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_parameter_type_enum_1".as_bytes()), Text("parameter_type_enum".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_product_1".as_bytes()), Text("product".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_schema_meta_1".as_bytes()), Text("schema_meta".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_source_file_1".as_bytes()), Text("source_file".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_source_identity_1".as_bytes()), Text("source_identity".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_source_identity_scan_1".as_bytes()), Text("source_identity_scan".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_source_parse_evidence_1".as_bytes()), Text("source_parse_evidence".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_source_producer_1".as_bytes()), Text("source_producer".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_space_usage_1".as_bytes()), Text("space_usage".as_bytes()), Null],
    [Text("index".as_bytes()), Text("sqlite_autoindex_translation_1".as_bytes()), Text("translation".as_bytes()), Null],
    [Text("table".as_bytes()), Text("translation".as_bytes()), Text("translation".as_bytes()), Text("CREATE TABLE \"translation\" (\n            scope          TEXT NOT NULL,\n            scope_id       TEXT NOT NULL,\n            language       TEXT NOT NULL,\n            ref_id         TEXT NOT NULL,\n            attribute_name TEXT NOT NULL,\n            text           TEXT,\n            PRIMARY KEY (scope, scope_id, language, ref_id, attribute_name)\n        ) STRICT".as_bytes())],
    [Text("index".as_bytes()), Text("translation_lookup".as_bytes()), Text("translation".as_bytes()), Text("CREATE INDEX translation_lookup ON translation (scope, scope_id, language, ref_id)".as_bytes())],
];

pub(super) struct ExpectedIndex {
    pub list: [ValueRef<'static>; 4], // name, unique, origin, partial (not incidental seq)
    pub xinfo: &'static [[ValueRef<'static>; 6]],
}
pub(super) struct ExpectedRelation {
    pub table: [ValueRef<'static>; 6],
    pub columns: &'static [[ValueRef<'static>; 7]],
    pub indexes: &'static [ExpectedIndex],
    pub foreign_keys: &'static [[ValueRef<'static>; 8]],
}

#[rustfmt::skip]
pub(super) const RELATIONS: &[ExpectedRelation] = &[
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("application_program".as_bytes()), Text("table".as_bytes()), Integer(23), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("manufacturer_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("name".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(3), Text("application_number".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("application_version".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("program_type".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(6), Text("mask_version".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(7), Text("pei_type".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(8), Text("load_procedure_style".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(9), Text("default_language".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(10), Text("hash".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(11), Text("linkable".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(12), Text("original_manufacturer".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(13), Text("source_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(14), Text("is_secure_enabled".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(15), Text("max_security_group_key_table_entries".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(16), Text("max_security_individual_address_entries".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(17), Text("max_security_p2p_key_table_entries".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(18), Text("max_tunneling_user_entries".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(19), Text("max_user_entries".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(20), Text("min_ets_version".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(21), Text("replaces_versions".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(22), Text("write_authority_recorded".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Text("0".as_bytes()), Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("application_program_source".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(13), Text("source_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_application_program_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("catalog_item".as_bytes()), Text("table".as_bytes()), Integer(10), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("manufacturer_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("section_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(3), Text("name".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("number".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("visible_description".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(6), Text("product_ref_id".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(7), Text("hardware2program_ref_id".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(8), Text("default_language".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(9), Text("source_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("catalog_item_section".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(2), Text("section_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("catalog_item_source".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(9), Text("source_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_catalog_item_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("catalog_section".as_bytes()), Text("table".as_bytes()), Integer(8), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("manufacturer_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("parent_id".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(3), Text("name".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("number".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("visible_description".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(6), Text("default_language".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(7), Text("source_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("catalog_section_source".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(7), Text("source_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_catalog_section_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("com_object".as_bytes()), Text("table".as_bytes()), Integer(16), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("program_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("number".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(3), Text("name".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("text".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("function_text".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(6), Text("visible_description".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(7), Text("object_size".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(8), Text("priority".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(9), Text("dpt_list".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(10), Text("read_flag".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(11), Text("write_flag".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(12), Text("transmit_flag".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(13), Text("update_flag".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(14), Text("communication_flag".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(15), Text("read_on_init_flag".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_com_object_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("com_object_ref".as_bytes()), Text("table".as_bytes()), Integer(17), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("program_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("com_object_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(3), Text("tag".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("text".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("function_text".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(6), Text("visible_description".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(7), Text("object_size".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(8), Text("priority".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(9), Text("dpt_list".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(10), Text("read_flag".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(11), Text("write_flag".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(12), Text("transmit_flag".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(13), Text("update_flag".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(14), Text("communication_flag".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(15), Text("read_on_init_flag".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(16), Text("text_parameter_ref_id".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("com_object_ref_object".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(2), Text("com_object_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("com_object_ref_program".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_com_object_ref_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("datapoint_type".as_bytes()), Text("table".as_bytes()), Integer(5), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("main".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("sub".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(3), Text("name".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("text".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_datapoint_type_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("dynamic_node".as_bytes()), Text("table".as_bytes()), Integer(18), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("program_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("module_def_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("node_id".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(3), Integer(0)],
            [Integer(3), Text("parent_id".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("position".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(5), Text("kind".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(6), Text("element_id".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(7), Text("ref_id".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(8), Text("test".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(9), Text("is_default".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(10), Text("text".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(11), Text("extra".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(12), Text("value".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(13), Text("name".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(14), Text("number".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(15), Text("text_parameter_ref_id".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(16), Text("access".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(17), Text("ui_hint".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("dynamic_node_sibling".as_bytes()), Integer(1), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("module_def_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(3), Text("parent_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(3), Integer(4), Text("position".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(4), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_dynamic_node_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("module_def_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(2), Text("node_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(3), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("function_point".as_bytes()), Text("table".as_bytes()), Integer(6), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("function_type_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("datapoint_type".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(3), Text("role".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("characteristics".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("text".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("function_point_function_type".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(1), Text("function_type_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_function_point_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("function_type".as_bytes()), Text("table".as_bytes()), Integer(4), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("number".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(2), Text("text".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(3), Text("status".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_function_type_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("hardware".as_bytes()), Text("table".as_bytes()), Integer(15), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("manufacturer_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("name".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(3), Text("serial_number".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("version_number".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("bus_current".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(6), Text("has_individual_address".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(7), Text("has_application_program".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(8), Text("is_accessory".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(9), Text("is_coupler".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(10), Text("is_power_supply".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(11), Text("is_ip_enabled".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(12), Text("is_power_line_repeater".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(13), Text("original_manufacturer".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(14), Text("source_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("hardware_source".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(14), Text("source_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_hardware_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("hardware2program".as_bytes()), Text("table".as_bytes()), Integer(10), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("manufacturer_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("hardware_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(3), Text("application_program_ref".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("medium_types".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("hash".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(6), Text("registration_number".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(7), Text("registration_status".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(8), Text("registration_signature".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(9), Text("source_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("hardware2program_program".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(3), Text("application_program_ref".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("hardware2program_source".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(9), Text("source_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_hardware2program_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("ingest_unknown".as_bytes()), Text("table".as_bytes()), Integer(8), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("id".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(1), Integer(0)],
            [Integer(1), Text("source_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("program_id".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(3), Text("xpath".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(4), Text("kind".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(5), Text("name".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(6), Text("occurrences".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(7), Text("sample".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("ingest_unknown_conflict_identity".as_bytes()), Integer(1), Text("c".as_bytes()), Integer(1)], xinfo: &[
                [Integer(0), Integer(1), Text("source_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(3), Text("xpath".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(5), Text("name".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(3), Integer(6), Text("occurrences".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(4), Integer(-2), Null, Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(5), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("ingest_unknown_construct_identity".as_bytes()), Integer(1), Text("c".as_bytes()), Integer(1)], xinfo: &[
                [Integer(0), Integer(1), Text("source_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-2), Null, Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(3), Text("xpath".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(3), Integer(4), Text("kind".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(4), Integer(5), Text("name".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(5), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("ingest_unknown_source".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(1), Text("source_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("legacy_diagnostic".as_bytes()), Text("table".as_bytes()), Integer(4), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("payload_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("ordinal".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("kind".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(3), Text("detail".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_legacy_diagnostic_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("payload_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("ordinal".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("legacy_source".as_bytes()), Text("payload_sha256".as_bytes()), Text("payload_sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("legacy_program".as_bytes()), Text("table".as_bytes()), Integer(3), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("program_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("payload_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("exim_program_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_legacy_program_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("legacy_source".as_bytes()), Text("payload_sha256".as_bytes()), Text("payload_sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("legacy_source".as_bytes()), Text("table".as_bytes()), Integer(8), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("payload_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("namespace".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("member_name".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(3), Text("member_kind".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(4), Text("format_version".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("exported_at".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(6), Text("producer".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(7), Text("charset".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_legacy_source_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("payload_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_legacy_source_2".as_bytes()), Integer(1), Text("u".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(1), Text("namespace".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("legacy_source_file".as_bytes()), Text("table".as_bytes()), Integer(4), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("payload_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("original_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("source_name".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(3), Integer(0)],
            [Integer(3), Text("encrypted".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_legacy_source_file_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("payload_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("original_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(2), Text("source_name".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(3), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("legacy_source".as_bytes()), Text("payload_sha256".as_bytes()), Text("payload_sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("manufacturer".as_bytes()), Text("table".as_bytes()), Integer(2), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("name".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_manufacturer_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("master_declaration".as_bytes()), Text("table".as_bytes()), Integer(13), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("source_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("ordinal".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("family".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(3), Text("element".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(4), Text("logical_id".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("xml_path".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(6), Text("byte_start".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(7), Text("byte_end".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(8), Text("main_number".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(9), Text("function_type_id".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(10), Text("missing_id".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(11), Text("orphan_parent".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(12), Text("observed_write".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("master_declaration_identity".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(2), Text("family".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(4), Text("logical_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(0), Text("source_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(3), Integer(1), Text("ordinal".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(4), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_master_declaration_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("source_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("ordinal".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("master_source".as_bytes()), Text("source_sha256".as_bytes()), Text("source_sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("master_source".as_bytes()), Text("table".as_bytes()), Integer(5), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("source_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("definition_version".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("history".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(3), Text("status".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(4), Text("reason".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_master_source_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("source_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("master_winner".as_bytes()), Text("table".as_bytes()), Integer(4), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("family".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("logical_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("source_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(3), Text("ordinal".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_master_winner_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("family".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("logical_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("master_declaration".as_bytes()), Text("source_sha256".as_bytes()), Text("source_sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
            [Integer(0), Integer(1), Text("master_declaration".as_bytes()), Text("ordinal".as_bytes()), Text("ordinal".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("module_def_argument".as_bytes()), Text("table".as_bytes()), Integer(8), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("program_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("module_def_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(3), Integer(0)],
            [Integer(3), Text("name".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("arg_type".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("allocates".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(6), Text("position".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(7), Text("extra".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("module_def_argument_scope".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("module_def_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_module_def_argument_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("module_def_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(2), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(3), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("package".as_bytes()), Text("table".as_bytes()), Integer(11), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("source_name".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("scheme".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(3), Text("size".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(4), Text("bytes".as_bytes()), Text("BLOB".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(5), Text("unknown_count".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(6), Text("translation_program_count".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Text("0".as_bytes()), Integer(0), Integer(0)],
            [Integer(7), Text("translation_catalog_count".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Text("0".as_bytes()), Integer(0), Integer(0)],
            [Integer(8), Text("translation_hardware_count".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Text("0".as_bytes()), Integer(0), Integer(0)],
            [Integer(9), Text("translation_master_count".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Text("0".as_bytes()), Integer(0), Integer(0)],
            [Integer(10), Text("dropped_datapoint_type_count".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Text("0".as_bytes()), Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_package_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("package_baggage_declaration".as_bytes()), Text("table".as_bytes()), Integer(12), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("package_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("index_path".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("ordinal".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(3), Integer(0)],
            [Integer(3), Text("baggage_id".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("name".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("target_path".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(6), Text("install_on_import".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(7), Text("time_info".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(8), Text("file_version".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(9), Text("resolution".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(10), Text("member_path".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(11), Text("detail".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_package_baggage_declaration_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("package_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("index_path".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(2), Text("ordinal".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(3), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("package_baggage_inventory".as_bytes()), Text("package_sha256".as_bytes()), Text("package_sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("package_baggage_inventory".as_bytes()), Text("table".as_bytes()), Integer(2), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("package_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("status".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_package_baggage_inventory_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("package_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("package".as_bytes()), Text("package_sha256".as_bytes()), Text("sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("package_baggage_payload".as_bytes()), Text("table".as_bytes()), Integer(11), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("package_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("member_path".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(3), Text("size".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(4), Text("media_class".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(5), Text("declarations".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(6), Text("nested_status".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(7), Text("nested_entries".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(8), Text("nested_expanded_size".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(9), Text("nested_encrypted_entries".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(10), Text("nested_archive_names".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_package_baggage_payload_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("package_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("member_path".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("source_file".as_bytes()), Text("sha256".as_bytes()), Text("sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
            [Integer(1), Integer(0), Text("package_baggage_inventory".as_bytes()), Text("package_sha256".as_bytes()), Text("package_sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("package_conflict".as_bytes()), Text("table".as_bytes()), Integer(7), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("package_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("ordinal".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("table_name".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(3), Text("logical_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(4), Text("kept_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(5), Text("other_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(6), Text("occurrence".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Text("1".as_bytes()), Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_package_conflict_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("package_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("ordinal".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("package".as_bytes()), Text("package_sha256".as_bytes()), Text("sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("package_install_count".as_bytes()), Text("table".as_bytes()), Integer(5), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("package_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("ordinal".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("category".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(3), Text("disposition".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(4), Text("count".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_package_install_count_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("package_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("ordinal".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_package_install_count_2".as_bytes()), Integer(1), Text("u".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("package_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(2), Text("category".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(3), Text("disposition".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(3), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("package_install_report".as_bytes()), Text("package_sha256".as_bytes()), Text("package_sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("package_install_diagnostic".as_bytes()), Text("table".as_bytes()), Integer(7), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("package_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("ordinal".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("kind".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(3), Text("archive_path".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(4), Text("xml_path".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(5), Text("detail".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(6), Text("occurrences".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_package_install_diagnostic_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("package_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("ordinal".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_package_install_diagnostic_2".as_bytes()), Integer(1), Text("u".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("package_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(2), Text("kind".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(3), Text("archive_path".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(3), Integer(4), Text("xml_path".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(4), Integer(5), Text("detail".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(5), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("package_install_report".as_bytes()), Text("package_sha256".as_bytes()), Text("package_sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("package_install_report".as_bytes()), Text("table".as_bytes()), Integer(5), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("package_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("report_version".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("status".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(3), Text("unknown_distinct".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(4), Text("unknown_occurrences".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_package_install_report_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("package_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("package".as_bytes()), Text("package_sha256".as_bytes()), Text("sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("package_install_unknown".as_bytes()), Text("table".as_bytes()), Integer(7), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("package_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("ordinal".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("xpath".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(3), Text("kind".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(4), Text("name".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(5), Text("occurrences".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(6), Text("sample".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_package_install_unknown_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("package_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("ordinal".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_package_install_unknown_2".as_bytes()), Integer(1), Text("u".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("package_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(2), Text("xpath".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(3), Text("kind".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(3), Integer(4), Text("name".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(4), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("package_install_report".as_bytes()), Text("package_sha256".as_bytes()), Text("package_sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("package_member".as_bytes()), Text("table".as_bytes()), Integer(6), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("package_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("ordinal".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("path".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(3), Text("role".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(4), Text("source_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(5), Text("size".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_package_member_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("package_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(2), Text("path".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_package_member_2".as_bytes()), Integer(1), Text("u".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("package_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("ordinal".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("source_file".as_bytes()), Text("source_sha256".as_bytes()), Text("sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
            [Integer(1), Integer(0), Text("package".as_bytes()), Text("package_sha256".as_bytes()), Text("sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("package_source_name".as_bytes()), Text("table".as_bytes()), Integer(2), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("package_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("source_name".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_package_source_name_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("package_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("source_name".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("package".as_bytes()), Text("package_sha256".as_bytes()), Text("sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("parameter".as_bytes()), Text("table".as_bytes()), Integer(22), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("program_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("name".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(3), Text("text".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("parameter_type_id".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("access".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(6), Text("value".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(7), Text("suffix".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(8), Text("code_segment".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(9), Text("offset".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(10), Text("bit_offset".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(11), Text("union_id".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(12), Text("union_size_in_bit".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(13), Text("own_offset".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(14), Text("own_bit_offset".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(15), Text("property_object_index".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(16), Text("property_object_type".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(17), Text("property_id".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(18), Text("property_offset".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(19), Text("property_bit_offset".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(20), Text("property_occurrence".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(21), Text("suffix_text".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_parameter_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("parameter_calculation_ref".as_bytes()), Text("table".as_bytes()), Integer(4), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("program_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("calculation_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("side".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(3), Integer(0)],
            [Integer(3), Text("parameter_ref_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(4), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("parameter_calculation_ref_member".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(3), Text("parameter_ref_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_parameter_calculation_ref_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("calculation_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(2), Text("side".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(3), Integer(3), Text("parameter_ref_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(4), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("parameter_ref".as_bytes()), Text("table".as_bytes()), Integer(9), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("program_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("parameter_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(3), Text("display_order".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("tag".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("text".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(6), Text("value".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(7), Text("access".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(8), Text("suffix_text".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("parameter_ref_program".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_parameter_ref_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("parameter_type".as_bytes()), Text("table".as_bytes()), Integer(9), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("program_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("name".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(3), Text("kind".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(4), Text("size_in_bit".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("base".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(6), Text("min_inclusive".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(7), Text("max_inclusive".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(8), Text("number_type".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_parameter_type_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("parameter_type_enum".as_bytes()), Text("table".as_bytes()), Integer(10), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("program_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("parameter_type_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(3), Text("value".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("text".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("display_order".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(6), Text("binary_value".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(7), Text("name".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(8), Text("icon".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(9), Text("picture_alignment".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("parameter_type_enum_type".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("parameter_type_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_parameter_type_enum_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("program_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(2), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("product".as_bytes()), Text("table".as_bytes()), Integer(11), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("manufacturer_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("hardware_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(3), Text("text".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("order_number".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(5), Text("is_rail_mounted".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(6), Text("width_in_millimeter".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(7), Text("default_language".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(8), Text("hash".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(9), Text("registration_status".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(10), Text("source_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("product_hardware".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(2), Text("hardware_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("product_source".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(10), Text("source_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_product_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("schema_meta".as_bytes()), Text("table".as_bytes()), Integer(2), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("key".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("value".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_schema_meta_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("key".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("source_file".as_bytes()), Text("table".as_bytes()), Integer(5), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("source_path".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("manufacturer_id".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(3), Text("len".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(4), Text("bytes".as_bytes()), Text("BLOB".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("source_file_manufacturer".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(2), Text("manufacturer_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_source_file_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("source_identity".as_bytes()), Text("table".as_bytes()), Integer(5), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("source_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("table_name".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("logical_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(3), Integer(0)],
            [Integer(3), Text("occurrence".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(4), Integer(0)],
            [Integer(4), Text("digest".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("source_identity_by_id".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(1), Text("table_name".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(2), Text("logical_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("sqlite_autoindex_source_identity_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("source_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("table_name".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(2), Text("logical_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(3), Integer(3), Text("occurrence".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(4), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("source_identity_scan".as_bytes()), Text("source_sha256".as_bytes()), Text("source_sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("source_identity_scan".as_bytes()), Text("table".as_bytes()), Integer(4), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("source_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("status".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
            [Integer(2), Text("reason".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(3), Text("scanner".as_bytes()), Text("INTEGER".as_bytes()), Integer(1), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_source_identity_scan_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("source_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("source_file".as_bytes()), Text("source_sha256".as_bytes()), Text("sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("source_parse_evidence".as_bytes()), Text("table".as_bytes()), Integer(1), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_source_parse_evidence_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("source_file".as_bytes()), Text("sha256".as_bytes()), Text("sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("source_producer".as_bytes()), Text("table".as_bytes()), Integer(4), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("source_sha256".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("root_namespace".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(2), Text("created_by".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(3), Text("tool_version".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_source_producer_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("source_sha256".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
            [Integer(0), Integer(0), Text("source_file".as_bytes()), Text("source_sha256".as_bytes()), Text("sha256".as_bytes()), Text("NO ACTION".as_bytes()), Text("NO ACTION".as_bytes()), Text("NONE".as_bytes())],
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("space_usage".as_bytes()), Text("table".as_bytes()), Integer(3), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("number".as_bytes()), Text("INTEGER".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(2), Text("text".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_space_usage_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("sqlite_schema".as_bytes()), Text("table".as_bytes()), Integer(5), Integer(0), Integer(0)],
        columns: &[
            [Integer(0), Text("type".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(1), Text("name".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(2), Text("tbl_name".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(3), Text("rootpage".as_bytes()), Text("INT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
            [Integer(4), Text("sql".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
        ],
        foreign_keys: &[
        ],
    },
    ExpectedRelation {
        table: [Text("main".as_bytes()), Text("translation".as_bytes()), Text("table".as_bytes()), Integer(6), Integer(0), Integer(1)],
        columns: &[
            [Integer(0), Text("scope".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(1), Integer(0)],
            [Integer(1), Text("scope_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(2), Integer(0)],
            [Integer(2), Text("language".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(3), Integer(0)],
            [Integer(3), Text("ref_id".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(4), Integer(0)],
            [Integer(4), Text("attribute_name".as_bytes()), Text("TEXT".as_bytes()), Integer(1), Null, Integer(5), Integer(0)],
            [Integer(5), Text("text".as_bytes()), Text("TEXT".as_bytes()), Integer(0), Null, Integer(0), Integer(0)],
        ],
        indexes: &[
            ExpectedIndex { list: [Text("sqlite_autoindex_translation_1".as_bytes()), Integer(1), Text("pk".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("scope".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("scope_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(2), Text("language".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(3), Integer(3), Text("ref_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(4), Integer(4), Text("attribute_name".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(5), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
            ExpectedIndex { list: [Text("translation_lookup".as_bytes()), Integer(0), Text("c".as_bytes()), Integer(0)], xinfo: &[
                [Integer(0), Integer(0), Text("scope".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(1), Integer(1), Text("scope_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(2), Integer(2), Text("language".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(3), Integer(3), Text("ref_id".as_bytes()), Integer(0), Text("BINARY".as_bytes()), Integer(1)],
                [Integer(4), Integer(-1), Null, Integer(0), Text("BINARY".as_bytes()), Integer(0)],
            ] },
        ],
        foreign_keys: &[
        ],
    },
];
