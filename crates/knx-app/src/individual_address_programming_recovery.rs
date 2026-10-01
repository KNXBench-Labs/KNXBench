//! Fail-closed recovery gate for MP §2.3 button-based individual-address writes.
//!
//! A valid confirmation phrase and a broadcast programming-mode read identify
//! the intended write, but neither persists the current device's affected
//! storage. The device may not be known until after tunnel opening. Production
//! CLI/HTTP entry points must refuse before any tunnel until a durable,
//! device-specific pre-write backup/readback and recovery contract exists.

/// No public caller currently proves complete, durable pre-write recovery.
/// No flag, phrase or operator-provided path can substitute for that proof.
pub fn require_persistent_pre_write_recovery() -> Result<(), &'static str> {
    Err("individual-address programming blocked: no verified durable pre-write backup of all affected storage areas for the device in programming mode; read-only planning remains available, but no address write is authorized")
}
