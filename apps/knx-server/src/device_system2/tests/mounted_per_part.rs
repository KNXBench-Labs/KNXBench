//! Mounted worker: a verified run settles per part through the App helper, not the old bool.
use super::failure_diagnostic::{fixture, settled_slow, Fixture};
use super::*;

fn belief(state: &crate::SharedState, device: DeviceId) -> [bool; 4] {
    let project = state.project.lock().unwrap();
    let c = &project
        .as_ref()
        .unwrap()
        .devices
        .get(device)
        .unwrap()
        .commissioning;
    [
        c.individual_address_loaded,
        c.application_program_loaded,
        c.parameters_loaded,
        c.communication_part_loaded,
    ]
}

/// Marks every part of the device, before the preparation so the start binding sees them.
fn mark_all(state: &crate::SharedState, device: DeviceId) {
    let mut project = state.project.lock().unwrap();
    let c = &mut project
        .as_mut()
        .unwrap()
        .devices
        .get_mut(device)
        .unwrap()
        .commissioning;
    c.individual_address_loaded = true;
    c.application_program_loaded = true;
    c.parameters_loaded = true;
    c.communication_part_loaded = true;
}

/// A verified mounted run whose project is edited while the first data write is paused,
/// after start admission. Returns the beliefs after the terminal settlement.
async fn verified_run_edited_while_paused(
    marked_before: bool,
    edit: impl FnOnce(&crate::SharedState, DeviceId),
) -> [bool; 4] {
    let Fixture {
        directory: _directory,
        hardware,
        state,
        router,
        device,
    } = fixture().await;
    if marked_before {
        mark_all(&state, device);
    }
    hardware.pause_data_write.store(true, Ordering::SeqCst);
    let preview = prepare(&router, device).await;
    http(
        &router,
        "/api/device-system2/start",
        Some(start(&preview)),
        200,
    )
    .await;
    tokio::time::timeout(Duration::from_secs(60), hardware.data_written.notified())
        .await
        .expect("worker must reach an actual data write after start admission");
    edit(&state, device);
    hardware.release_write.notify_one();
    settled_slow(&state, &router, "finished").await;
    let status = http(&router, "/api/device-system2/status", None, 200).await;
    assert_eq!(status["downloadResult"], "verified");
    assert!(
        matches!(
            status["projectSettlement"].as_str(),
            Some("persisted" | "sessionOnly")
        ),
        "{}",
        status["projectSettlement"]
    );
    let address = state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .devices
        .get(device)
        .unwrap()
        .address;
    assert_eq!(
        address,
        Some(TARGET.parse().unwrap()),
        "the address is the project's own"
    );
    belief(&state, device)
}

fn edit_device(
    state: &crate::SharedState,
    device: DeviceId,
    edit: impl FnOnce(&mut knx_core::DeviceInstance),
) {
    edit(
        state
            .project
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .devices
            .get_mut(device)
            .unwrap(),
    );
}

#[tokio::test]
#[ignore = "requires digest-pinned public package; set KNXBENCH_PUBLIC_UP258; no network or hardware"]
async fn s2_per_part_a_rename_during_a_verified_run_still_credits_every_part() {
    // After start admission: a rename changes the whole preparation, not what was written.
    let after = verified_run_edited_while_paused(false, |state, device| {
        edit_device(state, device, |d| {
            d.name.push_str(" renamed during the run");
        });
    })
    .await;
    assert_eq!(after, [true; 4]);
}

#[tokio::test]
#[ignore = "requires digest-pinned public package; set KNXBENCH_PUBLIC_UP258; no network or hardware"]
async fn s2_per_part_a_parameter_edit_during_a_verified_run_credits_address_and_application() {
    // Another option of the source's own PIR field: a real, admitted parameter value.
    let other = alternative_pir_value().await;
    let after = verified_run_edited_while_paused(false, move |state, device| {
        let mut project = state.project.lock().unwrap();
        let parameter = project
            .as_mut()
            .unwrap()
            .installations
            .iter_mut()
            .flat_map(|installation| &mut installation.parameters)
            .find(|p| p.device == device && p.source.ets_id == PIR_REFERENCE)
            .expect("the fixture stored the nondefault PIR value");
        assert_eq!(parameter.raw, PIR_RAW);
        parameter.raw = other;
    })
    .await;
    // Parameters and the group tables are pending; address and application program stay.
    assert_eq!(after, [true, true, false, false]);
}

#[tokio::test]
#[ignore = "requires digest-pinned public package; set KNXBENCH_PUBLIC_UP258; no network or hardware"]
async fn s2_per_part_a_refused_source_after_a_verified_run_credits_nothing() {
    // Every part was marked before the run, so a wrongly credited "all" cannot hide.
    let after = verified_run_edited_while_paused(true, |state, device| {
        // The App helper refuses a switched product reference: the worker maps it to NONE.
        edit_device(state, device, |d| d.product_ref.push_str("-switched"));
    })
    .await;
    // Memory parts pending; the address is not touched by a download that proves no source.
    assert_eq!(after, [true, false, false, false]);
}

/// A PIR option other than the stored one, read from the real parameter panel.
async fn alternative_pir_value() -> String {
    let Fixture {
        directory: _directory,
        router,
        device,
        ..
    } = fixture().await;
    let panel = http(
        &router,
        &format!("/api/device/{}/parameters", device.0),
        None,
        200,
    )
    .await;
    let field = pir_fields(&panel)[0].clone();
    assert_eq!(field["value"], PIR_RAW);
    field["enumOptions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["value"].as_str().unwrap().to_owned())
        .find(|value| value != PIR_RAW)
        .expect("the PIR field offers another value")
}
