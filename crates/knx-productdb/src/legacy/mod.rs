//! Legacy ETS3-era EX-IM files (`.vd3`–`.vd5`, `.pr3`–`.pr5`): detection, decryption, grammar.
//!
//! These files are not `.knxprod` packages. They are a ZIP archive with one
//! member (`ets.vd_`, `ets2.vd_` or `ets.pr_`), normally ZipCrypto-encrypted,
//! that holds a CRLF-separated text table dump called `EX-IM` here after its
//! first line. *The KNX Standard* names `vd3`–`vd5` as the ETS3 end-user
//! product database format but does not describe its bytes, so every rule in
//! this module is an observation of real files
//! (docs/superpowers/specs/2026-09-26-legacy-vd-pr-product-import-design.md,
//! ADR-0094), not a normative grammar.
//!
//! This path is deliberately separate from the modern package parser:
//! nothing here uses `quick_xml`, and no `.knxprod` code calls the EX-IM
//! parser. This crate does not decrypt and holds no password: the
//! application layer (`knx_app::legacy`) decrypts with a password the user
//! supplied, through `knx-secure`, and never guesses one.

mod container;
mod error;
mod exim;
mod inspect;
mod text;

pub use container::{
    detect_legacy_container, read_legacy_member, LegacyCheckBytes, LegacyContainer, LegacyMember,
    LegacyMemberKind, LegacyPayload, MAX_LEGACY_FILE, MAX_LEGACY_PAYLOAD,
};
pub use error::LegacyError;
pub use exim::{
    parse_exim, parse_exim_with_limits, ExImColumn, ExImContent, ExImDiagnostic, ExImDocument,
    ExImLimits, ExImTable,
};
pub use inspect::{
    inspect_payload, LegacyInspection, LegacyProductSummary, LegacyTableSummary,
    PAYLOAD_CHARSET_ASSUMPTION,
};
