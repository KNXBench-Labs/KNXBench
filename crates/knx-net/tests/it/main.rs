//! knx-net integration tests in one binary: one link, one test pool (ADR-0144).

mod diagnostic_export;
mod diagnostic_export_protocol;
mod final_step_failure;
mod hardware_write_gate;
mod live_commissioning_readonly;
mod live_gateway;
mod live_identify;
mod live_individual_address_write;
mod live_memory_readonly;
mod live_programming_mode;
mod private_telegram_log;
mod property_download;
mod property_recovery;
mod simulator_system2_state;
mod support_diagnostics;
mod system2_memory_boundary;
