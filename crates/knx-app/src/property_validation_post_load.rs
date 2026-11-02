//! Journaled post-load octet observations: typed record, replay proof and status projection.
//!
//! Stored only when Net tolerated a declared firmware-maintained octet of a verified final
//! step (envelope format 5). Replay recomputes each record from the durable recovery
//! snapshot and the executed plan, so a record the declaration would not produce is refused.
use super::PointerObservationRun;
use crate::property_validation_bundle::invalid;
use knx_core::commissioning::property_download::PropertyDownloadPlan;
use knx_net::commissioning::{
    post_load_octets::PostLoadOctetObservation, property_recovery::PropertyRecoverySnapshot,
};
use serde::{Deserialize, Serialize};
use std::io;

/// One written octet that read back as its declared post-load value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostLoadOctetRecord {
    pub object: u8,
    pub address: u16,
    pub written: u8,
    pub observed: u8,
    /// The octet before the download; `None` when its allocation prestate was unreadable.
    pub prestate: Option<u8>,
}
impl From<&PostLoadOctetObservation> for PostLoadOctetRecord {
    fn from(o: &PostLoadOctetObservation) -> Self {
        Self {
            object: o.object,
            address: o.address,
            written: o.written,
            observed: o.observed,
            prestate: o.prestate,
        }
    }
}
impl PostLoadOctetRecord {
    fn observation(&self) -> PostLoadOctetObservation {
        PostLoadOctetObservation {
            object: self.object,
            address: self.address,
            written: self.written,
            observed: self.observed,
            prestate: self.prestate,
        }
    }
}

/// One recorded post-load octet with the run that observed it, for status display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PostLoadOctetProgress {
    pub run: PointerObservationRun,
    pub record: PostLoadOctetRecord,
}

/// Every record must be distinct and exactly what Net's declared tolerance reports for
/// `executed_plan` over the durable prestate; an empty list is never journaled.
pub(super) fn check_genuine(
    snapshot: &PropertyRecoverySnapshot,
    executed_plan: &PropertyDownloadPlan,
    records: &[PostLoadOctetRecord],
) -> io::Result<()> {
    if records.is_empty() {
        return Err(invalid("empty post-load octet observation"));
    }
    for (index, record) in records.iter().enumerate() {
        if records[..index]
            .iter()
            .any(|other| other.object == record.object && other.address == record.address)
        {
            return Err(invalid("post-load octet observed twice"));
        }
        if !snapshot.post_load_observation_is_genuine(executed_plan, &record.observation()) {
            return Err(invalid(
                "post-load octet observation is not admitted by the bound declaration",
            ));
        }
    }
    Ok(())
}
