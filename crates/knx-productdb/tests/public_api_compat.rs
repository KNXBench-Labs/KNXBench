use knx_productdb::dynamic::parse::DynamicIngest;
use knx_productdb::parse::hardware::HardwareIngest;
use knx_productdb::parse::program::ProgramIngest;
use knx_productdb::report::TranslationCounts;
use knx_productdb::{FileKind, IngestOutcome, MasterIngest};

#[test]
fn pre_pdb3_public_outcomes_remain_constructible() {
    let mut translations = TranslationCounts::default();
    translations.add(TranslationCounts {
        program: 1,
        ..TranslationCounts::default()
    });
    assert_eq!(translations.program, 1);
    let _ = IngestOutcome::Ingested {
        sha256: String::new(),
        kind: FileKind::Unrecognized,
        unknown: 0,
        conflicts: Vec::new(),
        translations: TranslationCounts::default(),
    };
    let _ = HardwareIngest {
        unknown: Vec::new(),
        conflicts: Vec::new(),
    };
    let _ = ProgramIngest {
        program_id: String::new(),
        unknown: Vec::new(),
        conflicts: Vec::new(),
        translations: 0,
    };
    let _ = DynamicIngest {
        unknown: Vec::new(),
    };
    let _ = MasterIngest {
        unknown: Vec::new(),
        translations: 0,
        dropped_datapoint_types: 0,
    };
}
