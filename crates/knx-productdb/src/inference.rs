//! Inferences: rules a download applies where the KNX documents are silent.
//!
//! ADR-0086: the specification, product data and project files are the
//! evidence of record. Where they say nothing but a working solution
//! exists — one consistent with every source and with what a device ETS
//! programmed holds — the image builder or the plan applies it and records
//! an [`Inference`]. Every inference reaches the user: in a program's
//! readiness and in the plan a download acknowledgement shows. A program
//! whose plan carries one is never `Verified` by evidence of a run without
//! it.

use std::fmt;

/// One inference a download applies, with where its analysis lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inference {
    /// Stable short name of the rule, the same for every application of it.
    pub rule: &'static str,
    /// What it did here, naming the elements concerned.
    pub detail: String,
    /// Where the analysis is documented (a RESEARCH section).
    pub reference: &'static str,
}

impl fmt::Display for Inference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} ({})", self.rule, self.detail, self.reference)
    }
}
