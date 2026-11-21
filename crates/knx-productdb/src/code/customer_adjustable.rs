//! Retains bounded customer-adjustable declarations without interpreting their values.

use std::fmt;

use super::{
    memory_placement, module_memory_placement, property_placement, unmodelled_placement, CodeError,
    Element, ParameterPlacement, Parser, ProgramCode,
};

/// Closed declaration contexts; neither source identifiers nor values are public diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomerAdjustableContext {
    /// An own root Static/Parameters/Parameter.
    RootParameter,
    /// A direct Union member in root Static/Parameters.
    RootUnionParameter,
    /// A parameter in an explicitly entered module definition's own Static.
    ModuleParameter,
    /// A direct Union member in that module's own Static/Parameters.
    ModuleUnionParameter,
    /// An own root Static/Options declaration.
    RootOptions,
    /// An Options declaration directly inside an explicitly entered module definition's own Static.
    ModuleOptions,
}

/// Payload-free summary used by write refusal and its public consumers.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CustomerAdjustableCounts {
    /// Root parameter declarations.
    pub root_parameters: usize,
    /// Root direct Union member declarations.
    pub root_union_parameters: usize,
    /// Module parameter declarations.
    pub module_parameters: usize,
    /// Module direct Union member declarations.
    pub module_union_parameters: usize,
    /// Root Options declarations.
    pub root_options: usize,
    /// Module-local Options declarations.
    pub module_options: usize,
}

impl CustomerAdjustableCounts {
    /// Number of declarations, including repetitions; every present value is uninterpreted.
    pub fn total(self) -> usize {
        self.root_parameters
            + self.root_union_parameters
            + self.module_parameters
            + self.module_union_parameters
            + self.root_options
            + self.module_options
    }
}

/// Ordered ephemeral evidence. Debug exposes only counts, never tokens or occurrence paths.
#[derive(Default, Clone)]
pub struct CustomerAdjustableDeclarations {
    declarations: Vec<Declaration>,
    // Flat parent indices share module prefixes and cannot recursively drop on hostile depth.
    paths: Vec<PathNode>,
}

impl CustomerAdjustableDeclarations {
    /// The only public projection of declaration evidence.
    pub fn counts(&self) -> CustomerAdjustableCounts {
        let mut counts = CustomerAdjustableCounts::default();
        for declaration in &self.declarations {
            match declaration.context {
                CustomerAdjustableContext::RootParameter => counts.root_parameters += 1,
                CustomerAdjustableContext::RootUnionParameter => counts.root_union_parameters += 1,
                CustomerAdjustableContext::ModuleParameter => counts.module_parameters += 1,
                CustomerAdjustableContext::ModuleUnionParameter => {
                    counts.module_union_parameters += 1
                }
                CustomerAdjustableContext::RootOptions => counts.root_options += 1,
                CustomerAdjustableContext::ModuleOptions => counts.module_options += 1,
            }
        }
        counts
    }

    pub(super) fn observe(
        &mut self,
        element: &Element,
        context: CustomerAdjustableContext,
        path: &[Occurrence],
    ) {
        if let Some(token) = element.attributes.evidence_value(Self::attribute(context)) {
            // Only the fixed-depth root helpers use slices; module observations use shared indices.
            let path = self.append_prefix(path);
            self.declarations.push(Declaration {
                context,
                token: token.to_owned(),
                path,
            });
        }
    }

    fn attribute(context: CustomerAdjustableContext) -> &'static str {
        if matches!(
            context,
            CustomerAdjustableContext::RootOptions | CustomerAdjustableContext::ModuleOptions
        ) {
            "CustomerAdjustableParameters"
        } else {
            "CustomerAdjustable"
        }
    }

    fn append_node(&mut self, parent: Option<usize>, occurrence: Occurrence) -> usize {
        let index = self.paths.len();
        self.paths.push(PathNode { parent, occurrence });
        index
    }

    fn append_prefix(&mut self, path: &[Occurrence]) -> Option<usize> {
        let mut parent = None;
        for &occurrence in path {
            parent = Some(self.append_node(parent, occurrence));
        }
        parent
    }

    fn observe_child(
        &mut self,
        element: &Element,
        context: CustomerAdjustableContext,
        parent: Option<usize>,
        occurrence: Occurrence,
    ) {
        if let Some(token) = element.attributes.evidence_value(Self::attribute(context)) {
            let path = Some(self.append_node(parent, occurrence));
            self.declarations.push(Declaration {
                context,
                token: token.to_owned(),
                path,
            });
        }
    }

    fn same_path(&self, mut left: Option<usize>, other: &Self, mut right: Option<usize>) -> bool {
        while let (Some(a), Some(b)) = (left, right) {
            let a = &self.paths[a];
            let b = &other.paths[b];
            if a.occurrence != b.occurrence {
                return false;
            }
            left = a.parent;
            right = b.parent;
        }
        left.is_none() && right.is_none()
    }
}

// Equality remains declaration/path equality, independent of unused arena prefixes or indices.
impl PartialEq for CustomerAdjustableDeclarations {
    fn eq(&self, other: &Self) -> bool {
        self.declarations.len() == other.declarations.len()
            && self
                .declarations
                .iter()
                .zip(&other.declarations)
                .all(|(a, b)| {
                    a.context == b.context
                        && a.token == b.token
                        && self.same_path(a.path, other, b.path)
                })
    }
}

impl Eq for CustomerAdjustableDeclarations {}

impl fmt::Debug for CustomerAdjustableDeclarations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CustomerAdjustableDeclarations")
            .field(&self.counts())
            .finish()
    }
}

#[derive(Clone)]
struct Declaration {
    context: CustomerAdjustableContext,
    token: String,
    path: Option<usize>,
}

#[derive(Clone)]
struct PathNode {
    parent: Option<usize>,
    occurrence: Occurrence,
}

// Ordinals are 1-based positions among ALL element children of the parent, not source IDs.
// Closed edge kinds plus each sibling ordinal distinguish duplicate IDs and both module anchors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Occurrence {
    Static(usize),
    Parameters(usize),
    Parameter(usize),
    Union(usize),
    Options(usize),
    ModuleDefs(usize),
    ModuleDef(usize),
}

#[derive(Clone, Copy)]
enum ModuleScope {
    Defs,
    Def,
    Static,
    Parameters,
    Union,
    /// Marker returned by the `Parameter` arm after it consumed the
    /// parameter subtree itself: the trailing `skip` must not run again.
    Consumed,
}

/// The placement a module-local `Parameter` element's children give it:
/// a `Memory` carrying `BaseOffset` first, else the plain `Memory`/`Property`
/// reading, else `Unmodelled` — the same names `field_of` refuses by name.
fn module_parameter_placement_of(
    parser: &mut Parser<'_>,
    element: &Element,
) -> Result<Option<ParameterPlacement>, CodeError> {
    let mut placement = None;
    if element.empty {
        return Ok(placement);
    }
    while let Some(child) = parser.next_inside(&element.name)? {
        if placement.is_none() && matches!(child.name.as_str(), "Memory" | "Property") {
            placement = Some(
                if let Some(module_memory) = module_memory_placement(&child) {
                    module_memory
                } else {
                    match (memory_placement(&child), property_placement(&child)) {
                        (Some(memory), _) => ParameterPlacement::Memory(memory),
                        (None, Some(property)) => ParameterPlacement::Property(property),
                        (None, None) => unmodelled_placement(&child.name, &child),
                    }
                },
            );
        }
        parser.skip(&child)?;
    }
    Ok(placement)
}

impl Parser<'_> {
    /// Explicit iterative traversal: untrusted module nesting never consumes the Rust call stack.
    pub(super) fn read_customer_modules(
        &mut self,
        element: &Element,
        code: &mut ProgramCode,
        prefix: &[Occurrence],
    ) -> Result<(), CodeError> {
        if element.empty {
            return Ok(());
        }
        let parent = code.customer_adjustable.append_prefix(prefix);
        let mut frames = vec![(ModuleScope::Defs, 0usize, parent)];
        while let Some((scope, ordinal, parent)) = frames.last_mut() {
            let Some(child) = self.next_inside("ModuleDefs")? else {
                frames.pop();
                continue;
            };
            *ordinal += 1;
            let index = *ordinal;
            let edge = match (*scope, child.name.as_str()) {
                (ModuleScope::Defs, "ModuleDef") => {
                    Some((ModuleScope::Def, Occurrence::ModuleDef(index)))
                }
                (ModuleScope::Def, "Static") => {
                    Some((ModuleScope::Static, Occurrence::Static(index)))
                }
                (ModuleScope::Def | ModuleScope::Static, "ModuleDefs") => {
                    Some((ModuleScope::Defs, Occurrence::ModuleDefs(index)))
                }
                (ModuleScope::Static, "Parameters") => {
                    Some((ModuleScope::Parameters, Occurrence::Parameters(index)))
                }
                (ModuleScope::Static, "Options") => {
                    // ADR-0181: a module-local Options declaration is an admitted
                    // context, not a skipped sibling.
                    code.customer_adjustable.observe_child(
                        &child,
                        CustomerAdjustableContext::ModuleOptions,
                        *parent,
                        Occurrence::Options(index),
                    );
                    None
                }
                (ModuleScope::Parameters, "Union") => {
                    Some((ModuleScope::Union, Occurrence::Union(index)))
                }
                (ModuleScope::Parameters | ModuleScope::Union, "Parameter") => {
                    let context = if matches!(scope, ModuleScope::Union) {
                        CustomerAdjustableContext::ModuleUnionParameter
                    } else {
                        CustomerAdjustableContext::ModuleParameter
                    };
                    code.customer_adjustable.observe_child(
                        &child,
                        context,
                        *parent,
                        Occurrence::Parameter(index),
                    );
                    // The same walk also reads the parameter's placement into
                    // `code.parameters`, keyed by its module-local `@Id`: a
                    // module-local `Memory/@BaseOffset` is kept as
                    // [`ParameterPlacement::ModuleMemory`] so the image builder
                    // can resolve it per instantiation (ADR-0177). A non-empty
                    // parameter subtree is consumed here (through its closing
                    // tag), so the trailing `skip` must not run again.
                    let id = child.attributes.get("Id").map(str::to_string);
                    if !child.empty {
                        if let Some(placement) = module_parameter_placement_of(self, &child)? {
                            if let Some(id) = id {
                                code.parameters.insert(id, placement);
                            }
                        }
                        Some((ModuleScope::Consumed, Occurrence::Parameter(index)))
                    } else {
                        None
                    }
                }
                (ModuleScope::Union, "Memory" | "Property") => {
                    // The union's own placement; its members are read above.
                    // A `Memory/@BaseOffset` on the union element itself is
                    // not promoted to members — each member's placement comes
                    // from its own children only, so this one is skipped.
                    self.skip(&child)?;
                    None
                }
                _ => None,
            };
            if let Some((next, occurrence)) = edge.filter(|_| !child.empty) {
                if matches!(next, ModuleScope::Consumed) {
                    // Subtree already consumed inside the arm — do not skip again.
                } else {
                    let parent = Some(code.customer_adjustable.append_node(*parent, occurrence));
                    frames.push((next, 0, parent));
                }
            } else {
                self.skip(&child)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "customer_adjustable_tests.rs"]
mod tests;
