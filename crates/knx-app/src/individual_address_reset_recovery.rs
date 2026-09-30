//! Fail-closed recovery gate for MP §2.18 individual-address resets.
//!
//! A broadcast reset can affect every device in programming mode. A typed
//! phrase and an address list do not persist the previous affected storage;
//! neither a closing broadcast read nor a later address reprogramming step is
//! a complete pre-write backup. Public CLI entry points must refuse before a
//! tunnel until an action-specific, durable and read-back recovery record for
//! every affected device is implemented and tested.

/// No current production caller can prove full affected-storage recovery.
/// Do not introduce an opt-out or treat a prior address alone as a backup.
pub fn require_persistent_pre_write_recovery() -> Result<(), &'static str> {
    Err("individual-address reset blocked: no verified durable pre-write backup of all affected storage areas for every device in programming mode; plan-only mode remains available, but no address reset is authorized")
}
