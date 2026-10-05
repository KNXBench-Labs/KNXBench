//! Durable pre-write record for service control and its Verify Mode setup.
//!
//! This is a property backup, not a whole-device image. It must be written
//! inside the same management session after reading both original properties
//! and before any property write. Recovery is manual and requires rechecking
//! target identity and mask; this module never replays a write automatically.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use knx_core::commissioning::properties::{PID_DEVICE_CONTROL, PID_SERVICE_CONTROL};
use knx_core::{ContactableAddress, IndividualAddress};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "UncheckedBackup")]
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
    pub device_control_property_id: u8,
    /// Original PID_DEVICE_CONTROL octet, before setting Verify Mode.
    pub device_control_octets: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UncheckedBackup {
    format: u32,
    kind: String,
    taken: String,
    device: String,
    mask: String,
    object_index: u8,
    property_id: u8,
    octets: String,
    device_control_property_id: u8,
    device_control_octets: String,
}

impl TryFrom<UncheckedBackup> for ServiceControlBackup {
    type Error = io::Error;

    fn try_from(raw: UncheckedBackup) -> Result<Self, Self::Error> {
        let record = Self {
            format: raw.format,
            kind: raw.kind,
            taken: raw.taken,
            device: raw.device,
            mask: raw.mask,
            object_index: raw.object_index,
            property_id: raw.property_id,
            octets: raw.octets,
            device_control_property_id: raw.device_control_property_id,
            device_control_octets: raw.device_control_octets,
        };
        record.validate()?;
        Ok(record)
    }
}

impl ServiceControlBackup {
    fn validate(&self) -> io::Result<()> {
        fn hex(value: &str, width: usize) -> bool {
            value.len() == width && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        }
        let target = self.device.parse::<IndividualAddress>().ok();
        if self.format != 2
            || self.kind != "knxbench-service-control-backup"
            || chrono::DateTime::parse_from_rfc3339(&self.taken).is_err()
            || !target.is_some_and(|address| ContactableAddress::new(address).is_ok())
            || !hex(&self.mask, 4)
            || self.object_index != 0
            || self.property_id != PID_SERVICE_CONTROL
            || !hex(&self.octets, 4)
            || self.device_control_property_id != PID_DEVICE_CONTROL
            || !hex(&self.device_control_octets, 2)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported or invalid service-control recovery record",
            ));
        }
        Ok(())
    }

    fn new(device: IndividualAddress, mask: u16, raw: u16, device_control: u8) -> Self {
        Self {
            format: 2,
            kind: "knxbench-service-control-backup".into(),
            taken: chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, false),
            device: device.to_string(),
            mask: format!("{mask:04X}"),
            object_index: 0,
            property_id: PID_SERVICE_CONTROL,
            octets: format!("{raw:04X}"),
            device_control_property_id: PID_DEVICE_CONTROL,
            device_control_octets: format!("{device_control:02X}"),
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
    device_control: u8,
) -> io::Result<PathBuf> {
    write_record(
        dir,
        ServiceControlBackup::new(device, mask, raw, device_control),
    )
}

fn write_record(dir: &Path, record: ServiceControlBackup) -> io::Result<PathBuf> {
    write_record_with_sync(dir, record, |directory| File::open(directory)?.sync_all())
}

fn write_record_with_sync(
    dir: &Path,
    record: ServiceControlBackup,
    sync_directory: impl FnMut(&Path) -> io::Result<()>,
) -> io::Result<PathBuf> {
    record.validate()?;
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
    crate::backup_directory::sync_chain(dir, sync_directory)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_semantically_invalid_recovery_records() {
        let record = ServiceControlBackup::new("1.1.67".parse().unwrap(), 0x0701, 0x0104, 0x02);
        let valid = serde_json::to_value(&record).unwrap();
        let decoded: ServiceControlBackup = serde_json::from_value(valid.clone()).unwrap();
        assert_eq!(
            decoded, record,
            "original properties must roundtrip unchanged"
        );
        for (field, value) in [
            ("format", serde_json::json!(1)),
            ("format", serde_json::json!(3)),
            ("kind", serde_json::json!("foreign-backup")),
            ("taken", serde_json::json!("not-a-timestamp")),
            ("device", serde_json::json!("invalid-address")),
            ("device", serde_json::json!("1.1.220")),
            ("mask", serde_json::json!("070")),
            ("mask", serde_json::json!("ZZZZ")),
            ("object_index", serde_json::json!(1)),
            ("property_id", serde_json::json!(14)),
            ("octets", serde_json::json!("04")),
            ("octets", serde_json::json!("XXXX")),
            ("device_control_property_id", serde_json::json!(8)),
            ("device_control_octets", serde_json::json!("0002")),
            ("device_control_octets", serde_json::json!("XX")),
            ("mask", serde_json::json!("+701")),
            ("octets", serde_json::json!("+104")),
            ("device_control_octets", serde_json::json!("+2")),
            ("synthetic_extension", serde_json::json!("not-understood")),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = value;
            assert!(
                serde_json::from_value::<ServiceControlBackup>(invalid).is_err(),
                "invalid recovery field admitted: {field}"
            );
        }
    }

    #[test]
    fn every_original_device_control_octet_is_retained_without_bit_interpretation() {
        for device_control in u8::MIN..=u8::MAX {
            for service_control in [0, 0x0104, 0x8000, u16::MAX] {
                let record = ServiceControlBackup::new(
                    "1.1.67".parse().unwrap(),
                    0x0701,
                    service_control,
                    device_control,
                );
                let mut wire = serde_json::to_value(&record).unwrap();
                let decoded: ServiceControlBackup = serde_json::from_value(wire.clone()).unwrap();
                assert_eq!(decoded, record, "all original bits must be preserved");
                wire["device_control_octets"] = serde_json::json!(format!("{device_control:02x}"));
                let decoded: ServiceControlBackup = serde_json::from_value(wire.clone()).unwrap();
                assert_eq!(serde_json::to_value(decoded).unwrap(), wire);
            }
        }
    }

    #[test]
    fn invalid_records_are_refused_before_filesystem_changes() {
        let temp = tempfile::tempdir().unwrap();
        let seed = temp.path().join("existing.backup.json");
        let seed_bytes = b"existing recovery evidence must remain unchanged";
        fs::write(&seed, seed_bytes).unwrap();
        let valid = ServiceControlBackup::new("1.1.67".parse().unwrap(), 0x0701, 0x0104, 0x02);
        for (label, record) in [
            (
                "format",
                ServiceControlBackup {
                    format: 3,
                    ..valid.clone()
                },
            ),
            (
                "path",
                ServiceControlBackup {
                    device: "../../outside".into(),
                    ..valid.clone()
                },
            ),
            (
                "excluded-target",
                ServiceControlBackup {
                    device: "1.1.220".into(),
                    ..valid.clone()
                },
            ),
            (
                "original-octet",
                ServiceControlBackup {
                    device_control_octets: "+2".into(),
                    ..valid.clone()
                },
            ),
        ] {
            let dir = temp.path().join(format!("uncreated-{label}"));
            let mut synced = false;
            let result = write_record_with_sync(&dir, record, |_| {
                synced = true;
                Ok(())
            });
            assert!(
                !dir.exists(),
                "invalid recovery record must be refused before creating directories: {label}"
            );
            assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
            assert!(!synced, "a refused record must not reach directory sync");
            assert_eq!(fs::read(&seed).unwrap(), seed_bytes);
        }
    }

    #[test]
    fn nested_backup_directory_entries_are_all_synced() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("new/parent/backups");
        let record = ServiceControlBackup::new("1.1.67".parse().unwrap(), 0x0701, 0x0104, 0);
        let mut synced = Vec::new();
        let path = write_record_with_sync(&dir, record.clone(), |directory| {
            synced.push(directory.to_path_buf());
            File::open(directory)?.sync_all()
        })
        .unwrap();
        let expected: Vec<_> = dir.ancestors().map(Path::to_path_buf).collect();
        assert_eq!(synced, expected, "new parent entries need their own sync");
        let saved: ServiceControlBackup = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(saved, record);
    }

    #[test]
    fn parent_sync_failure_refuses_a_backup_receipt() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("new/parent/backups");
        let parent = dir.parent().unwrap();
        let record = ServiceControlBackup::new("1.1.67".parse().unwrap(), 0x0701, 0x0104, 0);
        let result = write_record_with_sync(&dir, record, |directory| {
            if directory == parent {
                return Err(io::Error::other("injected ancestor sync failure"));
            }
            File::open(directory)?.sync_all()
        });
        assert!(
            result.is_err(),
            "an unconfirmed ancestor is not a durable receipt"
        );
    }

    #[test]
    fn saves_the_entire_original_property_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let device = IndividualAddress::new(1, 1, 67).unwrap();
        let path = write_backup(dir.path(), device, 0x0701, 0x0104, 0x02).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let record: ServiceControlBackup = serde_json::from_str(&text).unwrap();
        let wire: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(wire["format"], 2, "v1 does not back up Verify Mode setup");
        assert_eq!(wire["device_control_property_id"], 14);
        assert_eq!(wire["device_control_octets"], "02");
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
        assert!(write_backup(&blocker, address, 0x0701, 0, 0).is_err());
    }
}
