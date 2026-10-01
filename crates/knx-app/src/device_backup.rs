//! Durable per-download device backups and restore plans.
//!
//! The executor reads what a download is about to overwrite
//! ([`knx_core::commissioning::device_backup`]); this module keeps it. A
//! backup is written to a new file, flushed and synced, and read back and
//! compared before the executor is allowed to write to the device: a
//! backup that exists only in memory is no backup.
//!
//! Format 1, plain JSON so an operator can read it without KNXBench:
//!
//! ```json
//! { "format": 1, "kind": "knxbench-device-backup",
//!   "taken": "2026-09-29T14:10:00+02:00",
//!   "device": "1.1.67", "mask": "0701", "manufacturer": "0083",
//!   "application": "M-0083_A-0027-15-0BAC", "partial": null,
//!   "load_states": [{"machine": "address-table", "state": "Loaded"}],
//!   "regions": [{"address": "4000", "octets": "05..."}] }
//! ```
//!
//! `application` and `partial` say which plan the backup fits: a restore
//! rebuilds that plan and swaps the octets in
//! ([`knx_core::commissioning::device_backup::restore_plan`]), which refuses
//! any other shape.

use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use knx_core::commissioning::device_backup::{
    restore_plan, BackupRegion, DeviceBackup, RestoreError,
};
use knx_core::commissioning::load_control_memory::MemoryLoadStateMachine;
use knx_core::commissioning::load_state::{LoadState, MaskVersion};
use knx_core::commissioning::memory_download::MemoryDownloadPlan;
use knx_core::commissioning::partial_memory_download::derive_partial_plan;
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_core::IndividualAddress;
use serde::{Deserialize, Serialize};

const FORMAT: u32 = 1;
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
    if file.format != FORMAT {
        return Err(BackupFileError::Malformed(format!(
            "format {}; this build reads {FORMAT}",
            file.format
        )));
    }
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
    if plan
        .steps
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        != stored.plan_steps
    {
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
        let dir = temp.path().join("new/parent/backups");
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
            good.replace("\"format\": 1", "\"format\": 2"),
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
}
