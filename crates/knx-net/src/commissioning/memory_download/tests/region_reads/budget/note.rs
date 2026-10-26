//! The connect step's progress note says what the reported budget is, and what it is not.

use super::*;

/// The connect note reports the ordinary-memory budget, which user memory undercuts by one.
#[tokio::test]
async fn the_connect_note_labels_the_budget_as_ordinary_memory_data_per_pdu() {
    let device = device_with_apdu(19);
    let plan = writing(0xFFF0, vec![0xA5; 16]);
    let transport = Encoding(&device);
    let authorisation =
        WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).expect("ok");
    let mut session =
        ManagementSession::authorised(&transport, AuthorisationPlan::Skip, fast(), authorisation)
            .expect("a simulator session");
    let mut notes = Vec::new();
    run_memory_download_with_backup(
        &mut session,
        &plan,
        |progress| {
            if let Progress::StepDone(done) = progress {
                notes.extend(done.observed);
            }
        },
        &mut |_: &DeviceBackup| Ok(()),
    )
    .await
    .expect("download");
    assert!(
        notes.contains(
            &"mask 0701h, manufacturer 0083h, 16 ordinary-memory data octets per PDU".to_string()
        ),
        "{notes:?}"
    );
}
