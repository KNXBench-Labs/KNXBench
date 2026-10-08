//! Replay ledger that makes a catalog batch request safe to resend (DATA-03).
//!
//! A client whose `POST /api/devices` response was lost cannot tell whether
//! the batch was committed. When it sends a `requestId`, the server records
//! each *successful* outcome here; an identical resend gets that outcome back
//! instead of a second batch. The ledger is in memory, bounded, and cleared
//! whenever the open project is replaced, so it never outlives the project or
//! the server run it describes. A request that failed is not recorded: nothing
//! was committed, so running it again is the correct retry.

use std::collections::VecDeque;

use crate::domain::{CreatedCatalogDevice, CreationDiagnostic};

/// Outcomes kept per open project. A resend that arrives after its entry was
/// evicted is applied again; at the web client's one-batch-at-a-time pace
/// this bound is far beyond any realistic retry window.
pub const CATALOG_REQUEST_LEDGER_CAPACITY: usize = 256;

/// Longest accepted `requestId`, in bytes.
pub const MAX_CATALOG_REQUEST_ID_LEN: usize = 128;

/// Everything that decides what a catalog request creates. A resend must
/// match it exactly; the same ID with other content is refused rather than
/// silently answered with a different batch's outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogRequestFingerprint {
    pub line_id: Option<u32>,
    pub catalog_item_id: String,
    pub name: String,
    pub quantity: u32,
    pub allocate_addresses: bool,
    pub unique_names: bool,
    /// ADR-0093 placement and the previewed outcome: a resend with another
    /// target or another expectation is a different request.
    pub installation_id: Option<u8>,
    pub building_part_id: Option<u32>,
    pub expected: Option<Vec<(String, Option<knx_core::IndividualAddress>)>>,
}

/// The recorded result of one committed catalog request.
#[derive(Debug, Clone)]
pub struct RecordedCatalogRequest {
    pub diagnostics: Vec<CreationDiagnostic>,
    pub items: Vec<CreatedCatalogDevice>,
}

#[derive(Debug, Default)]
pub struct CatalogRequestLedger {
    entries: VecDeque<(String, CatalogRequestFingerprint, RecordedCatalogRequest)>,
}

impl CatalogRequestLedger {
    /// The recorded outcome for `id`, `None` if it was never committed (or
    /// was evicted), or an error if `id` was committed with other content.
    pub fn lookup(
        &self,
        id: &str,
        fingerprint: &CatalogRequestFingerprint,
    ) -> Result<Option<&RecordedCatalogRequest>, String> {
        match self.entries.iter().find(|(known, _, _)| known == id) {
            None => Ok(None),
            Some((_, recorded, outcome)) if recorded == fingerprint => Ok(Some(outcome)),
            Some(_) => Err(format!(
                "catalog request {id} was already used for a different request; send a new requestId"
            )),
        }
    }

    pub fn record(
        &mut self,
        id: String,
        fingerprint: CatalogRequestFingerprint,
        outcome: RecordedCatalogRequest,
    ) {
        if self.entries.len() == CATALOG_REQUEST_LEDGER_CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back((id, fingerprint, outcome));
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A request ID is 1–128 ASCII letters, digits, `-` or `_` (a UUID fits).
pub fn validate_request_id(id: &str) -> Result<(), String> {
    let valid = !id.is_empty()
        && id.len() <= MAX_CATALOG_REQUEST_ID_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if valid {
        Ok(())
    } else {
        Err(format!(
            "requestId must be 1–{MAX_CATALOG_REQUEST_ID_LEN} ASCII letters, digits, '-' or '_'"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fingerprint(name: &str) -> CatalogRequestFingerprint {
        CatalogRequestFingerprint {
            line_id: None,
            catalog_item_id: "CI".into(),
            name: name.into(),
            quantity: 2,
            allocate_addresses: false,
            unique_names: false,
            installation_id: None,
            building_part_id: None,
            expected: None,
        }
    }

    fn outcome() -> RecordedCatalogRequest {
        RecordedCatalogRequest {
            diagnostics: vec![],
            items: vec![],
        }
    }

    #[test]
    fn identical_resend_finds_its_outcome_and_other_content_is_refused() {
        let mut ledger = CatalogRequestLedger::default();
        assert!(ledger.lookup("a", &fingerprint("x")).unwrap().is_none());
        ledger.record("a".into(), fingerprint("x"), outcome());
        assert!(ledger.lookup("a", &fingerprint("x")).unwrap().is_some());
        assert!(ledger.lookup("a", &fingerprint("y")).is_err());
        assert!(ledger.lookup("b", &fingerprint("x")).unwrap().is_none());
    }

    #[test]
    fn the_ledger_is_bounded_and_evicts_the_oldest_entry() {
        let mut ledger = CatalogRequestLedger::default();
        for i in 0..=CATALOG_REQUEST_LEDGER_CAPACITY {
            ledger.record(format!("r{i}"), fingerprint("x"), outcome());
        }
        assert_eq!(ledger.len(), CATALOG_REQUEST_LEDGER_CAPACITY);
        assert!(ledger.lookup("r0", &fingerprint("x")).unwrap().is_none());
        assert!(ledger.lookup("r1", &fingerprint("x")).unwrap().is_some());
        ledger.clear();
        assert!(ledger.is_empty());
    }

    #[test]
    fn request_ids_are_bounded_ascii_tokens() {
        assert!(validate_request_id("3f2b-1c_A9").is_ok());
        assert!(validate_request_id(&"x".repeat(MAX_CATALOG_REQUEST_ID_LEN)).is_ok());
        for bad in ["", "has space", "ümlaut", "a/b"] {
            assert!(validate_request_id(bad).is_err(), "{bad:?}");
        }
        assert!(validate_request_id(&"x".repeat(MAX_CATALOG_REQUEST_ID_LEN + 1)).is_err());
    }
}
