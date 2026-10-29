//! Shares cumulative diagnostic-copy work and residency ceilings across all nested inspections.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub(super) const XML_DEPTH: usize = 256;
pub(super) const XML_PATH: usize = 8192;
const WORK: usize = 128 * 1024 * 1024;
const OUTPUT: usize = 1024 * 1024 * 1024;
pub(super) const MATCH_WORK: usize = 16 * 1024 * 1024;
pub(super) const NATIVE_BYTES: u64 = 512 * 1024 * 1024;
pub(super) const SOURCE_BYTES: u64 = 256 * 1024 * 1024;
const DECODED: usize = 1024 * 1024 * 1024;

#[derive(Clone, Default)]
pub(super) struct Budget(Rc<State>);

#[derive(Default)]
struct State {
    work: Cell<usize>,
    output: Cell<usize>,
    decoded: Cell<usize>,
    matching: Cell<usize>,
    values: Cell<usize>,
    value_bytes: Cell<usize>,
    findings: Cell<usize>,
    metadata: Cell<usize>,
    error: RefCell<Option<&'static str>>,
}

impl Budget {
    fn charge(
        &self,
        counter: &Cell<usize>,
        amount: usize,
        limit: usize,
        reason: &'static str,
    ) -> Result<(), &'static str> {
        self.check()?;
        let next = counter.get().checked_add(amount).filter(|n| *n <= limit);
        match next {
            Some(next) => {
                counter.set(next);
                Ok(())
            }
            None => {
                *self.0.error.borrow_mut() = Some(reason);
                Err(reason)
            }
        }
    }
    pub(super) fn work(&self, amount: usize) -> Result<(), &'static str> {
        self.charge(
            &self.0.work,
            amount,
            WORK,
            "inspection work budget exceeded",
        )
    }
    pub(super) fn output(&self, amount: usize) -> Result<(), &'static str> {
        self.charge(
            &self.0.output,
            amount,
            OUTPUT,
            "inspection output budget exceeded",
        )
    }
    pub(super) fn decoded(&self, amount: usize) -> Result<(), &'static str> {
        self.charge(
            &self.0.decoded,
            amount,
            DECODED,
            "decoded expansion budget exceeded",
        )
    }
    pub(super) fn matching(&self, amount: usize) -> Result<(), &'static str> {
        self.charge(
            &self.0.matching,
            amount,
            MATCH_WORK,
            "residual matching budget exceeded",
        )
    }
    pub(super) fn value(&self, bytes: usize) -> Result<(), &'static str> {
        self.charge(
            &self.0.values,
            1,
            16384,
            "learned value count budget exceeded",
        )?;
        self.charge(
            &self.0.value_bytes,
            bytes,
            8 * 1024 * 1024,
            "learned value bytes budget exceeded",
        )
    }
    pub(super) fn finding(&self, bytes: usize) -> Result<(), &'static str> {
        self.charge(&self.0.findings, 1, 32768, "finding count budget exceeded")?;
        self.charge(
            &self.0.metadata,
            bytes,
            8 * 1024 * 1024,
            "finding metadata budget exceeded",
        )
    }
    #[cfg(test)]
    pub(super) fn work_used(&self) -> usize {
        self.0.work.get()
    }
    #[cfg(test)]
    pub(super) fn output_used(&self) -> usize {
        self.0.output.get()
    }
    #[cfg(test)]
    pub(super) fn matching_used(&self) -> usize {
        self.0.matching.get()
    }
    pub(super) fn check(&self) -> Result<(), &'static str> {
        self.0.error.borrow().map_or(Ok(()), Err)
    }
}
