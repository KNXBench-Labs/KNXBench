//! Datapoint type references, wire payloads and their codec.
//!
//! `DptRef` and `DptParseError` identify a datapoint type; `GroupValue` is
//! the payload of a group telegram, however it got here (bus, CLI, tests);
//! `codec` (in `codec.rs`) turns one into the other. The DPT catalogue
//! itself (`knx_master.xml`, 289 subtypes) belongs to the product database,
//! not to each project (DATA_MODEL §9) — referenced, not inlined.

use std::fmt;

pub mod codec;
pub mod resolve;

pub use codec::{
    decode, default_input_format, encode, encode_inferred_format, encoding_rulings, DptCodecError,
    DptEncodingRuling, DptInputFormat, DptValue,
};
pub use resolve::{
    group_address_dpt_from, resolve_group_address_dpt, resolve_project_group_address_dpts,
    GroupAddressDpt,
};

/// A reference to a datapoint type, e.g. `DPST-1-1` (main type 1, subtype 1)
/// or `DPT-1` (main type only, no subtype selected).
///
/// Ordered by main type, then subtype — the order a human would expect, and
/// the order `resolve::GroupAddressDpt::Conflict` sorts its entries in. That
/// order must be deterministic across runs (a `Conflict` vector feeds
/// straight into CLI output), which a derived `Ord` over two integer fields
/// gives for free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DptRef {
    pub main: u16,
    pub sub: Option<u16>,
}

impl DptRef {
    /// Parses a single `DPST-<main>-<sub>` or `DPT-<main>` token.
    ///
    /// This does not accept the space-separated lists observed in
    /// `ComObjectRef@DatapointType` (a polymorphic com object's set of
    /// acceptable types) — that belongs to application-program ingestion in
    /// `knx-productdb` (Session 4), not to a single reference type. See
    /// RESEARCH.md for the observed example.
    pub fn parse(s: &str) -> Result<Self, DptParseError> {
        let malformed = || DptParseError::Malformed(s.to_string());

        if let Some(rest) = s.strip_prefix("DPST-") {
            let [main, sub] = rest.split('-').collect::<Vec<_>>()[..] else {
                return Err(malformed());
            };
            let main: u16 = main.parse().map_err(|_| malformed())?;
            let sub: u16 = sub.parse().map_err(|_| malformed())?;
            return Ok(DptRef {
                main,
                sub: Some(sub),
            });
        }

        if let Some(rest) = s.strip_prefix("DPT-") {
            if rest.is_empty() || rest.contains('-') {
                return Err(malformed());
            }
            let main: u16 = rest.parse().map_err(|_| malformed())?;
            return Ok(DptRef { main, sub: None });
        }

        Err(malformed())
    }
}

impl fmt::Display for DptRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.sub {
            Some(sub) => write!(f, "DPST-{}-{}", self.main, sub),
            None => write!(f, "DPT-{}", self.main),
        }
    }
}

/// A `DptRef::parse` input was neither `DPST-<main>-<sub>` nor `DPT-<main>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DptParseError {
    Malformed(String),
}

impl std::error::Error for DptParseError {}

impl fmt::Display for DptParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DptParseError::Malformed(s) => write!(f, "malformed datapoint type reference: {s:?}"),
        }
    }
}

/// The wire form of a group value, before any DPT interpretation. A group
/// value is the payload of a group telegram — a domain concept (moved here
/// from `knx-net::cemi`, spec E4-D2), not an IP-transport one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupValue {
    /// Fits in the TPCI/APCI-low octet's 6 data bits (e.g. a DPT-1 boolean).
    Short(u8),
    /// One or more full octets follow the TPCI/APCI-low octet.
    Bytes(Vec<u8>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dpst_with_sub() {
        assert_eq!(
            DptRef::parse("DPST-1-1").unwrap(),
            DptRef {
                main: 1,
                sub: Some(1)
            }
        );
    }

    #[test]
    fn parses_dpt_without_sub() {
        assert_eq!(
            DptRef::parse("DPT-1").unwrap(),
            DptRef { main: 1, sub: None }
        );
    }

    #[test]
    fn displays_with_and_without_sub() {
        assert_eq!(
            DptRef {
                main: 14,
                sub: Some(19)
            }
            .to_string(),
            "DPST-14-19"
        );
        assert_eq!(DptRef { main: 1, sub: None }.to_string(), "DPT-1");
    }

    #[test]
    fn rejects_malformed_input() {
        assert!(DptRef::parse("bogus").is_err());
        assert!(DptRef::parse("DPST-1-1 DPST-1-2").is_err());
    }
}
