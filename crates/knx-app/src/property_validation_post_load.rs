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
    /// The octet before the download; `null` when its allocation prestate was unreadable.
    /// The key is required, so each record has exactly one encoding.
    #[serde(deserialize_with = "Option::deserialize")]
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

#[cfg(test)]
mod tests {
    use super::super::{
        DownloadFacts, DownloadResult, ExecutionBuild, LocalOutcome, OperationProgress,
        System2Report,
    };
    use super::*;

    const FLAG: PostLoadOctetRecord = PostLoadOctetRecord {
        object: 3,
        address: 0x3CF4,
        written: 1,
        observed: 0,
        prestate: Some(0xA5),
    };

    #[test]
    fn a_record_without_its_prestate_key_is_refused() {
        let complete = serde_json::to_value(FLAG).unwrap();
        assert_eq!(complete["prestate"], 0xA5);
        let mut missing = complete.clone();
        missing.as_object_mut().unwrap().remove("prestate");
        assert!(serde_json::from_value::<PostLoadOctetRecord>(missing).is_err());
        let unreadable = PostLoadOctetRecord {
            prestate: None,
            ..FLAG
        };
        let null = serde_json::to_value(unreadable).unwrap();
        assert!(null["prestate"].is_null(), "null is written, never omitted");
        assert_eq!(
            serde_json::from_value::<PostLoadOctetRecord>(null).unwrap(),
            unreadable
        );
    }

    #[test]
    fn the_shareable_report_names_run_and_object_only() {
        let facts = DownloadFacts {
            result: DownloadResult::Verified,
            outcome: LocalOutcome::Finished,
            completed_steps: 27,
            total_steps: Some(27),
        };
        let progress = OperationProgress {
            outcome: LocalOutcome::Finished,
            completed_steps: 27,
            total_steps: Some(27),
            backup_available: true,
            restore_available: true,
            mutation_possible: true,
            recovery_phrases: None,
            prestate_differs: false,
            pointer_observations: Vec::new(),
            post_load_octets: vec![PostLoadOctetProgress {
                run: PointerObservationRun::Download,
                record: FLAG,
            }],
            program_version_observation: None,
            prestate_unreadable_allocations: Vec::new(),
            failure_diagnostics: Vec::new(),
            final_step_failures: Vec::new(),
            download: facts,
            restore_attempted: false,
        };
        let report = serde_json::to_value(
            System2Report::from_progress(&progress, ExecutionBuild::Unknown, "0.1.0").unwrap(),
        )
        .unwrap();
        assert_eq!(report["format"], 4);
        assert_eq!(
            report["postLoadOctets"],
            serde_json::json!([{"run": "download", "object": 3}])
        );
        let text = report.to_string();
        for leak in [
            "\"address\"",
            "\"written\"",
            "\"observed\"",
            "\"prestate\"",
            "15604",
        ] {
            assert!(!text.contains(leak), "report leaks {leak}");
        }
    }
}
