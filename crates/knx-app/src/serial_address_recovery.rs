//! Fail-closed recovery gate for serial-number individual-address writes.
//!
//! MP §2.5 identifies a device by serial number and reads back the resulting
//! individual address, but our CLI/HTTP paths currently persist no pre-write
//! device recovery evidence. A confirmation phrase is not a backup. Until a
//! complete, durable pre-write recovery contract is implemented and tested,
//! production entry points must refuse before opening a tunnel.

/// Refuse a serial-address write until its affected storage can be backed up
/// and the backup durably read back before the management telegram is sent.
/// This has no opt-out: merely recording the previous address after a write
/// or accepting a caller-supplied path would not establish that precondition.
pub fn require_persistent_pre_write_recovery() -> Result<(), &'static str> {
    Err("serial-address write blocked: no verified durable pre-write backup of all affected storage areas; read-only find-serial remains available, but no serial-address write is authorized")
}
