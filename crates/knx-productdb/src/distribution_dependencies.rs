//! Inventories declared Script/calculation/validation/handler/Assign facts without running them.

use super::{readers::ReferenceIndex, xml::Document, *};
use std::collections::{hash_map::Entry, BTreeMap, HashMap, HashSet};

/// One declared dependency fact. Never executed, resolved to a function or applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionDependencyEvidence {
    pub kind: DistributionDependencyKind,
    pub source: DistributionSourceRef,
    pub application_source: DistributionSourceRef,
    pub manufacturer_source: DistributionSourceRef,
    /// Nearest enclosing ModuleDef definition; `None` means the application scope itself.
    pub module_definition_source: Option<DistributionSourceRef>,
    /// Fixed per kind, in a fixed order; every listed name is reported, also when absent.
    pub attributes: Vec<DistributionDeclaredAttribute>,
    /// Calculation sides and validation wrappers, one row per occurrence or one absent row.
    pub member_lists: Vec<DistributionMemberList>,
    pub endpoints: Vec<DistributionDependencyEndpoint>,
    /// Raw original bytes between the Script start and end tag, never decoded or echoed.
    pub content: Option<DistributionRawContent>,
    pub assignment_form: Option<DistributionAssignmentForm>,
    pub function_resolution: Option<DistributionFunctionResolution>,
    pub graph_cycle: DistributionGraphCycle,
    pub disposition: DistributionDependencyDisposition,
    pub impact_scope: DistributionImpactScope,
    pub condition_context: DistributionConditionContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionDependencyKind {
    Script,
    ParameterCalculation,
    ParameterValidation,
    ButtonHandler,
    Assign,
}

/// `presentValue` keeps whitespace; only the zero-length value is `presentEmpty`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionPresence {
    Absent,
    PresentEmpty,
    PresentValue,
}

/// An unqualified attribute with its XML-decoded value, verbatim otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionDeclaredAttribute {
    pub name: String,
    pub presence: DistributionPresence,
    pub value: Option<String>,
}

/// Side or membership labels only: no input/output direction, trigger or order is implied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionEndpointRole {
    CalculationLeft,
    CalculationRight,
    ValidationMember,
    AssignTarget,
    AssignSource,
}

/// A wrapper occurrence: `presentEmpty` declares no member, `absent` has no wrapper at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionMemberList {
    pub role: DistributionEndpointRole,
    pub presence: DistributionPresence,
    pub source: Option<DistributionSourceRef>,
}

/// A declared reference resolved like `references[]`; candidates keep every duplicate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionDependencyEndpoint {
    pub role: DistributionEndpointRole,
    pub source: DistributionSourceRef,
    pub attribute: String,
    pub presence: DistributionPresence,
    pub raw_ref: Option<String>,
    /// `None` only when the attribute is absent; an empty reference is `dangling`.
    pub reference_state: Option<DistributionReferenceState>,
    pub candidates: Vec<DistributionSourceRef>,
}

/// Half-open original member byte range, its size and SHA-256 (CDATA delimiters included).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionRawContent {
    pub start_byte: u64,
    pub end_byte: u64,
    pub size: u64,
    pub sha256: String,
}

/// Declared Assign shape; `conflictingDeclarations` chooses no precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionAssignmentForm {
    SourceReference,
    LiteralValue,
    ConflictingDeclarations,
    MissingSourceAndValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionFunctionResolution {
    NotAttempted,
}

/// A cycle of declared Assign source-to-target edges, not a runtime loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionGraphCycle {
    /// Not an Assign, or an Assign without a uniquely resolved source and target.
    NotApplicable,
    NoDeclaredCycle,
    DeclaredGraphCycle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionDependencyDisposition {
    RetainedNotExecuted,
}

/// Uncertainty localization, never a dependency-closure or non-impact guarantee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionImpactScope {
    DeclaredEndpointsOnly,
    ApplicationImpactUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionConditionApplicability {
    NotEvaluated,
}

/// Enclosing Dynamic `choose`/`when` occurrences, outermost first; never evaluated here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionConditionContext {
    pub applicability: DistributionConditionApplicability,
    pub conditions: Vec<DistributionSourceRef>,
}

use DistributionDependencyKind as Kind;
use DistributionEndpointRole as Role;

fn named(doc: &Document<'_>, id: usize, local: &str) -> bool {
    let n = &doc.nodes[id];
    n.ns.as_ref() == doc.namespace && n.local == local
}

/// The application whose reader-projected `Static` (own or ModuleDef) is the parent of `id`.
fn static_child(doc: &Document<'_>, id: usize) -> Option<usize> {
    let parent = doc.nodes[id].parent?;
    let app = doc.application(id)?;
    (named(doc, parent, "Static") && doc.projected(parent, app)).then_some(app)
}

/// `Static/<wrapper>/<declaration>` with exact names in the document namespace.
fn static_declaration(doc: &Document<'_>, id: usize, wrapper: &str, local: &str) -> Option<usize> {
    let parent = doc.nodes[id].parent?;
    (named(doc, id, local) && named(doc, parent, wrapper))
        .then(|| static_child(doc, parent))
        .flatten()
}

fn declared_kind(doc: &Document<'_>, id: usize) -> Option<(Kind, usize)> {
    let n = &doc.nodes[id];
    if n.ns.as_ref() != doc.namespace {
        return None;
    }
    match n.local {
        "Script" => static_child(doc, id).map(|app| (Kind::Script, app)),
        "ParameterCalculation" => static_declaration(doc, id, "ParameterCalculations", n.local)
            .map(|app| (Kind::ParameterCalculation, app)),
        "ParameterValidation" => static_declaration(doc, id, "ParameterValidations", n.local)
            .map(|app| (Kind::ParameterValidation, app)),
        // Projection admits only Dynamic descendants for these names; foreign wrappers stop it.
        "Button" | "Assign" => {
            let app = doc.application(id)?;
            let kind = if n.local == "Button" {
                Kind::ButtonHandler
            } else {
                Kind::Assign
            };
            doc.projected(id, app).then_some((kind, app))
        }
        _ => None,
    }
}

/// Exact membership role of a `ParameterRefRef` in a calculation side or validation wrapper.
pub(super) fn membership(doc: &Document<'_>, id: usize) -> Option<Role> {
    if !named(doc, id, "ParameterRefRef") {
        return None;
    }
    let list = doc.nodes[id].parent?;
    let owner = doc.nodes[list].parent?;
    let role = list_role(doc, owner, list)?;
    declared_kind(doc, owner).map(|_| role)
}

fn list_role(doc: &Document<'_>, owner: usize, list: usize) -> Option<Role> {
    if named(doc, owner, "ParameterCalculation") && named(doc, list, "LParameters") {
        Some(Role::CalculationLeft)
    } else if named(doc, owner, "ParameterCalculation") && named(doc, list, "RParameters") {
        Some(Role::CalculationRight)
    } else if named(doc, owner, "ParameterValidation") && named(doc, list, "Parameters") {
        Some(Role::ValidationMember)
    } else {
        None
    }
}

fn presence(value: &Option<String>) -> DistributionPresence {
    match value.as_deref() {
        None => DistributionPresence::Absent,
        Some("") => DistributionPresence::PresentEmpty,
        Some(_) => DistributionPresence::PresentValue,
    }
}

fn attribute_names(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::Script => &[],
        Kind::ParameterCalculation => &["Id", "LRTransformationFunc", "RLTransformationFunc"],
        Kind::ParameterValidation => &["Id", "ValidationFunc"],
        Kind::ButtonHandler => &["Id", "Text", "EventHandler", "EventHandlerOnline"],
        Kind::Assign => &["TargetParamRefRef", "SourceParamRefRef", "Value"],
    }
}

/// Declarations a reference names, looked up exactly like `references[]` in the original scope.
fn candidates<'i>(
    doc: &Document<'_>,
    index: &'i ReferenceIndex,
    owner: usize,
    value: &str,
) -> &'i [usize] {
    index
        .get(&(
            doc.manufacturer(owner),
            doc.entity_scope(owner),
            "ParameterRef".to_string(),
            value.to_string(),
        ))
        .map_or(&[][..], Vec::as_slice)
}

struct Endpoints<'a, 'd> {
    doc: &'a Document<'d>,
    index: &'a ReferenceIndex,
    references: &'a mut usize,
    max_references: usize,
}
impl Endpoints<'_, '_> {
    /// Every endpoint and candidate is charged to the shared reference budget before copying.
    fn endpoint(
        &mut self,
        role: Role,
        owner: usize,
        attribute: &str,
    ) -> Result<DistributionDependencyEndpoint> {
        let raw_ref = self.doc.nodes[owner].attr(attribute);
        let found = raw_ref
            .as_deref()
            .map_or(&[][..], |v| candidates(self.doc, self.index, owner, v));
        *self.references = self
            .references
            .checked_add(1 + found.len())
            .filter(|&n| n <= self.max_references)
            .ok_or_else(|| limit("references"))?;
        let reference_state = raw_ref.as_ref().map(|_| match found.len() {
            0 => DistributionReferenceState::Dangling,
            1 => DistributionReferenceState::Resolved,
            _ => DistributionReferenceState::Ambiguous,
        });
        Ok(DistributionDependencyEndpoint {
            role,
            source: self.doc.coordinate(owner)?,
            attribute: attribute.into(),
            presence: presence(&raw_ref),
            raw_ref,
            reference_state,
            candidates: found
                .iter()
                .map(|&i| self.doc.coordinate(i))
                .collect::<Result<_>>()?,
        })
    }
}

/// Uniquely resolved Assign source-to-target edges, keyed by the Assign element.
fn assign_edges(doc: &Document<'_>, index: &ReferenceIndex) -> BTreeMap<usize, (usize, usize)> {
    let mut edges = BTreeMap::new();
    for id in 0..doc.nodes.len() {
        if !matches!(declared_kind(doc, id), Some((Kind::Assign, _))) {
            continue;
        }
        let unique = |attribute: &str| match doc.nodes[id].attr(attribute) {
            Some(v) => match candidates(doc, index, id, &v) {
                [one] => Some(*one),
                _ => None,
            },
            None => None,
        };
        if let (Some(source), Some(target)) =
            (unique("SourceParamRefRef"), unique("TargetParamRefRef"))
        {
            edges.insert(id, (source, target));
        }
    }
    edges
}

/// Strongly connected components (iterative Tarjan); no recursion over source-controlled depth.
fn components(edges: &BTreeMap<usize, (usize, usize)>) -> HashMap<usize, usize> {
    let mut graph = BTreeMap::<usize, Vec<usize>>::new();
    for &(source, target) in edges.values() {
        graph.entry(source).or_default().push(target);
        graph.entry(target).or_default();
    }
    let (mut order, mut low, mut component) = (HashMap::new(), HashMap::new(), HashMap::new());
    let (mut stack, mut on_stack, mut next, mut count) = (Vec::new(), HashSet::new(), 0usize, 0);
    for &root in graph.keys() {
        if order.contains_key(&root) {
            continue;
        }
        let mut work = vec![(root, 0usize)];
        order.insert(root, next);
        low.insert(root, next);
        next += 1;
        stack.push(root);
        on_stack.insert(root);
        while let Some(&(node, position)) = work.last() {
            let successors = &graph[&node];
            if let Some(&successor) = successors.get(position) {
                work.last_mut().expect("frame").1 += 1;
                if let Entry::Vacant(slot) = order.entry(successor) {
                    slot.insert(next);
                    low.insert(successor, next);
                    next += 1;
                    stack.push(successor);
                    on_stack.insert(successor);
                    work.push((successor, 0));
                } else if on_stack.contains(&successor) {
                    let reached = order[&successor];
                    let current = low.get_mut(&node).expect("low");
                    *current = (*current).min(reached);
                }
                continue;
            }
            work.pop();
            let node_low = low[&node];
            if let Some(&(parent, _)) = work.last() {
                let current = low.get_mut(&parent).expect("low");
                *current = (*current).min(node_low);
            }
            if node_low == order[&node] {
                while let Some(member) = stack.pop() {
                    on_stack.remove(&member);
                    component.insert(member, count);
                    if member == node {
                        break;
                    }
                }
                count += 1;
            }
        }
    }
    component
}

/// Reports declarations in element order; nothing is executed, resolved to a function or applied.
pub(super) fn inspect(
    doc: &Document<'_>,
    index: &ReferenceIndex,
    references: &mut usize,
    limits: &DistributionLimits,
    report: &mut DistributionInspection,
) -> Result<()> {
    let edges = assign_edges(doc, index);
    let component = components(&edges);
    let mut endpoints = Endpoints {
        doc,
        index,
        references,
        max_references: limits.max_references,
    };
    for id in 0..doc.nodes.len() {
        let Some((kind, app)) = declared_kind(doc, id) else {
            continue;
        };
        let Some(manufacturer) = doc.manufacturer(id) else {
            continue;
        };
        let node = &doc.nodes[id];
        let attributes = attribute_names(kind)
            .iter()
            .map(|&name| {
                let value = node.attr(name);
                DistributionDeclaredAttribute {
                    name: name.into(),
                    presence: presence(&value),
                    value,
                }
            })
            .collect::<Vec<_>>();
        let mut member_lists = Vec::new();
        let mut row_endpoints = Vec::new();
        let expected: &[Role] = match kind {
            Kind::ParameterCalculation => &[Role::CalculationLeft, Role::CalculationRight],
            Kind::ParameterValidation => &[Role::ValidationMember],
            _ => &[],
        };
        for list in doc.children(id) {
            let Some(role) = list_role(doc, id, list) else {
                continue;
            };
            let members = doc
                .children(list)
                .filter(|&m| named(doc, m, "ParameterRefRef"))
                .collect::<Vec<_>>();
            member_lists.push(DistributionMemberList {
                role,
                presence: if members.is_empty() {
                    DistributionPresence::PresentEmpty
                } else {
                    DistributionPresence::PresentValue
                },
                source: Some(doc.coordinate(list)?),
            });
            for member in members {
                row_endpoints.push(endpoints.endpoint(role, member, "RefId")?);
            }
        }
        for &role in expected {
            if !member_lists.iter().any(|l| l.role == role) {
                member_lists.push(DistributionMemberList {
                    role,
                    presence: DistributionPresence::Absent,
                    source: None,
                });
            }
        }
        let mut assignment_form = None;
        let mut graph_cycle = DistributionGraphCycle::NotApplicable;
        if kind == Kind::Assign {
            for (role, attribute) in [
                (Role::AssignTarget, "TargetParamRefRef"),
                (Role::AssignSource, "SourceParamRefRef"),
            ] {
                if node.attr(attribute).is_some() {
                    row_endpoints.push(endpoints.endpoint(role, id, attribute)?);
                }
            }
            assignment_form = Some(
                match (
                    node.attr("SourceParamRefRef").is_some(),
                    node.attr("Value").is_some(),
                ) {
                    (true, true) => DistributionAssignmentForm::ConflictingDeclarations,
                    (true, false) => DistributionAssignmentForm::SourceReference,
                    (false, true) => DistributionAssignmentForm::LiteralValue,
                    (false, false) => DistributionAssignmentForm::MissingSourceAndValue,
                },
            );
            if let Some(&(source, target)) = edges.get(&id) {
                graph_cycle =
                    if source == target || component.get(&source) == component.get(&target) {
                        DistributionGraphCycle::DeclaredGraphCycle
                    } else {
                        DistributionGraphCycle::NoDeclaredCycle
                    };
            }
        }
        let content = (kind == Kind::Script).then(|| {
            let (start, end) = node.content;
            let bytes = &doc.bytes[start..end];
            DistributionRawContent {
                start_byte: start as u64,
                end_byte: end as u64,
                size: bytes.len() as u64,
                sha256: crate::sha256_hex(bytes),
            }
        });
        let mut conditions = Vec::new();
        if matches!(kind, Kind::Assign | Kind::ButtonHandler) {
            let mut next = node.parent;
            while let Some(i) = next.filter(|&i| !named(doc, i, "Dynamic")) {
                if named(doc, i, "choose") || named(doc, i, "when") {
                    conditions.push(doc.coordinate(i)?);
                }
                next = doc.nodes[i].parent;
            }
            conditions.reverse();
        }
        let scope = doc.entity_scope(id).filter(|&s| s != app);
        let row = DistributionDependencyEvidence {
            kind,
            source: doc.coordinate(id)?,
            application_source: doc.coordinate(app)?,
            manufacturer_source: doc.coordinate(manufacturer)?,
            module_definition_source: scope.map(|s| doc.coordinate(s)).transpose()?,
            attributes,
            member_lists,
            endpoints: row_endpoints,
            content,
            assignment_form,
            function_resolution: matches!(
                kind,
                Kind::ParameterCalculation | Kind::ParameterValidation | Kind::ButtonHandler
            )
            .then_some(DistributionFunctionResolution::NotAttempted),
            graph_cycle,
            disposition: DistributionDependencyDisposition::RetainedNotExecuted,
            impact_scope: if kind == Kind::Assign {
                DistributionImpactScope::DeclaredEndpointsOnly
            } else {
                DistributionImpactScope::ApplicationImpactUnknown
            },
            condition_context: DistributionConditionContext {
                applicability: DistributionConditionApplicability::NotEvaluated,
                conditions,
            },
        };
        doc.push(&mut report.dependency_evidence, row)?;
        if report.dependency_evidence.len() > limits.max_output_records {
            return Err(limit("output records"));
        }
    }
    Ok(())
}
