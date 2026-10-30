//! Retains bounded customer-adjustable declarations without interpreting their values.

use std::fmt;

use super::{CodeError, Element, Parser, ProgramCode};

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
    /// An own root Static/Options declaration (module Options are excluded).
    RootOptions,
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
}

impl CustomerAdjustableCounts {
    /// Number of declarations, including repetitions; every present value is uninterpreted.
    pub fn total(self) -> usize {
        self.root_parameters
            + self.root_union_parameters
            + self.module_parameters
            + self.module_union_parameters
            + self.root_options
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
        if context == CustomerAdjustableContext::RootOptions {
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
                    None
                }
                _ => None,
            };
            if let Some((next, occurrence)) = edge.filter(|_| !child.empty) {
                let parent = Some(code.customer_adjustable.append_node(*parent, occurrence));
                frames.push((next, 0, parent));
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
