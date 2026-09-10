//! `knx-diff` computes what changed between two `knx_core::Project`s — a
//! "KNXBench project diff", never described as an ETS comparison or a
//! replacement for one (no ETS-produced comparison sample exists in this
//! repository — design spec §1). It renders nothing: `diff_projects`
//! returns typed values; turning them into text is each calling surface's
//! own job (design spec §5).
mod key;
mod semantic;
#[cfg(test)]
mod testutil;

pub use key::*;
