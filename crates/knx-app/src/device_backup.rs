//! Durable per-download device backups and restore plans.
//!
//! The executor reads what a download is about to overwrite
//! ([`knx_core::commissioning::device_backup`]); this module keeps it. A
//! backup is written to a new file, flushed and synced, and read back and
//! compared before the executor is allowed to write to the device: a
//! backup that exists only in memory is no backup.
//!
//! Format 3, plain JSON so an operator can read it without KNXBench:
//!
//! ```json
//! { "format": 3, "kind": "knxbench-device-backup",
//!   "taken": "2026-09-29T14:10:00+02:00",
//!   "device": "1.1.67", "mask": "0701", "manufacturer": "0083",
//!   "application": "M-0083_A-0027-15-0BAC", "partial": null,
//!   "prior_application": "0083002715", "exemplar_serial": "008312345678",
//!   "load_states": [{"machine": "address-table", "state": "Loaded"}],
//!   "regions": [{"address": "4000", "octets": "05..."}] }
//! ```
//!
//! `application` and `partial` say which plan the backup fits: a restore
//! rebuilds that plan and swaps the octets in
//! ([`knx_core::commissioning::device_backup::restore_plan`]), which refuses
//! any other shape.
//!
//! `prior_application` is the five-octet application identity (object 3,
//! PID 13) the device answered before the first write, or `null` when it
//! gave no usable answer. Format 2 requires the field. Format 1 files are
//! still read, never rewritten, and their absent field means "not
//! recorded"; a format 1 file that carries the field is refused, so no
//! evidence can be smuggled into an older file. Builds that only know
//! format 1 refuse format 2 files: there is no downgrade path. A restore
//! needs recorded evidence or the plan's own identity precondition
//! ([`knx_core::commissioning::device_backup::restore_plan`]).
//!
//! `exemplar_serial` (format 3, required, nullable) is the twelve-hex-digit
//! Device Object serial (PID 11) read before the first write. It is private
//! evidence of this file only: it binds a restore to the same exemplar and
//! never reaches history, reports or HTTP. New files write uppercase hex,
//! reading accepts either case. Formats 1 and 2 read as "not recorded" and
//! refuse the field; a backup without a bindable serial is never restorable.

use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use knx_core::commissioning::device_backup::{
    restore_plan, BackupRegion, DeviceBackup, ExemplarEvidence, PriorApplication, RestoreError,
};
use knx_core::commissioning::load_control_memory::MemoryLoadStateMachine;
use knx_core::commissioning::load_state::{LoadState, MaskVersion};
use knx_core::commissioning::memory_download::{
    source_procedure_descriptions, ExemplarSerial, MemoryDownloadPlan,
};
use knx_core::commissioning::partial_memory_download::derive_partial_plan;
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_core::IndividualAddress;
use serde::{Deserialize, Serialize};

const FORMAT: u32 = 3;
/// The last format without `exemplar_serial`; still read.
const FORMAT_WITHOUT_EXEMPLAR: u32 = 2;
/// The last format without `prior_application`; still read.
const FORMAT_WITHOUT_PRIOR: u32 = 1;
const KIND: &str = "knxbench-device-backup";

/// A backup with what it was taken for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredBackup {
    /// The backup.
    pub backup: DeviceBackup,
    /// The application program the plan was built for.
    pub application: String,
    /// The partial selection the plan was derived with, if any.
    pub partial: Option<PartialDownloadParts>,
    /// When it was taken (RFC 3339).
    pub taken: String,
    /// The original procedure's step descriptions (no new memory payload).
    /// A restore must rederive these same checks and load records.
    pub plan_steps: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileLoadState {
    machine: String,
    state: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileRegion {
    address: String,
    octets: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FilePartial {
    parameters: bool,
    group_addresses: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupFile {
    format: u32,
    kind: String,
    taken: String,
    device: String,
    mask: String,
    manufacturer: String,
    application: String,
    partial: Option<FilePartial>,
    plan_steps: Vec<String>,
    load_states: Vec<FileLoadState>,
    regions: Vec<FileRegion>,
    /// Absent (`None`) is not the same as `null` (`Some(None)`).
    #[serde(default, deserialize_with = "present")]
    prior_application: Option<Option<String>>,
    /// Same present-versus-null distinction; required from format 3.
    #[serde(default, deserialize_with = "present")]
    exemplar_serial: Option<Option<String>>,
}

/// Keeps `null` apart from an absent field.
fn present<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(deserializer).map(Some)
}

/// Why a backup was not written or read.
#[derive(Debug)]
pub enum BackupFileError {
    /// The file system failed.
    Io(std::io::Error),
    /// Not a backup of a format this build reads.
    Malformed(String),
    /// What was read back is not what was written.
    ReadBackDiffers(PathBuf),
}

impl std::error::Error for BackupFileError {}

impl fmt::Display for BackupFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Malformed(e) => write!(f, "not a device backup this build reads: {e}"),
            Self::ReadBackDiffers(path) => {
                write!(f, "{} does not read back as written", path.display())
            }
        }
    }
}

impl From<std::io::Error> for BackupFileError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

fn machine_name(machine: MemoryLoadStateMachine) -> &'static str {
    match machine {
        MemoryLoadStateMachine::AddressTable => "address-table",
        MemoryLoadStateMachine::AssociationTable => "association-table",
        MemoryLoadStateMachine::ApplicationProgram => "application-program",
        MemoryLoadStateMachine::PeiProgram => "pei-program",
    }
}

fn machine_from(name: &str) -> Result<MemoryLoadStateMachine, BackupFileError> {
    Ok(match name {
        "address-table" => MemoryLoadStateMachine::AddressTable,
        "association-table" => MemoryLoadStateMachine::AssociationTable,
        "application-program" => MemoryLoadStateMachine::ApplicationProgram,
        "pei-program" => MemoryLoadStateMachine::PeiProgram,
        other => return Err(BackupFileError::Malformed(format!("machine {other:?}"))),
    })
}

fn state_from(name: &str) -> Result<LoadState, BackupFileError> {
    [
        LoadState::Unloaded,
        LoadState::Loaded,
        LoadState::Loading,
        LoadState::Error,
        LoadState::Unloading,
        LoadState::LoadCompleting,
    ]
    .into_iter()
    .find(|state| format!("{state:?}") == name)
    .ok_or_else(|| BackupFileError::Malformed(format!("load state {name:?}")))
}

fn hex(octets: &[u8]) -> String {
    octets.iter().map(|octet| format!("{octet:02X}")).collect()
}

fn unhex(text: &str) -> Result<Vec<u8>, BackupFileError> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return Err(BackupFileError::Malformed(format!("octets {text:?}")));
    }
    (0..text.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&text[i..i + 2], 16)
                .map_err(|_| BackupFileError::Malformed(format!("octets {text:?}")))
        })
        .collect()
}

fn prior_from(text: &str) -> Result<[u8; 5], BackupFileError> {
    let malformed = || BackupFileError::Malformed(format!("prior_application {text:?}"));
    if text.len() != 10 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(malformed());
    }
    unhex(text)?.try_into().map_err(|_| malformed())
}

fn serial_from(text: &str) -> Result<ExemplarSerial, BackupFileError> {
    // Never echo the text: it is the private serial or a damaged copy of it.
    let malformed = || BackupFileError::Malformed("exemplar_serial".to_owned());
    if text.len() != 2 * ExemplarSerial::OCTETS
        || !text.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(malformed());
    }
    let octets: [u8; ExemplarSerial::OCTETS] = unhex(text)?.try_into().map_err(|_| malformed())?;
    Ok(ExemplarSerial::new(octets))
}

fn hex16(text: &str, what: &str) -> Result<u16, BackupFileError> {
    if text.len() != 4 {
        return Err(BackupFileError::Malformed(format!("{what} {text:?}")));
    }
    u16::from_str_radix(text, 16)
        .map_err(|_| BackupFileError::Malformed(format!("{what} {text:?}")))
}

/// The backup as its file's text.
pub fn to_json(stored: &StoredBackup) -> String {
    let backup = &stored.backup;
    let file = BackupFile {
        format: FORMAT,
        kind: KIND.to_owned(),
        taken: stored.taken.clone(),
        device: backup.target.to_string(),
        mask: format!("{:04X}", backup.mask.0),
        manufacturer: format!("{:04X}", backup.manufacturer),
        application: stored.application.clone(),
        partial: stored.partial.map(|parts| FilePartial {
            parameters: parts.parameters,
            group_addresses: parts.group_addresses,
        }),
        plan_steps: stored.plan_steps.clone(),
        load_states: backup
            .load_states
            .iter()
            .map(|(machine, state)| FileLoadState {
                machine: machine_name(*machine).to_owned(),
                state: format!("{state:?}"),
            })
            .collect(),
        regions: backup
            .regions
            .iter()
            .map(|region| FileRegion {
                address: format!("{:04X}", region.address),
                octets: hex(&region.octets),
            })
            .collect(),
        prior_application: Some(match backup.prior_application {
            PriorApplication::Recorded(identity) => Some(hex(&identity)),
            PriorApplication::NotRecorded => None,
        }),
        exemplar_serial: Some(match backup.exemplar_serial {
            ExemplarEvidence::Recorded(serial) => Some(hex(&serial.octets())),
            ExemplarEvidence::NotRecorded => None,
        }),
    };
    serde_json::to_string_pretty(&file).expect("a backup serialises") + "\n"
}

/// A backup file's text, parsed.
pub fn from_json(text: &str) -> Result<StoredBackup, BackupFileError> {
    let file: BackupFile =
        serde_json::from_str(text).map_err(|e| BackupFileError::Malformed(e.to_string()))?;
    if file.kind != KIND {
        return Err(BackupFileError::Malformed(format!("kind {:?}", file.kind)));
    }
    let prior_application = match (file.format, &file.prior_application) {
        (FORMAT_WITHOUT_PRIOR, None) => PriorApplication::NotRecorded,
        (FORMAT_WITHOUT_PRIOR, Some(_)) => {
            return Err(BackupFileError::Malformed(format!(
                "format {FORMAT_WITHOUT_PRIOR} cannot carry prior_application"
            )))
        }
        (FORMAT_WITHOUT_EXEMPLAR | FORMAT, None) => {
            return Err(BackupFileError::Malformed(format!(
                "format {} requires prior_application (null if not recorded)",
                file.format
            )))
        }
        (FORMAT_WITHOUT_EXEMPLAR | FORMAT, Some(None)) => PriorApplication::NotRecorded,
        (FORMAT_WITHOUT_EXEMPLAR | FORMAT, Some(Some(text))) => {
            PriorApplication::Recorded(prior_from(text)?)
        }
        (other, _) => {
            return Err(BackupFileError::Malformed(format!(
                "format {other}; this build reads {FORMAT_WITHOUT_PRIOR} to {FORMAT}"
            )))
        }
    };
    let exemplar_serial = match (file.format, &file.exemplar_serial) {
        (FORMAT_WITHOUT_PRIOR | FORMAT_WITHOUT_EXEMPLAR, None) => ExemplarEvidence::NotRecorded,
        (_, Some(_)) if file.format != FORMAT => {
            return Err(BackupFileError::Malformed(format!(
                "format {} cannot carry exemplar_serial",
                file.format
            )))
        }
        (_, None) => {
            return Err(BackupFileError::Malformed(format!(
                "format {FORMAT} requires exemplar_serial (null if not recorded)"
            )))
        }
        (_, Some(None)) => ExemplarEvidence::NotRecorded,
        (_, Some(Some(text))) => ExemplarEvidence::Recorded(serial_from(text)?),
    };
    let target: IndividualAddress = file
        .device
        .parse()
        .map_err(|_| BackupFileError::Malformed(format!("device {:?}", file.device)))?;
    let load_states = file
        .load_states
        .iter()
        .map(|entry| Ok((machine_from(&entry.machine)?, state_from(&entry.state)?)))
        .collect::<Result<_, BackupFileError>>()?;
    let regions = file
        .regions
        .iter()
        .map(|region| {
            Ok(BackupRegion {
                address: hex16(&region.address, "address")?,
                octets: unhex(&region.octets)?,
            })
        })
        .collect::<Result<_, BackupFileError>>()?;
    Ok(StoredBackup {
        backup: DeviceBackup {
            target,
            mask: MaskVersion(hex16(&file.mask, "mask")?),
            manufacturer: hex16(&file.manufacturer, "manufacturer")?,
            load_states,
            regions,
            prior_application,
            exemplar_serial,
        },
        application: file.application,
        partial: file.partial.map(|p| PartialDownloadParts {
            parameters: p.parameters,
            group_addresses: p.group_addresses,
        }),
        taken: file.taken,
        plan_steps: file.plan_steps,
    })
}

/// Why a backup could not be turned back into a plan.
#[derive(Debug)]
pub enum RestorePrepareError {
    /// The backup's application no longer plans offline.
    Plan(String),
    /// The partial selection does not derive.
    Partial(knx_core::commissioning::partial_memory_download::PartialPlanError),
    /// The plan's shape does not fit the backup.
    Shape(RestoreError),
    /// The procedure reconstructed from product data has changed.
    DifferentProcedure,
    /// A backup taken from a non-loaded part cannot fully restore it.
    NotLoaded,
}

impl std::error::Error for RestorePrepareError {}

impl fmt::Display for RestorePrepareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plan(e) => write!(f, "the backup's application does not plan: {e}"),
            Self::Partial(e) => write!(f, "the backup's partial selection: {e}"),
            Self::Shape(e) => write!(f, "{e}"),
            Self::DifferentProcedure => f.write_str("the product's download steps differ from the backup; restore refused"),
            Self::NotLoaded => f.write_str("the backup was taken while a part was not Loaded; a restore cannot reproduce that state"),
        }
    }
}

/// The plan that writes `stored` back: the backup's application planned
/// offline from the product database, cut to the backup's partial
/// selection, with the backed-up octets in place of the planned ones.
/// No project is needed; the load procedure's steps are the product's, and
/// [`restore_plan`] refuses a plan of any other shape.
pub fn prepare_restore(
    conn: &knx_productdb::Connection,
    stored: &StoredBackup,
) -> Result<MemoryDownloadPlan, RestorePrepareError> {
    if !stored.backup.was_loaded() {
        return Err(RestorePrepareError::NotLoaded);
    }
    let complete = crate::download_support::offline_plan(conn, &stored.application)
        .map_err(|(_, detail)| RestorePrepareError::Plan(detail))?;
    let plan = match stored.partial {
        None => complete,
        Some(parts) => {
            derive_partial_plan(&complete, parts)
                .map_err(RestorePrepareError::Partial)?
                .plan
        }
    };
    if source_procedure_descriptions(&plan) != stored.plan_steps {
        return Err(RestorePrepareError::DifferentProcedure);
    }
    restore_plan(&plan, stored.backup.target, &stored.backup).map_err(RestorePrepareError::Shape)
}

/// The file name a backup of `target` taken at `stamp` gets: sortable, and
/// without characters a file system might refuse.
pub fn file_name(target: IndividualAddress, stamp: &str) -> String {
    let stamp: String = stamp
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("{target}_{stamp}.backup.json")
}

/// Writes `stored` to a new file in `dir` (created if missing), syncs it,
/// reads it back and compares. Never overwrites an existing file.
pub fn write_backup(dir: &Path, stored: &StoredBackup) -> Result<PathBuf, BackupFileError> {
    write_backup_with_sync(dir, stored, |directory| {
        fs::File::open(directory)?.sync_all()
    })
}

fn write_backup_with_sync(
    dir: &Path,
    stored: &StoredBackup,
    sync_directory: impl FnMut(&Path) -> std::io::Result<()>,
) -> Result<PathBuf, BackupFileError> {
    fs::create_dir_all(dir)?;
    let path = dir.join(file_name(stored.backup.target, &stored.taken));
    let text = to_json(stored);
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    // The device's memory can carry private configuration or keys.
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&path)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    drop(file);
    let back = read_backup(&path)?;
    if back != *stored {
        return Err(BackupFileError::ReadBackDiffers(path));
    }
    // An fsynced file is not yet durable without its directory entry.
    crate::backup_directory::sync_chain(dir, sync_directory)?;
    Ok(path)
}

/// Reads a backup file.
pub fn read_backup(path: &Path) -> Result<StoredBackup, BackupFileError> {
    from_json(&fs::read_to_string(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERIAL: [u8; 6] = [0x00, 0x83, 0x12, 0xAB, 0xCD, 0xEF];
    const EXEMPLAR: &str = "\"exemplar_serial\": \"008312ABCDEF\"";

    fn stored() -> StoredBackup {
        StoredBackup {
            backup: DeviceBackup {
                target: "1.1.67".parse().unwrap(),
                mask: MaskVersion(0x0701),
                manufacturer: 0x0083,
                load_states: vec![
                    (MemoryLoadStateMachine::AddressTable, LoadState::Loaded),
                    (MemoryLoadStateMachine::ApplicationProgram, LoadState::Error),
                ],
                regions: vec![
                    BackupRegion {
                        address: 0x4000,
                        octets: vec![0x05],
                    },
                    BackupRegion {
                        address: 0x4400,
                        octets: vec![0x00, 0xAB, 0xFF],
                    },
                ],
                prior_application: PriorApplication::Recorded([0x00, 0x83, 0x00, 0x27, 0x15]),
                exemplar_serial: ExemplarEvidence::Recorded(ExemplarSerial::new(SERIAL)),
            },
            application: "M-0083_A-0027-15-0BAC".into(),
            partial: Some(PartialDownloadParts {
                parameters: true,
                group_addresses: false,
            }),
            taken: "2026-09-29T14:10:00+02:00".into(),
            plan_steps: vec!["connect; check mask and manufacturer".into()],
        }
    }

    #[test]
    fn nested_backup_directory_entries_are_all_synced() {
        let temp = tempfile::tempdir().unwrap();
        // A resolved base, so the chain is exactly the supplied path even when
        // the runner's TMPDIR is a symlink (the alias case has its own test in
        // `backup_directory`).
        let dir = temp
            .path()
            .canonicalize()
            .unwrap()
            .join("new/parent/backups");
        let mut synced = Vec::new();
        let path = write_backup_with_sync(&dir, &stored(), |directory| {
            synced.push(directory.to_path_buf());
            fs::File::open(directory)?.sync_all()
        })
        .unwrap();
        let expected: Vec<_> = dir.ancestors().map(Path::to_path_buf).collect();
        assert_eq!(synced, expected, "new parent entries need their own sync");
        assert_eq!(read_backup(&path).unwrap(), stored());
    }

    #[test]
    fn parent_sync_failure_refuses_a_backup_receipt() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("new/parent/backups");
        let parent = dir.parent().unwrap();
        let result = write_backup_with_sync(&dir, &stored(), |directory| {
            if directory == parent {
                return Err(std::io::Error::other("injected ancestor sync failure"));
            }
            fs::File::open(directory)?.sync_all()
        });
        assert!(
            result.is_err(),
            "an unconfirmed ancestor is not a durable receipt"
        );
    }

    #[test]
    fn a_backup_survives_its_file() {
        assert_eq!(from_json(&to_json(&stored())).unwrap(), stored());
        let mut complete = stored();
        complete.partial = None;
        assert_eq!(from_json(&to_json(&complete)).unwrap(), complete);
    }

    #[test]
    fn restore_refuses_an_incompletely_loaded_backup() {
        let mut damaged = stored();
        let dir = tempfile::tempdir().unwrap();
        let products =
            knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        assert!(matches!(
            prepare_restore(&products, &damaged),
            Err(RestorePrepareError::NotLoaded)
        ));
        damaged.backup.load_states[1].1 = LoadState::Loaded;
        assert!(matches!(
            prepare_restore(&products, &damaged),
            Err(RestorePrepareError::Plan(_))
        ));
    }

    #[test]
    fn the_file_is_readable_without_knxbench() {
        let text = to_json(&stored());
        for needle in [
            "\"device\": \"1.1.67\"",
            "\"mask\": \"0701\"",
            "\"address\": \"4400\"",
            "\"octets\": \"00ABFF\"",
            "\"machine\": \"application-program\"",
            "\"state\": \"Error\"",
        ] {
            assert!(text.contains(needle), "{needle} in {text}");
        }
    }

    #[test]
    fn a_file_is_written_once_synced_and_never_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_backup(&dir.path().join("backups"), &stored()).unwrap();
        assert_eq!(
            path.file_name().unwrap().to_str().unwrap(),
            "1.1.67_2026-09-29T14-10-00-02-00.backup.json"
        );
        assert_eq!(read_backup(&path).unwrap(), stored());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert!(matches!(
            write_backup(&dir.path().join("backups"), &stored()),
            Err(BackupFileError::Io(e)) if e.kind() == std::io::ErrorKind::AlreadyExists
        ));
    }

    #[test]
    fn a_foreign_or_damaged_file_is_refused() {
        let good = to_json(&stored());
        for bad in [
            good.replace("\"format\": 3", "\"format\": 4"),
            good.replace("\"format\": 3", "\"format\": 0"),
            good.replace(KIND, "something-else"),
            good.replace("\"00ABFF\"", "\"00ABF\""),
            good.replace("\"00ABFF\"", "\"00ABZZ\""),
            good.replace("\"4400\"", "\"44000\""),
            good.replace("\"Error\"", "\"Broken\""),
            good.replace("\"application-program\"", "\"kitchen\""),
            good.replace("\"1.1.67\"", "\"1.1.999\""),
            good.replace("\"plan_steps\"", "\"missing_plan_steps\""),
            good.replace("\"kind\"", "\"extra\": 1, \"kind\""),
            "{".to_owned(),
        ] {
            assert!(
                matches!(from_json(&bad), Err(BackupFileError::Malformed(_))),
                "{bad}"
            );
        }
    }

    const PRIOR: &str = "\"prior_application\": \"0083002715\"";

    fn without(text: &str, field: &str) -> String {
        let mut value: serde_json::Value = serde_json::from_str(text).unwrap();
        value.as_object_mut().unwrap().remove(field);
        serde_json::to_string_pretty(&value).unwrap()
    }

    fn without_prior(text: &str) -> String {
        without(text, "prior_application")
    }

    /// `text` as a format-1 file: neither later field.
    fn as_format_one(text: &str) -> String {
        without(&without_prior(text), "exemplar_serial").replace("\"format\": 3", "\"format\": 1")
    }

    /// `text` as a format-2 file: no serial.
    fn as_format_two(text: &str) -> String {
        without(text, "exemplar_serial").replace("\"format\": 3", "\"format\": 2")
    }

    #[test]
    fn format_three_writes_the_prior_application_or_an_explicit_null() {
        let text = to_json(&stored());
        assert!(text.contains("\"format\": 3"), "{text}");
        assert!(text.contains(PRIOR), "{text}");
        let mut unrecorded = stored();
        unrecorded.backup.prior_application = PriorApplication::NotRecorded;
        let text = to_json(&unrecorded);
        assert!(text.contains("\"prior_application\": null"), "{text}");
        assert_eq!(from_json(&text).unwrap(), unrecorded);
    }

    #[test]
    fn format_one_reads_as_not_recorded_and_is_never_rewritten() {
        let old = as_format_one(&to_json(&stored()));
        assert!(!old.contains("prior_application"), "{old}");
        let read = from_json(&old).unwrap();
        assert_eq!(read.backup.prior_application, PriorApplication::NotRecorded);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.backup.json");
        fs::write(&path, &old).unwrap();
        read_backup(&path).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            old,
            "reading rewrites nothing"
        );
    }

    #[test]
    fn format_one_cannot_carry_the_new_field() {
        let good = to_json(&stored()).replace("\"format\": 3", "\"format\": 1");
        for bad in [
            good.clone(),
            good.replace(PRIOR, "\"prior_application\": null"),
        ] {
            assert!(
                matches!(from_json(&bad), Err(BackupFileError::Malformed(_))),
                "{bad}"
            );
        }
    }

    #[test]
    fn format_three_refuses_a_missing_or_malformed_prior_application() {
        let good = to_json(&stored());
        for bad in [
            without_prior(&good),
            good.replace(PRIOR, "\"prior_application\": \"00830027\""),
            good.replace(PRIOR, "\"prior_application\": \"008300271500\""),
            good.replace(PRIOR, "\"prior_application\": \"00830027ZZ\""),
            good.replace(PRIOR, "\"prior_application\": \"+083002715\""),
            good.replace(PRIOR, "\"prior_application\": \"\""),
            good.replace(PRIOR, "\"prior_application\": 83002715"),
            good.replace(PRIOR, "\"prior_application\": [0,131,0,39,21]"),
            good.replace(PRIOR, &format!("{PRIOR}, {PRIOR}")),
            good.replace(PRIOR, &format!("{PRIOR}, \"prior\": 1")),
        ] {
            assert!(
                matches!(from_json(&bad), Err(BackupFileError::Malformed(_))),
                "{bad}"
            );
        }
    }

    #[test]
    fn a_legacy_complete_backup_reads_but_cannot_be_restored() {
        let dir = tempfile::tempdir().unwrap();
        let products =
            knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        let mut legacy = stored();
        legacy.backup.load_states[1].1 = LoadState::Loaded;
        legacy.backup.prior_application = PriorApplication::NotRecorded;
        legacy.backup.exemplar_serial = ExemplarEvidence::NotRecorded;
        // The plan does not derive from an empty product database, so the
        // identity refusal itself is covered in `knx-core`; here the file
        // still reads and reaches the plan step.
        assert!(matches!(
            prepare_restore(&products, &legacy),
            Err(RestorePrepareError::Plan(_))
        ));
        let text = as_format_one(&to_json(&legacy));
        assert_eq!(from_json(&text).unwrap(), legacy);
    }

    #[test]
    fn format_three_writes_the_serial_in_uppercase_or_an_explicit_null() {
        let text = to_json(&stored());
        assert!(text.contains(EXEMPLAR), "{text}");
        assert_eq!(from_json(&text).unwrap(), stored());
        let mut unrecorded = stored();
        unrecorded.backup.exemplar_serial = ExemplarEvidence::NotRecorded;
        let text = to_json(&unrecorded);
        assert!(text.contains("\"exemplar_serial\": null"), "{text}");
        assert_eq!(from_json(&text).unwrap(), unrecorded);
        let lower = to_json(&stored()).replace("008312ABCDEF", "008312abcdef");
        assert_eq!(
            from_json(&lower).unwrap(),
            stored(),
            "reading accepts either case"
        );
    }

    #[test]
    fn format_three_refuses_a_missing_or_malformed_serial_without_echoing_it() {
        let good = to_json(&stored());
        for bad in [
            without(&good, "exemplar_serial"),
            good.replace(EXEMPLAR, "\"exemplar_serial\": \"008312ABCD\""),
            good.replace(EXEMPLAR, "\"exemplar_serial\": \"008312ABCDEF00\""),
            good.replace(EXEMPLAR, "\"exemplar_serial\": \"008312ABCDEZ\""),
            good.replace(EXEMPLAR, "\"exemplar_serial\": \"+08312ABCDEF\""),
            good.replace(EXEMPLAR, "\"exemplar_serial\": \"\""),
            good.replace(EXEMPLAR, "\"exemplar_serial\": 8312"),
            good.replace(EXEMPLAR, "\"exemplar_serial\": [0,131,18,171,205,239]"),
            good.replace(EXEMPLAR, &format!("{EXEMPLAR}, {EXEMPLAR}")),
            good.replace(EXEMPLAR, &format!("{EXEMPLAR}, \"serial\": 1")),
        ] {
            let error = from_json(&bad).expect_err(&bad);
            assert!(matches!(error, BackupFileError::Malformed(_)), "{bad}");
            assert!(!error.to_string().contains("ABCDEF"), "{error}");
        }
    }

    #[test]
    fn older_formats_read_without_a_serial_and_refuse_a_smuggled_one() {
        for (old, name) in [
            (as_format_one(&to_json(&stored())), 1),
            (as_format_two(&to_json(&stored())), 2),
        ] {
            let read = from_json(&old).unwrap();
            assert_eq!(
                read.backup.exemplar_serial,
                ExemplarEvidence::NotRecorded,
                "format {name}"
            );
        }
        let smuggled_two = to_json(&stored()).replace("\"format\": 3", "\"format\": 2");
        let smuggled_null = as_format_two(&to_json(&stored())).replace(
            "\"prior_application\"",
            "\"exemplar_serial\": null, \"prior_application\"",
        );
        let smuggled_one = to_json(&stored()).replace("\"format\": 3", "\"format\": 1");
        for bad in [smuggled_two, smuggled_null, smuggled_one] {
            assert!(
                matches!(from_json(&bad), Err(BackupFileError::Malformed(_))),
                "{bad}"
            );
        }
    }

    #[test]
    fn restore_never_leaks_the_serial_through_debug() {
        let shown = format!("{:?}", stored());
        assert!(!shown.to_uppercase().contains("ABCDEF"), "{shown}");
    }

    #[test]
    fn a_non_ascii_serial_is_refused_without_a_panic_or_an_echo() {
        let good = to_json(&stored());
        // 12 bytes long but not 12 ASCII hex digits; and a 6-char multi-byte text.
        for text in [
            "00831234567\u{e4}",
            "\u{e4}\u{e4}\u{e4}\u{e4}\u{e4}\u{e4}",
            "0083\u{ff10}\u{ff11}2345",
        ] {
            let bad = good.replace("008312ABCDEF", text);
            let error = from_json(&bad).expect_err(text);
            assert!(matches!(error, BackupFileError::Malformed(_)), "{text}");
            assert!(!error.to_string().contains(text), "{error}");
        }
    }

    #[test]
    fn a_format_one_file_carrying_only_the_serial_is_refused() {
        // No prior_application (as format 1 requires) but a smuggled exemplar_serial.
        let only_serial =
            without_prior(&to_json(&stored())).replace("\"format\": 3", "\"format\": 1");
        assert!(only_serial.contains("exemplar_serial"), "{only_serial}");
        assert!(!only_serial.contains("prior_application"), "{only_serial}");
        assert!(matches!(
            from_json(&only_serial),
            Err(BackupFileError::Malformed(_))
        ));
        let null_serial = only_serial.replace("\"008312ABCDEF\"", "null");
        assert!(matches!(
            from_json(&null_serial),
            Err(BackupFileError::Malformed(_))
        ));
    }
}
