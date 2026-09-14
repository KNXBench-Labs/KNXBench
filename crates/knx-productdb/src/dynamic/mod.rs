//! The `ApplicationProgram/Dynamic` tree: the `Channel`/`ParameterBlock`/
//! `choose`/`when` visibility program that decides which parameters and
//! communication objects are active for a given device configuration.
//!
//! This module does two things: store the tree losslessly, unevaluated
//! (`parse`, design D1-D5), and evaluate a stored tree headlessly into the
//! active parameter-ref and communication-object-ref sets plus a
//! diagnostics list (`evaluate`, design D6-D11). See
//! `docs/superpowers/specs/2026-09-11-dynamic-tree-parse-and-evaluate-design.md`.
//!
//! There is no UI, no HTTP surface, and no writing of parameter values
//! anywhere here or planned for this slice — `evaluate` is a pure function
//! of a loaded tree and a value map.

pub mod evaluate;
pub mod parse;

// Flattened re-export: the plan names these `dynamic::{load_tree,
// evaluate, Activation, Diagnostic, Test, Op}`, one level up from where
// they are actually implemented (`dynamic::evaluate`). `parse`'s own
// entry point (`parse_dynamic_trees`) is deliberately not flattened here —
// ingest and the migration backfill both already spell it out in full,
// and re-exporting it too would just give the same function two names.
pub use evaluate::{
    evaluate, load_program_trees, load_tree, resolve_values, Activation, ActiveRef, ControlKind,
    Diagnostic, DynamicNode, DynamicTree, ModuleScope, Op, ProgramTrees, ScopedDiagnostic, Test,
    UnparsableTest, ValueMap, MAX_MODULE_NESTING_DEPTH,
};
