//! Durable pre-write recovery record for Device Object PID_SERVICE_CONTROL.
//!
//! This is a property backup, not a whole-device image. It must be written
//! inside the same management session immediately after reading the property
//! and before the first write. Recovery is manual and requires rechecking
//! target identity and mask; this module never replays a write automatically.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use knx_core::IndividualAddress;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceControlBackup {
    pub format: u32,
    pub kind: String,
    pub taken: String,
    pub device: String,
    pub mask: String,
    pub object_index: u8,
    pub property_id: u8,
    /// Both original property octets, big-endian hexadecimal.
    pub octets: String,
}

impl ServiceControlBackup {
    fn new(device: IndividualAddress, mask: u16, raw: u16) -> Self {
        Self {
            format: 1,
            kind: "knxbench-service-control-backup".into(),
            taken: chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, false),
            device: device.to_string(),
            mask: format!("{mask:04X}"),
            object_index: 0,
            property_id: 8,
            octets: format!("{raw:04X}"),
        }
    }
}

/// Create a new owner-only file, sync bytes, read them back, then sync the
/// directory entry. Any failure refuses the caller's device write.
pub fn write_backup(
    dir: &Path,
    device: IndividualAddress,
    mask: u16,
    raw: u16,
) -> io::Result<PathBuf> {
    write_record(dir, ServiceControlBackup::new(device, mask, raw))
}

fn write_record(dir: &Path, record: ServiceControlBackup) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let stamp: String = record
        .taken
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect();
    let path = dir.join(format!(
        "service-control-{}-{stamp}.backup.json",
        record.device
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&path)?;
    serde_json::to_writer_pretty(&mut file, &record).map_err(io::Error::other)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    drop(file);
    let read_back: ServiceControlBackup =
        serde_json::from_slice(&fs::read(&path)?).map_err(io::Error::other)?;
    if read_back != record {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "service-control backup differs on readback",
        ));
    }
    File::open(dir)?.sync_all()?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_the_entire_original_property_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let device = IndividualAddress::new(1, 1, 67).unwrap();
        let path = write_backup(dir.path(), device, 0x0701, 0x0104).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let record: ServiceControlBackup = serde_json::from_str(&text).unwrap();
        assert_eq!(record.device, "1.1.67");
        assert_eq!(record.mask, "0701");
        assert_eq!(record.octets, "0104");
        assert_eq!(record.object_index, 0);
        assert_eq!(record.property_id, 8);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        // The writer itself must reject a colliding name without touching it.
        assert!(write_record(dir.path(), record).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
    }

    #[test]
    fn blocked_backup_directory_does_not_leave_a_recovery_claim() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("occupied");
        fs::write(&blocker, b"not a directory").unwrap();
        let address = IndividualAddress::new(1, 1, 67).unwrap();
        assert!(write_backup(&blocker, address, 0x0701, 0).is_err());
    }
}
