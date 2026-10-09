//! Reconstructs declarative System-B AP1 sequences without granting execution capability.
use std::collections::{BTreeMap, BTreeSet};

mod xml;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use xml::parse;

use crate::{load_source_file, sha256_hex, ProductDbError};

pub const MAX_XML_BYTES: usize = 8 * 1024 * 1024;
const MAX_STEPS: usize = 2048;
const MAX_OUTPUT: usize = 1024 * 1024;

/// Source-bearing local data; never included in reduced contribution exports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub namespace: String,
    pub name: String,
    pub attributes: BTreeMap<String, String>,
    pub children: Vec<Node>,
    pub text: String,
    pub byte_start: u64,
    pub byte_end: u64,
}
impl Node {
    fn matches(&self, ns: &str, name: &str) -> bool {
        self.namespace == ns && self.name == name
    }
    fn children_named<'a>(&'a self, ns: &'a str, name: &'a str) -> impl Iterator<Item = &'a Node> {
        self.children.iter().filter(move |n| n.matches(ns, name))
    }
    fn attr(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(String::as_str)
    }
    // Conservative JSON upper bound, charged before cloning attacker-controlled subtrees.
    fn weight(&self) -> usize {
        256 + 6 * (self.namespace.len() + self.name.len() + self.text.len())
            + self
                .attributes
                .iter()
                .map(|(k, v)| 32 + 6 * (k.len() + v.len()))
                .sum::<usize>()
            + self.children.iter().map(Node::weight).sum::<usize>()
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub source: String,
    pub sha256: String,
    pub namespace: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Origin {
    pub source: String,
    pub sha256: String,
    pub location: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub node: Node,
    pub origin: Origin,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    pub code: String,
    pub location: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Merge {
    pub merge_id: String,
    pub disposition: String,
    pub origin: Origin,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    pub format_version: u32,
    pub program_id: String,
    pub mask: String,
    pub style: String,
    pub variant: String,
    pub status: String,
    /// A structural expansion can never be submitted to a download executor.
    pub executable: bool,
    pub sources: Vec<Source>,
    pub template_attributes: BTreeMap<String, String>,
    pub program_attributes: BTreeMap<String, String>,
    pub unplaced_declarations: Vec<Step>,
    pub omitted_issue_count: usize,
    pub steps: Vec<Step>,
    pub merges: Vec<Merge>,
    pub issues: Vec<Issue>,
}
impl Resolution {
    fn new(id: &str, mask: &str, style: &str) -> Self {
        Self {
            format_version: 1,
            program_id: id.into(),
            mask: mask.into(),
            style: style.into(),
            variant: "ap1".into(),
            status: "unavailable".into(),
            executable: false,
            sources: vec![],
            template_attributes: BTreeMap::new(),
            program_attributes: BTreeMap::new(),
            unplaced_declarations: vec![],
            omitted_issue_count: 0,
            steps: vec![],
            merges: vec![],
            issues: vec![],
        }
    }
    fn issue(&mut self, code: &str, location: &str) {
        if self.issues.len() >= MAX_STEPS {
            self.omitted_issue_count += 1;
        }
        if self.issues.len() < MAX_STEPS {
            self.issues.push(Issue {
                code: code.into(),
                location: location.into(),
            });
        }
    }
}
fn path<'a>(root: &'a Node, names: &[&'a str]) -> Vec<&'a Node> {
    let mut nodes = vec![root];
    for name in names {
        nodes = nodes
            .into_iter()
            .flat_map(|n| n.children_named(&root.namespace, name))
            .collect();
    }
    nodes
}
fn unique<'a>(nodes: Vec<&'a Node>, code: &'static str) -> Result<&'a Node, &'static str> {
    if nodes.len() != 1 {
        Err(code)
    } else {
        Ok(nodes[0])
    }
}
fn loaded_source(
    conn: &Connection,
    sha: &str,
) -> Result<Result<Vec<u8>, &'static str>, ProductDbError> {
    let size: Option<(i64, i64)> = conn
        .query_row(
            "SELECT len,length(bytes) FROM source_file WHERE sha256=?1",
            [sha],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match size {
        None => return Ok(Err("missing-source")),
        Some((declared, actual)) if declared != actual || actual < 0 => {
            return Ok(Err("source-length-mismatch"))
        }
        Some((_, actual)) if actual > MAX_XML_BYTES as i64 => return Ok(Err("source-byte-limit")),
        _ => {}
    }
    let bytes = load_source_file(conn, sha)?.ok_or_else(|| ProductDbError::Xml {
        source_path: "procedure source".into(),
        cause: "source disappeared during read".into(),
    })?;
    if sha256_hex(&bytes) != sha {
        return Ok(Err("source-hash-mismatch"));
    }
    Ok(Ok(bytes))
}
/// Resolves only sources owned by this exact package; no global master fallback.
/// A database failure propagates instead of being reported as absent evidence.
pub fn resolve_package_ap1(
    conn: &Connection,
    package_sha: &str,
    program_id: &str,
) -> Result<Option<Resolution>, ProductDbError> {
    let row:Option<(Option<String>,Option<String>,String)>=conn.query_row("SELECT mask_version,load_procedure_style,source_sha256 FROM application_program WHERE id=?1",[program_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
    let Some((mask, style, sha)) = row else {
        return Ok(None);
    };
    if mask.as_deref() != Some("MV-07B0")
        || !matches!(
            style.as_deref(),
            Some("MergedProcedure" | "DefaultProcedure")
        )
    {
        return Ok(None);
    }
    let mut r = Resolution::new(program_id, "MV-07B0", style.as_deref().unwrap());
    let owned:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM package_member WHERE package_sha256=?1 AND source_sha256=?2 AND role='ApplicationProgram')",[package_sha,&sha],|row|row.get(0))?;
    if !owned {
        r.issue("program-source-not-in-package", "program");
        return Ok(Some(r));
    }
    let mut stmt=conn.prepare("SELECT DISTINCT source_sha256 FROM package_member WHERE package_sha256=?1 AND role='Master' ORDER BY source_sha256")?;
    let masters: Vec<String> = stmt
        .query_map([package_sha], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    if masters.len() != 1 {
        r.issue("missing-or-ambiguous-master", "master");
        return Ok(Some(r));
    }
    let identity =
        crate::identity_candidates(conn, crate::IdentityKind::ApplicationProgram, program_id)?;
    let candidates = identity
        .candidates
        .iter()
        .filter(|c| c.packages.iter().any(|p| p == package_sha))
        .count();
    if candidates > 1 {
        r.issue("ambiguous-program-source", "program");
        return Ok(Some(r));
    }
    let program = match loaded_source(conn, &sha)? {
        Ok(b) => b,
        Err(code) => {
            r.issue(code, "program");
            return Ok(Some(r));
        }
    };
    let master = match loaded_source(conn, &masters[0])? {
        Ok(b) => b,
        Err(code) => {
            r.issue(code, "master");
            return Ok(Some(r));
        }
    };
    match expand(&mut r, &program, &master, &sha, &masters[0]) {
        Ok(()) => {
            if candidates != 1 {
                r.issue("unmeasured-program-source", "program");
            }
            r.status = if r.issues.is_empty() {
                "expanded"
            } else {
                "partial"
            }
            .into();
        }
        Err(code) => {
            r.issue(code, "source");
            if !r.steps.is_empty() || !r.unplaced_declarations.is_empty() {
                r.status = "partial".into();
            }
        }
    }
    Ok(Some(r))
}
fn expand(
    r: &mut Resolution,
    program: &[u8],
    master: &[u8],
    psha: &str,
    msha: &str,
) -> Result<(), &'static str> {
    let p = parse(program)?;
    let m = parse(master)?;
    if p.namespace != m.namespace {
        return Err("source-namespace-conflict");
    }
    let ns = &p.namespace;
    let a = unique(
        path(
            &p,
            &[
                "ManufacturerData",
                "Manufacturer",
                "ApplicationPrograms",
                "ApplicationProgram",
            ],
        )
        .into_iter()
        .filter(|n| n.attr("Id") == Some(&r.program_id))
        .collect(),
        "missing-or-duplicate-program",
    )?;
    if a.attr("MaskVersion") != Some(&r.mask) || a.attr("LoadProcedureStyle") != Some(&r.style) {
        return Err("program-identity-conflict");
    }
    let mask = unique(
        path(&m, &["MasterData", "MaskVersions", "MaskVersion"])
            .into_iter()
            .filter(|n| n.attr("Id") == Some(&r.mask))
            .collect(),
        "missing-or-duplicate-mask",
    )?;
    let mut templates = Vec::new();
    for context in mask.children_named(ns, "HawkConfigurationData") {
        for container in context.children_named(ns, "Procedures") {
            for template in container.children_named(ns, "Procedure") {
                if template.attr("ProcedureType") == Some("Load")
                    && template.attr("ProcedureSubType") == Some("ap1")
                {
                    templates.push((context, container, template));
                }
            }
        }
    }
    if templates.len() != 1 {
        return Err("missing-or-duplicate-template");
    }
    let (context, container, template) = templates[0];
    // The observed unqualified wrapper is the boundary. LegacyVersion and
    // other selector semantics are not guessed, nor used for first-wins.
    if !context.attributes.is_empty()
        || !container.attributes.is_empty()
        || !context.text.trim().is_empty()
        || !container.text.trim().is_empty()
    {
        return Err("uninterpreted-template-container");
    }
    r.sources = vec![
        Source {
            source: "program".into(),
            sha256: psha.into(),
            namespace: ns.clone(),
        },
        Source {
            source: "master".into(),
            sha256: msha.into(),
            namespace: ns.clone(),
        },
    ];
    let identity: BTreeMap<String, String> = a
        .attributes
        .iter()
        .filter(|(k, _)| {
            matches!(
                k.as_str(),
                "Id" | "MaskVersion"
                    | "LoadProcedureStyle"
                    | "ApplicationNumber"
                    | "ApplicationVersion"
                    | "PeiType"
            )
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let mut output = 512
        + 6 * identity
            .iter()
            .chain(template.attributes.iter())
            .map(|(k, v)| k.len() + v.len())
            .sum::<usize>();
    if output > MAX_OUTPUT {
        return Err("resolution-output-limit");
    }
    r.program_attributes = identity;
    r.template_attributes = template.attributes.clone();
    if template
        .attributes
        .keys()
        .any(|k| !matches!(k.as_str(), "ProcedureType" | "ProcedureSubType" | "Access"))
        || !template.text.trim().is_empty()
    {
        r.issue("uninterpreted-template-metadata", "master/template");
    }
    let containers: Vec<_> = a
        .children_named(ns, "Static")
        .flat_map(|s| s.children_named(ns, "LoadProcedures"))
        .collect();
    if containers.len() > 1 {
        return Err("duplicate-fragment-container");
    }
    let mut fragments = BTreeMap::<&str, Vec<&Node>>::new();
    if let Some(container) = containers.first() {
        if !container.attributes.is_empty() || !container.text.trim().is_empty() {
            r.issue("uninterpreted-container-metadata", "program/fragments");
        }
        for fragment in &container.children {
            if !fragment.matches(ns, "LoadProcedure")
                || fragment.attributes.len() != 1
                || fragment.attr("MergeId").is_none()
                || !fragment.text.trim().is_empty()
            {
                r.issue("uninterpreted-fragment", "program/fragments");
                continue;
            }
            fragments
                .entry(fragment.attr("MergeId").unwrap())
                .or_default()
                .push(fragment);
        }
    }
    if r.style == "DefaultProcedure" && !fragments.is_empty() {
        r.issue("fragments-on-default-procedure", "program/fragments");
    }
    let mut seen = BTreeSet::new();
    let mut used = BTreeSet::new();
    let counts: BTreeMap<&str, usize> = template
        .children
        .iter()
        .filter(|n| n.matches(ns, "LdCtrlMerge"))
        .filter_map(|n| n.attr("MergeId"))
        .fold(BTreeMap::new(), |mut acc, k| {
            *acc.entry(k).or_default() += 1;
            acc
        });
    for (index, node) in template.children.iter().enumerate() {
        let loc = format!("master/template/step-{}", index + 1);
        if !node.matches(ns, "LdCtrlMerge") {
            emit(r, node, "master", msha, &loc, &mut output)?;
            continue;
        }
        let id = node.attr("MergeId").unwrap_or("");
        seen.insert(id);
        let origin = Origin {
            source: "master".into(),
            sha256: msha.into(),
            location: loc.clone(),
        };
        if node.attributes.len() != 1
            || !node.children.is_empty()
            || !node.text.trim().is_empty()
            || !matches!(id, "1" | "2" | "4" | "6" | "7")
            || counts.get(id) != Some(&1)
        {
            r.issue("invalid-or-duplicate-merge-point", &loc);
            emit(r, node, "master", msha, &loc, &mut output)?;
            continue;
        }
        match fragments.get(id).filter(|_| r.style == "MergedProcedure") {
            Some(items) if items.len() == 1 && !items[0].children.is_empty() => {
                used.insert(items[0].byte_start);
                r.merges.push(Merge {
                    merge_id: id.into(),
                    disposition: "expanded".into(),
                    origin,
                });
                for (i, child) in items[0].children.iter().enumerate() {
                    emit(
                        r,
                        child,
                        "program",
                        psha,
                        &format!("program/merge-{id}/step-{}", i + 1),
                        &mut output,
                    )?;
                }
            }
            None if matches!(id, "1" | "6" | "7") => r.merges.push(Merge {
                merge_id: id.into(),
                disposition: "optional-omitted".into(),
                origin,
            }),
            _ => {
                r.issue(
                    if fragments.get(id).is_some_and(|f| f.len() > 1) {
                        "duplicate-fragment"
                    } else if fragments
                        .get(id)
                        .is_some_and(|f| f.len() == 1 && f[0].children.is_empty())
                    {
                        "empty-fragment"
                    } else {
                        "missing-mandatory-merge"
                    },
                    &loc,
                );
                emit(r, node, "master", msha, &loc, &mut output)?;
            }
        }
    }
    for required in ["2", "4"] {
        if !seen.contains(required) {
            r.issue(
                "missing-mandatory-point",
                &format!("master/merge-{required}"),
            );
        }
    }
    if let Some(container) = containers.first() {
        for (index, fragment) in container.children.iter().enumerate() {
            if used.contains(&fragment.byte_start) {
                continue;
            }
            let loc = format!("program/unplaced-fragment-{}", index + 1);
            r.issue("unused-fragment", &loc);
            retain_unplaced(r, fragment, psha, &loc, &mut output)?;
        }
        if !container.attributes.is_empty() || !container.text.trim().is_empty() {
            retain_unplaced(
                r,
                container,
                psha,
                "program/fragment-container",
                &mut output,
            )?;
        }
    }
    Ok(())
}
fn retain_unplaced(
    r: &mut Resolution,
    node: &Node,
    sha: &str,
    location: &str,
    output: &mut usize,
) -> Result<(), &'static str> {
    let weight = node.weight();
    if r.steps.len() + r.unplaced_declarations.len() >= MAX_STEPS
        || output.checked_add(weight).is_none_or(|n| n > MAX_OUTPUT)
    {
        return Err("resolution-output-limit");
    }
    *output += weight;
    r.unplaced_declarations.push(Step {
        node: node.clone(),
        origin: Origin {
            source: "program".into(),
            sha256: sha.into(),
            location: location.into(),
        },
    });
    Ok(())
}
fn emit(
    r: &mut Resolution,
    node: &Node,
    source: &str,
    sha: &str,
    location: &str,
    output: &mut usize,
) -> Result<(), &'static str> {
    let weight = node.weight();
    if r.steps.len() + r.unplaced_declarations.len() >= MAX_STEPS
        || output.checked_add(weight).is_none_or(|n| n > MAX_OUTPUT)
    {
        return Err("resolution-output-limit");
    }
    *output += weight;
    // These are documented control names, not validation/execution of their attributes.
    let recognized = matches!(
        node.name.as_str(),
        "LdCtrlConnect"
            | "LdCtrlDisconnect"
            | "LdCtrlRestart"
            | "LdCtrlUnload"
            | "LdCtrlLoad"
            | "LdCtrlLoadCompleted"
            | "LdCtrlRelSegment"
            | "LdCtrlWriteRelMem"
            | "LdCtrlLoadImageRelMem"
            | "LdCtrlWriteProp"
            | "LdCtrlReadProp"
            | "LdCtrlCompareProp"
            | "LdCtrlMapError"
    );
    let allowed: &[&str] = match node.name.as_str() {
        "LdCtrlConnect" | "LdCtrlDisconnect" | "LdCtrlRestart" => &[],
        "LdCtrlUnload" | "LdCtrlLoad" | "LdCtrlLoadCompleted" => &["LsmIdx"],
        "LdCtrlRelSegment" => &["LsmIdx", "Size", "Mode", "Fill"],
        "LdCtrlWriteRelMem" | "LdCtrlLoadImageRelMem" => &["ObjIdx", "Offset", "Size", "Verify"],
        "LdCtrlWriteProp" | "LdCtrlReadProp" | "LdCtrlCompareProp" => &[
            "ObjIdx",
            "PropId",
            "Count",
            "StartElement",
            "Verify",
            "InlineData",
        ],
        "LdCtrlMapError" => &["OriginalError", "MappedError"],
        _ => &[],
    };
    if !recognized
        || node.namespace != r.sources[0].namespace
        || !node.children.is_empty()
        || !node.text.trim().is_empty()
        || node
            .attributes
            .keys()
            .any(|k| !allowed.contains(&k.as_str()))
    {
        r.issue(
            if node.name == "LdCtrlMerge" {
                "nested-or-unresolved-merge"
            } else {
                "uninterpreted-step"
            },
            location,
        );
    }
    r.steps.push(Step {
        node: node.clone(),
        origin: Origin {
            source: source.into(),
            sha256: sha.into(),
            location: location.into(),
        },
    });
    Ok(())
}
