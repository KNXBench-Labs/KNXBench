//! Composes scoped typed readers and reconciles original reference and unknown identities.

use super::{xml::Document, *};
use crate::report::{UnknownConstruct, UnknownKind};
use rusqlite::{params, Connection};
use std::collections::{HashMap, HashSet};
use std::path::Path;

fn memory(bytes: &[u8]) -> Result<(Connection, String)> {
    let conn = crate::migration::open_and_migrate(Path::new(":memory:"))
        .map_err(|e| DistributionInspectionError::Reader(e.to_string()))?;
    conn.pragma_update(None, "temp_store", "MEMORY")
        .map_err(|e| DistributionInspectionError::Reader(e.to_string()))?;
    let filename: String = conn
        .query_row(
            "SELECT file FROM pragma_database_list WHERE name='main'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| DistributionInspectionError::Reader(e.to_string()))?;
    if !filename.is_empty() {
        return Err(DistributionInspectionError::Reader(
            "non-memory reader connection".into(),
        ));
    }
    let sha = crate::sha256_hex(bytes);
    conn.execute("INSERT INTO source_file(sha256,source_path,len,bytes) VALUES(?1,'derived-distribution-view',?2,?3)",params![sha, bytes.len() as i64, bytes]).map_err(|e| DistributionInspectionError::Reader(e.to_string()))?;
    Ok((conn, sha))
}
fn reader_error(e: crate::ProductDbError) -> DistributionInspectionError {
    DistributionInspectionError::Reader(e.to_string())
}

fn unknown_keys(
    doc: &Document<'_>,
    included: &HashSet<usize>,
    unknown: &[UnknownConstruct],
    keys: &mut HashSet<(usize, Option<String>)>,
) {
    let mut elements = HashSet::new();
    let mut attributes = HashMap::<String, HashSet<&str>>::new();
    let mut legacy_attributes = HashSet::new();
    for u in unknown {
        match u.kind {
            UnknownKind::Element => {
                elements.insert(format!("{}/{}", u.xpath, u.name));
            }
            UnknownKind::Attribute => {
                attributes
                    .entry(u.xpath.clone())
                    .or_default()
                    .insert(u.name.as_str());
                // These two readers use fixed grammar paths rather than actual nested paths.
                if u.xpath.contains("/Catalog/") || u.xpath.contains("/Hardware/") {
                    legacy_attributes
                        .insert((u.xpath.rsplit('/').next().unwrap_or(""), u.name.as_str()));
                }
            }
        }
    }
    // Paths are built only when a reader reported path-keyed unknowns at all.
    let by_path = !elements.is_empty() || !attributes.is_empty();
    for &id in included {
        let n = &doc.nodes[id];
        let path = if by_path {
            doc.path(id, false)
        } else {
            String::new()
        };
        if by_path && elements.contains(&path) {
            keys.insert((id, None));
        }
        let at_path = attributes.get(&path);
        for a in &n.attrs {
            if at_path.is_some_and(|s| s.contains(a.raw.as_str()))
                || legacy_attributes.contains(&(n.local, a.raw.as_str()))
            {
                keys.insert((id, Some(a.raw.clone())));
            }
        }
    }
}

/// Evaluator containers (design D10, ADR-0041 layout) structure Dynamic but declare no attributes.
const DYNAMIC_CONTAINERS: &[&str] = &["Dynamic", "ChannelIndependentBlock", "Rows", "Columns"];

/// Dynamic verdicts reuse the existing Dynamic grammar. An uninspected Dynamic subtree is
/// inventoried as unknown, never counted as covered.
fn dynamic_unknowns(
    doc: &Document<'_>,
    scope: usize,
    included: &HashSet<usize>,
    inspected: bool,
    unknown: &mut HashSet<(usize, Option<String>)>,
) {
    for root in scope + 1..doc.nodes[scope].end_node {
        let n = &doc.nodes[root];
        if n.local != "Dynamic" || n.ns.as_ref() != doc.namespace || !included.contains(&root) {
            continue;
        }
        let parent = n.parent.map(|p| doc.nodes[p].local);
        if !matches!(parent, Some("ApplicationProgram" | "ModuleDef")) {
            continue;
        }
        for i in root..n.end_node {
            if !included.contains(&i) {
                continue;
            }
            let node = &doc.nodes[i];
            if !inspected {
                unknown.insert((i, None));
                continue;
            }
            let kind = node.local;
            let parent = node.parent.map_or("", |p| doc.nodes[p].local);
            if !DYNAMIC_CONTAINERS.contains(&kind)
                && !crate::dynamic::parse::kind_is_modelled(kind)
                && !crate::parameter_layout::layout_element_is_known(parent, kind)
            {
                unknown.insert((i, None));
            }
            // The grid reader (ADR-0137) owns layout attributes the Dynamic grammar does not read.
            let mut in_block = false;
            let mut next = node.parent;
            while let Some(p) = next.filter(|&p| p >= root) {
                let b = &doc.nodes[p];
                in_block |= b.local == "ParameterBlock" && b.attr("Id").is_some();
                next = b.parent;
            }
            for a in node.attrs.iter().filter(|a| !a.raw.contains(':')) {
                if !crate::dynamic::parse::attribute_is_known(kind, &a.raw, Some(&a.value))
                    && !crate::parameter_layout::layout_attribute_is_known(kind, &a.raw, in_block)
                {
                    unknown.insert((i, Some(a.raw.clone())));
                }
            }
        }
    }
}

pub(super) fn inspect(
    doc: &Document<'_>,
    limits: &DistributionLimits,
    report: &mut DistributionInspection,
) -> Result<()> {
    let mut covered = HashSet::new();
    let mut unknown = HashSet::<(usize, Option<String>)>::new();
    let mut scope_count = 0;
    let before_sections = report.sections.len();
    for (id, n) in doc.nodes.iter().enumerate() {
        let source_metadata = doc.is_path(id, &["KNX"])
            || doc.is_path(id, &["KNX", "ManufacturerData", "Manufacturer"])
            || doc.is_path(
                id,
                &[
                    "KNX",
                    "ManufacturerData",
                    "Manufacturer",
                    "ApplicationPrograms",
                    "ApplicationProgram",
                ],
            );
        if source_metadata {
            for attr in &n.attrs {
                if matches!(
                    attr.raw.as_str(),
                    "CreatedBy"
                        | "ToolVersion"
                        | "Version"
                        | "RefId"
                        | "Id"
                        | "ApplicationNumber"
                        | "ApplicationVersion"
                        | "MaskVersion"
                ) {
                    doc.push(
                        &mut report.declared_metadata,
                        DistributionDeclaredMetadata {
                            attribute: attr.raw.clone(),
                            value: attr.value.to_string(),
                            source: doc.coordinate(id)?,
                        },
                    )?;
                }
            }
        }
        if doc.is_path(id, &["KNX"])
            || doc.is_path(id, &["KNX", "ManufacturerData"])
            || doc.is_path(id, &["KNX", "ManufacturerData", "Manufacturer"])
        {
            covered.insert(id);
            for a in &n.attrs {
                let known = match n.local {
                    "KNX" => matches!(a.raw.as_str(), "CreatedBy" | "ToolVersion" | "Version"),
                    "Manufacturer" => a.raw == "RefId",
                    _ => false,
                };
                if !known {
                    unknown.insert((id, Some(a.raw.clone())));
                }
            }
        }
        let kind = match n.local {
            "Catalog" => DistributionSectionKind::Catalog,
            "ApplicationPrograms" => DistributionSectionKind::ApplicationPrograms,
            "Hardware" => DistributionSectionKind::Hardware,
            "Baggages" => DistributionSectionKind::Baggages,
            _ => continue,
        };
        if !doc.is_path(id, &["KNX", "ManufacturerData", "Manufacturer", n.local]) {
            continue;
        }
        scope_count += 1;
        if scope_count > limits.max_scopes {
            return Err(limit("reader scopes"));
        }
        covered.insert(id);
        for attr in &n.attrs {
            unknown.insert((id, Some(attr.raw.clone())));
        }
        let manufacturer = doc
            .manufacturer(id)
            .and_then(|i| doc.nodes[i].attr("RefId"));
        let section_index = report.sections.len();
        doc.push(
            &mut report.sections,
            DistributionSection {
                kind,
                state: DistributionPassState::Preserved,
                declared_manufacturer_id: manufacturer.clone(),
                source: doc.coordinate(id)?,
            },
        )?;
        if kind == DistributionSectionKind::Baggages {
            for child in doc.children(id).filter(|&i| {
                doc.nodes[i].ns.as_ref() == doc.namespace && doc.nodes[i].local == "Baggage"
            }) {
                covered.insert(child);
                for a in &doc.nodes[child].attrs {
                    if !matches!(
                        a.raw.as_str(),
                        "Id" | "Name" | "TargetPath" | "FileInfo" | "Hash" | "InstallOnImport"
                    ) {
                        unknown.insert((child, Some(a.raw.clone())));
                    }
                }
            }
            continue;
        }
        let scopes = if kind == DistributionSectionKind::ApplicationPrograms {
            doc.children(id)
                .filter(|&i| {
                    doc.nodes[i].ns.as_ref() == doc.namespace
                        && doc.nodes[i].local == "ApplicationProgram"
                })
                .collect::<Vec<_>>()
        } else {
            vec![id]
        };
        // The container has been inspected even when it declares zero applications.
        report.sections[section_index].state = DistributionPassState::Parsed;
        for scope in scopes {
            scope_count += 1;
            if scope_count > limits.max_scopes {
                return Err(limit("reader scopes"));
            }
            let (view, included) = doc.view(scope, limits)?;
            let (conn, sha) = memory(&view)?;
            let mut pass = DistributionReaderPass {
                reader: String::new(),
                state: DistributionPassState::Parsed,
                source: doc.coordinate(scope)?,
                original_sha256: doc.sha256.clone(),
                derived_sha256: sha.clone(),
                derived_size: view.len() as u64,
                transformation: "knx-expanded-role-view-v1".into(),
                diagnostics: Vec::new(),
            };
            let outcome = match kind {
                DistributionSectionKind::Catalog => {
                    pass.reader = "catalog".into();
                    crate::parse::catalog::ingest_catalog(
                        &conn,
                        &sha,
                        "derived-distribution-view",
                        &view,
                    )
                    .map(|r| r.unknown)
                }
                DistributionSectionKind::Hardware => {
                    pass.reader = "hardware".into();
                    crate::parse::hardware::ingest_hardware(
                        &conn,
                        &sha,
                        "derived-distribution-view",
                        &view,
                    )
                    .map(|r| r.unknown)
                }
                DistributionSectionKind::ApplicationPrograms => {
                    pass.reader = "program-static".into();
                    crate::parse::program::ingest_program(
                        &conn,
                        &sha,
                        "derived-distribution-view",
                        &view,
                    )
                    .map(|r| r.unknown)
                }
                DistributionSectionKind::Baggages => unreachable!(),
            };
            let parsed = match outcome {
                Ok(u) => {
                    unknown_keys(doc, &included, &u, &mut unknown);
                    true
                }
                Err(e) => {
                    pass.state = DistributionPassState::PartiallyEvaluated;
                    pass.diagnostics.push(e.to_string());
                    report.sections[section_index].state =
                        DistributionPassState::PartiallyEvaluated;
                    for &i in &included {
                        unknown.insert((i, None));
                    }
                    false
                }
            };
            covered.extend(included.iter().copied());
            // Catalog's existing reader intentionally ignores unknown elements; inventory them here.
            // Hardware likewise does not return a complete element inventory.
            for &i in &included {
                let name = doc.nodes[i].local;
                let known = match kind {
                    DistributionSectionKind::Catalog => {
                        matches!(name, "Catalog" | "CatalogSection" | "CatalogItem")
                    }
                    DistributionSectionKind::Hardware => matches!(
                        name,
                        "Hardware"
                            | "Products"
                            | "Product"
                            | "Hardware2Programs"
                            | "Hardware2Program"
                            | "ApplicationProgramRef"
                    ),
                    _ => true,
                };
                if !known {
                    unknown.insert((i, None));
                }
            }
            // Qualified attributes are never handed to the local-name legacy readers.
            for &i in &included {
                for a in &doc.nodes[i].attrs {
                    if a.raw.contains(':') {
                        unknown.insert((i, Some(a.raw.clone())));
                    }
                }
            }
            match kind {
                DistributionSectionKind::Catalog => catalog(doc, scope, report)?,
                DistributionSectionKind::Hardware => hardware(doc, scope, report)?,
                DistributionSectionKind::ApplicationPrograms => application(
                    doc,
                    scope,
                    &conn,
                    &sha,
                    &view,
                    parsed,
                    limits,
                    report,
                    &mut unknown,
                    &included,
                )?,
                DistributionSectionKind::Baggages => {}
            }
            // Reader reuse must not enter generic ingest's authority/completeness path.
            for table in ["package", "source_parse_evidence"] {
                let count: i64 = conn
                    .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                    .map_err(|e| DistributionInspectionError::Reader(e.to_string()))?;
                if count != 0 {
                    return Err(DistributionInspectionError::Reader(
                        "unexpected installation/parse marker".into(),
                    ));
                }
            }
            doc.push(&mut report.reader_passes, pass)?;
        }
    }
    if report.sections.len() == before_sections {
        return Err(DistributionInspectionError::UnrecognizedSource);
    }
    for (id, n) in doc.nodes.iter().enumerate() {
        if !covered.contains(&id) {
            unknown.insert((id, None));
        }
        if !covered.contains(&id) || unknown.contains(&(id, None)) {
            for a in &n.attrs {
                unknown.insert((id, Some(a.raw.clone())));
            }
        }
    }
    let mut ordered = unknown.into_iter().collect::<Vec<_>>();
    ordered.sort();
    for (id, attr) in ordered {
        let n = &doc.nodes[id];
        let (kind, expanded_name, raw_name) = if let Some(attr) = attr {
            let a = n
                .attrs
                .iter()
                .find(|a| a.raw == attr)
                .expect("original attribute");
            ("attribute", a.expanded.clone(), a.raw.clone())
        } else {
            (
                "element",
                format!("{{{}}}{}", n.ns, n.local),
                n.raw.to_string(),
            )
        };
        doc.push(
            &mut report.unknowns,
            DistributionUnknown {
                kind: kind.into(),
                expanded_name,
                raw_name,
                source: doc.coordinate(id)?,
            },
        )?;
        if report.unknowns.len() > limits.max_output_records {
            return Err(limit("output records"));
        }
    }
    references(doc, limits, report)?;
    let mut unknown_positions = report
        .unknowns
        .iter()
        .filter(|u| u.source.member_ordinal == doc.member)
        .map(|u| doc.nodes[u.source.location as usize - 1].start as u64)
        .collect::<Vec<_>>();
    let mut app_positions = report
        .applications
        .iter()
        .filter(|a| a.source.member_ordinal == doc.member)
        .map(|a| doc.nodes[a.source.location as usize - 1].start as u64)
        .collect::<Vec<_>>();
    unknown_positions.sort_unstable();
    app_positions.sort_unstable();
    let in_section = |positions: &[u64], section: &DistributionSection| {
        positions
            .get(positions.partition_point(|&p| {
                p < doc.nodes[section.source.location as usize - 1].start as u64
            }))
            .is_some_and(|&p| p < doc.nodes[section.source.location as usize - 1].end as u64)
    };
    for section in &mut report.sections[before_sections..] {
        if section.state != DistributionPassState::Preserved
            && (in_section(&unknown_positions, section)
                || (section.kind == DistributionSectionKind::ApplicationPrograms
                    && in_section(&app_positions, section)))
        {
            section.state = DistributionPassState::PartiallyEvaluated;
        }
    }
    output_limit(report, limits)
}

fn catalog(doc: &Document<'_>, scope: usize, report: &mut DistributionInspection) -> Result<()> {
    for i in scope + 1..doc.nodes[scope].end_node {
        let n = &doc.nodes[i];
        if n.local == "CatalogItem" && doc.projected(i, scope) {
            doc.push(
                &mut report.catalog_items,
                DistributionCatalogItem {
                    id: n.attr("Id"),
                    name: n.attr("Name"),
                    product_ref_id: n.attr("ProductRefId"),
                    hardware2program_ref_id: n.attr("Hardware2ProgramRefId"),
                    source: doc.coordinate(i)?,
                },
            )?;
        }
    }
    Ok(())
}
fn hardware(doc: &Document<'_>, scope: usize, report: &mut DistributionInspection) -> Result<()> {
    for i in doc
        .children(scope)
        .filter(|&i| doc.nodes[i].local == "Hardware" && doc.nodes[i].ns.as_ref() == doc.namespace)
    {
        let n = &doc.nodes[i];
        let mut item = DistributionHardware {
            id: n.attr("Id"),
            name: n.attr("Name"),
            product_ids: Vec::new(),
            program_links: Vec::new(),
            source: doc.coordinate(i)?,
        };
        for container in doc
            .children(i)
            .filter(|&i| doc.nodes[i].ns.as_ref() == doc.namespace)
        {
            for child in doc
                .children(container)
                .filter(|&i| doc.nodes[i].ns.as_ref() == doc.namespace)
            {
                let n = &doc.nodes[child];
                match (doc.nodes[container].local, n.local) {
                    ("Products", "Product") => match n.attr("Id") {
                        Some(id) => item.product_ids.push(id),
                        None => missing_identity(doc, child, "Product without Id", report)?,
                    },
                    ("Hardware2Programs", "Hardware2Program") => {
                        let mut links = Vec::new();
                        for i in doc.children(child).filter(|&i| {
                            doc.nodes[i].ns.as_ref() == doc.namespace
                                && doc.nodes[i].local == "ApplicationProgramRef"
                        }) {
                            match doc.nodes[i].attr("RefId") {
                                Some(id) => links.push(id),
                                None => missing_identity(
                                    doc,
                                    i,
                                    "ApplicationProgramRef without RefId",
                                    report,
                                )?,
                            }
                        }
                        item.program_links.push(DistributionProgramLink {
                            id: n.attr("Id"),
                            application_program_ref_ids: links,
                            source: doc.coordinate(child)?,
                        });
                    }
                    _ => {}
                }
            }
        }
        doc.push(&mut report.hardware, item)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn application(
    doc: &Document<'_>,
    scope: usize,
    conn: &Connection,
    sha: &str,
    view: &[u8],
    parsed: bool,
    limits: &DistributionLimits,
    report: &mut DistributionInspection,
    unknown: &mut HashSet<(usize, Option<String>)>,
    included: &HashSet<usize>,
) -> Result<()> {
    let n = &doc.nodes[scope];
    let mut app = DistributionApplication {
        id: n.attr("Id"),
        name: n.attr("Name"),
        declared_manufacturer_id: doc
            .manufacturer(scope)
            .and_then(|i| doc.nodes[i].attr("RefId")),
        source: doc.coordinate(scope)?,
        parameter_fallbacks: Vec::new(),
        dynamic_state: DistributionPassState::NotAttempted,
        active_parameter_ref_ids: Vec::new(),
        active_parameter_refs: Vec::new(),
    };
    let mut entities = HashMap::<(Option<usize>, String, String), Vec<usize>>::new();
    let mut reader_ids = HashSet::new();
    let mut ambiguous = false;
    for i in scope + 1..n.end_node {
        if !included.contains(&i) || !doc.declaration(i) {
            continue;
        }
        let node = &doc.nodes[i];
        if let Some(id) = node.attr("Id") {
            if !reader_ids.insert((node.local.to_string(), id.clone())) {
                ambiguous = true;
            }
            entities
                .entry((doc.entity_scope(i), node.local.to_string(), id))
                .or_default()
                .push(i);
        }
    }

    for i in scope + 1..n.end_node {
        let node = &doc.nodes[i];
        if node.local != "ParameterRef" || !included.contains(&i) || !doc.declaration(i) {
            continue;
        }
        let Some(ref_id) = node.attr("Id") else {
            missing_identity(
                doc,
                i,
                "ParameterRef without Id has no fallback row",
                report,
            )?;
            continue;
        };
        let target = node.attr("RefId").unwrap_or_default();
        let candidates = entities
            .get(&(doc.entity_scope(i), "Parameter".into(), target.clone()))
            .map_or(&[][..], Vec::as_slice);
        let ref_value = node.attr("Value");
        // A missing or duplicated target is not the same as a resolved Parameter without Value.
        let (value, origin) = match (ref_value, candidates) {
            (Some(v), _) => (Some(v), DistributionFallbackOrigin::ParameterRef),
            (None, []) => (None, DistributionFallbackOrigin::DanglingParameter),
            (None, [one]) => match doc.nodes[*one].attr("Value") {
                Some(v) => (Some(v), DistributionFallbackOrigin::Parameter),
                None => (None, DistributionFallbackOrigin::Absent),
            },
            (None, _) => (None, DistributionFallbackOrigin::AmbiguousParameter),
        };

        doc.push(
            &mut app.parameter_fallbacks,
            DistributionParameterFallback {
                parameter_ref_id: ref_id,
                parameter_id: target,
                value,
                origin,
                source: doc.coordinate(i)?,
            },
        )?;
    }
    if parsed && !ambiguous && app.id.as_deref().is_some_and(|id| !id.is_empty()) {
        super::bounds::admit_dynamic(
            doc,
            scope,
            included,
            &app.parameter_fallbacks,
            limits.max_dynamic_work_bytes,
        )?;
        let id = app.id.as_deref().unwrap();
        match crate::dynamic::parse::parse_dynamic_trees(
            conn,
            sha,
            "derived-distribution-view",
            view,
        ) {
            Ok(_) => {
                dynamic_unknowns(doc, scope, included, true, unknown);
                let trees =
                    crate::dynamic::evaluate::load_program_trees(conn, id).map_err(reader_error)?;
                let values = crate::dynamic::evaluate::resolve_values(conn, id, &HashMap::new())
                    .map_err(reader_error)?;
                let activation = crate::dynamic::evaluate::evaluate(&trees, &values);
                app.dynamic_state = DistributionPassState::PartiallyEvaluated;
                // Every copied row is charged as it is made; scope chains cannot amplify unseen.
                for reference in &activation.parameter_refs {
                    doc.push(&mut app.active_parameter_ref_ids, reference.ref_id.clone())?;
                }
                for reference in &activation.parameter_refs {
                    let mut module_scope = Vec::new();
                    let mut next = reference.scope.as_deref();
                    while let Some(scope) = next {
                        module_scope.push(DistributionModuleScope {
                            module_node: scope.module_node,
                            module_id: scope.module_id.clone(),
                            module_def_id: scope.module_def_id.clone(),
                        });
                        next = scope.parent.as_deref();
                    }
                    module_scope.reverse();
                    doc.push(
                        &mut app.active_parameter_refs,
                        DistributionActiveRef {
                            ref_id: reference.ref_id.clone(),
                            module_scope,
                        },
                    )?;
                }
                for diagnostic in &activation.diagnostics {
                    let mut detail = format!("{diagnostic:?}");
                    if detail.len() > MAX_DIAGNOSTIC_DETAIL_BYTES {
                        let mut end = MAX_DIAGNOSTIC_DETAIL_BYTES;
                        while !detail.is_char_boundary(end) {
                            end -= 1;
                        }
                        detail.truncate(end);
                        detail.push_str(" (truncated; see immutable original scope)");
                    }

                    doc.push(
                        &mut report.diagnostics,
                        DistributionDiagnostic {
                            code: "dynamicEvaluatedSubset".into(),
                            source: Some(doc.coordinate(scope)?),
                            detail,
                        },
                    )?;
                }
            }
            Err(e) => {
                dynamic_unknowns(doc, scope, included, false, unknown);
                doc.push(
                    &mut report.diagnostics,
                    DistributionDiagnostic {
                        code: "dynamicReaderFailed".into(),
                        source: Some(doc.coordinate(scope)?),
                        detail: e.to_string(),
                    },
                )?
            }
        }
    } else {
        dynamic_unknowns(doc, scope, included, false, unknown);
        doc.push(&mut report.diagnostics, DistributionDiagnostic { code:"ambiguousOrUnparsedApplication".into(),source:Some(doc.coordinate(scope)?),detail:"Dynamic not attempted: duplicate declarations, missing application identity or failed Static pass".into() })?;
    }
    if app.parameter_fallbacks.len() > limits.max_references {
        return Err(limit("parameter fallbacks"));
    }
    doc.push(&mut report.reader_passes, DistributionReaderPass {
        reader:"dynamic-subset".into(),state:app.dynamic_state,source:doc.coordinate(scope)?,
        original_sha256:doc.sha256.clone(),derived_sha256:sha.into(),derived_size:view.len() as u64,transformation:"knx-expanded-role-view-v1".into(),
        diagnostics:vec!["Source fallback observations only; no Assign/calculation/validation/handler initialization or write authority".into()],
    })?;
    // Nested rows were charged individually; the application row below charges them again.
    let cap = limits.max_report_bytes;
    let nested = charged(&app.parameter_fallbacks, cap)?
        + charged(&app.active_parameter_ref_ids, cap)?
        + charged(&app.active_parameter_refs, cap)?;
    doc.uncharge(nested);
    doc.push(&mut report.applications, app)?;
    Ok(())
}

/// Exactly what `Document::push` charged for these rows.
fn charged<T: serde::Serialize>(rows: &[T], cap: usize) -> Result<usize> {
    rows.iter().try_fold(0usize, |sum, row| {
        Ok(sum + super::provenance::json_size(row, cap)? + 1)
    })
}

fn missing_identity(
    doc: &Document<'_>,
    id: usize,
    detail: &str,
    report: &mut DistributionInspection,
) -> Result<()> {
    doc.push(
        &mut report.diagnostics,
        DistributionDiagnostic {
            code: "missingDeclaredIdentity".into(),
            source: Some(doc.coordinate(id)?),
            detail: detail.into(),
        },
    )
}

fn target_kind(node: &str, attr: &str) -> Option<&'static str> {
    match (node, attr) {
        (_, "ProductRefId") => Some("Product"),
        (_, "Hardware2ProgramRefId") => Some("Hardware2Program"),
        ("ApplicationProgramRef", "RefId") => Some("ApplicationProgram"),
        ("ParameterRef", "RefId") => Some("Parameter"),
        (_, "ParameterType") => Some("ParameterType"),
        ("ParameterRefRef", "RefId") | (_, "ParamRefId" | "TextParameterRefId") => {
            Some("ParameterRef")
        }
        ("ComObjectRef", "RefId") => Some("ComObject"),
        ("ComObjectRefRef", "RefId") => Some("ComObjectRef"),
        ("Module", "RefId") => Some("ModuleDef"),
        _ => None,
    }
}
fn references(
    doc: &Document<'_>,
    limits: &DistributionLimits,
    report: &mut DistributionInspection,
) -> Result<()> {
    let mut index = HashMap::<(Option<usize>, Option<usize>, String, String), Vec<usize>>::new();
    for (i, n) in doc.nodes.iter().enumerate() {
        if n.ns.as_ref() != doc.namespace {
            continue;
        }
        let Some(manufacturer) = doc.manufacturer(i) else {
            continue;
        };
        if !doc.declaration(i) {
            continue;
        }
        if let Some(id) = n.attr("Id") {
            let app = if n.local == "ApplicationProgram" {
                None
            } else {
                doc.entity_scope(i)
            };
            doc.push(
                &mut report.identities,
                DistributionIdentity {
                    kind: n.local.to_string(),
                    declared_id: id.clone(),
                    source: doc.coordinate(i)?,
                    manufacturer_source: doc.coordinate(manufacturer)?,
                    application_scope: app.map(|i| doc.coordinate(i)).transpose()?,
                },
            )?;
            if id.is_empty() {
                doc.push(
                    &mut report.diagnostics,
                    DistributionDiagnostic {
                        code: "emptyDeclaredIdentity".into(),
                        source: Some(doc.coordinate(i)?),
                        detail: "Empty source identity is preserved but is not a reference target"
                            .into(),
                    },
                )?;
                continue;
            }
            index
                .entry((Some(manufacturer), app, n.local.to_string(), id))
                .or_default()
                .push(i);
        }
    }
    for candidates in index.values().filter(|c| c.len() > 1) {
        for &id in candidates {
            doc.push(&mut report.diagnostics, DistributionDiagnostic {
                code: "duplicateIdentity".into(),
                source: Some(doc.coordinate(id)?),
                detail:
                    "declared identity is not unique in its original manufacturer/application scope"
                        .into(),
            })?;
        }
    }
    report
        .diagnostics
        .sort_by_key(|d| d.source.as_ref().map(|s| (s.member_ordinal, s.location)));
    let mut reference_count = 0;
    for (i, n) in doc.nodes.iter().enumerate() {
        let Some(manufacturer) = doc.manufacturer(i) else {
            continue;
        };
        if !doc.semantic_node(i) {
            continue;
        }
        for a in &n.attrs {
            if a.raw.contains(':') {
                continue;
            }
            let kind = target_kind(n.local, &a.raw);
            let external = matches!(
                a.raw.as_str(),
                "MaskVersion" | "MediumTypes" | "DatapointType"
            );
            if kind.is_none() && !external {
                continue;
            }
            reference_count += 1;
            if reference_count > limits.max_references {
                return Err(limit("references"));
            }
            let app = if matches!(
                kind,
                Some("Product" | "Hardware2Program" | "ApplicationProgram")
            ) {
                None
            } else {
                doc.entity_scope(i)
            };
            let lookup = |scope| {
                kind.and_then(|k| {
                    index.get(&(Some(manufacturer), scope, k.into(), a.value.to_string()))
                })
                .cloned()
                .unwrap_or_default()
            };
            let mut candidates = lookup(app);
            let mut basis = DistributionResolutionBasis::OriginalScope;
            // Module parameters may name application-level types: nearest scope first, then lexical.
            if candidates.is_empty()
                && kind == Some("ParameterType")
                && app.is_some_and(|s| doc.nodes[s].local == "ModuleDef")
            {
                candidates = lookup(doc.application(i));
                if !candidates.is_empty() {
                    basis = DistributionResolutionBasis::EnclosingApplicationScope;
                }
            }
            reference_count = reference_count
                .checked_add(candidates.len())
                .ok_or_else(|| limit("reference candidates"))?;
            if reference_count > limits.max_references {
                return Err(limit("reference candidates"));
            }
            let state = if external {
                DistributionReferenceState::External
            } else {
                match candidates.len() {
                    0 => DistributionReferenceState::Dangling,
                    1 => DistributionReferenceState::Resolved,
                    _ => DistributionReferenceState::Ambiguous,
                }
            };
            doc.push(
                &mut report.references,
                DistributionReference {
                    attribute: a.raw.clone(),
                    declared_target: a.value.to_string(),
                    target_kind: kind.map(str::to_owned),
                    manufacturer_source: doc.coordinate(manufacturer)?,
                    resolution_basis: if external {
                        DistributionResolutionBasis::ExternalUnperformed
                    } else {
                        basis
                    },
                    source: doc.coordinate(i)?,
                    state,
                    candidates: candidates
                        .into_iter()
                        .map(|i| doc.coordinate(i))
                        .collect::<Result<Vec<_>>>()?,
                },
            )?;
            if report.references.len() > limits.max_output_records {
                return Err(limit("output records"));
            }
        }
    }
    Ok(())
}

pub(super) fn output_limit(
    report: &DistributionInspection,
    limits: &DistributionLimits,
) -> Result<()> {
    let count = output_count(report);
    if count > limits.max_output_records {
        Err(limit(&format!(
            "output records: {count} > {}",
            limits.max_output_records
        )))
    } else {
        Ok(())
    }
}
pub(super) fn output_count(report: &DistributionInspection) -> usize {
    report
        .documents
        .iter()
        .map(|d| 1 + d.names.len() + d.locations.len())
        .sum::<usize>()
        + report.sections.len()
        + report.catalog_items.len()
        + report
            .hardware
            .iter()
            .map(|h| {
                1 + h.product_ids.len()
                    + h.program_links
                        .iter()
                        .map(|p| 1 + p.application_program_ref_ids.len())
                        .sum::<usize>()
            })
            .sum::<usize>()
        + report.applications.len()
        + report.unknowns.len()
        + report.references.len()
        + report.identities.len()
        + report.reader_passes.len()
        + report.diagnostics.len()
        + report.members.len()
        + report.declared_metadata.len()
        + report.preserved_content.len()
        + report
            .applications
            .iter()
            .map(|a| {
                a.parameter_fallbacks.len()
                    + a.active_parameter_ref_ids.len()
                    + a.active_parameter_refs.len()
                    + a.active_parameter_refs
                        .iter()
                        .map(|r| r.module_scope.len())
                        .sum::<usize>()
            })
            .sum::<usize>()
        + report
            .references
            .iter()
            .map(|r| r.candidates.len())
            .sum::<usize>()
}
