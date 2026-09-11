//! The `ApplicationProgram/Dynamic` tree: the `Channel`/`ParameterBlock`/
//! `choose`/`when` visibility program that decides which parameters and
//! communication objects are active for a given device configuration.
//!
//! This module currently does one thing: store the tree losslessly,
//! unevaluated (`parse`). Evaluating it — turning a `Dynamic` tree plus a
//! set of parameter values into the active parameter-ref and
//! communication-object-ref sets — is a later slice; see
//! `docs/superpowers/specs/2026-09-11-dynamic-tree-parse-and-evaluate-design.md`
//! (decisions D1-D5 cover this slice, D6-D11 the evaluator to come).

pub mod parse;
