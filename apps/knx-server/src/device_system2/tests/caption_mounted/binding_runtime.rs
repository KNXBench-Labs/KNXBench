//! Final binding waits keep the mounted System-2 runtime free without advancing its executor.
use super::*;
use crate::device_system2::{settlement_observation, test_observation};
use knx_app::property_validation::System2Settlement;
use std::sync::mpsc;

#[derive(Default)]
struct Trace {
    connects: AtomicUsize,
    disconnects: AtomicUsize,
    frames: AtomicUsize,
    mutations: AtomicUsize,
}
// Count the existing in-process fake's services, not a socket or physical target.
struct TracedConnector(Arc<Hardware>, Arc<Trace>);
struct TracedTunnel(Box<dyn crate::BusTunnel>, Arc<Trace>);
impl crate::GatewayConnector for TracedConnector {
    #[allow(clippy::type_complexity)]
    fn connect_tunnel(
        &self,
        gateway: SocketAddrV4,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Box<dyn crate::BusTunnel>, crate::BusSessionError>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            let connector = Connector(self.0.clone());
            let tunnel = connector.connect_tunnel(gateway).await?;
            self.1.connects.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(TracedTunnel(tunnel, self.1.clone())) as Box<dyn crate::BusTunnel>)
        })
    }
    #[allow(clippy::type_complexity)]
    fn connect_tunnel_with_ack_wait(
        &self,
        gateway: SocketAddrV4,
        wait: knx_net::TunnelAckWait,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Box<dyn crate::BusTunnel>, crate::BusSessionError>>
                + Send
                + '_,
        >,
    > {
        assert_eq!(wait, knx_net::TunnelAckWait::Patient);
        self.connect_tunnel(gateway)
    }
    #[allow(clippy::type_complexity)]
    fn discover(
        &self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Vec<knx_net::DiscoveredGateway>, crate::BusSessionError>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async { panic!("binding fixture has no network discovery") })
    }
}
impl crate::BusTunnel for TracedTunnel {
    fn diagnostics_are_simulated(&self) -> bool {
        self.0.diagnostics_are_simulated()
    }
    fn assigned_address(&self) -> IndividualAddress {
        self.0.assigned_address()
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.0.subscribe()
    }
    fn send(
        &self,
        destination: Destination,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), crate::BusSessionError>> + Send + '_>> {
        self.0.send(destination, service)
    }
    fn send_frame(
        &self,
        destination: Destination,
        tpci: Tpci,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + '_>> {
        self.1.frames.fetch_add(1, Ordering::SeqCst);
        if matches!(
            service,
            ApplicationService::MemoryWrite { .. }
                | ApplicationService::UserMemoryWrite { .. }
                | ApplicationService::PropertyValueWrite { .. }
        ) {
            self.1.mutations.fetch_add(1, Ordering::SeqCst);
        }
        self.0.send_frame(destination, tpci, service)
    }
    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), crate::BusSessionError>> + Send>> {
        Box::pin(async move {
            self.0.disconnect().await?;
            self.1.disconnects.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
}

#[derive(Clone, Copy, Debug)]
enum End {
    Release,
    Stale,
    Cancel,
    Stop,
}

async fn run(end: End) {
    // Retained synthetic source members: the public prepare AND start handlers,
    // real read-only target observation and opaque current-source admission.
    let input = f::inputs(20, true);
    let data = input.dir.path();
    let mut project = input.project.clone();
    project.devices.get_mut(f::DEVICE).unwrap().address = Some(TARGET.parse().unwrap());
    let hardware = Arc::new(Hardware {
        device: old_device(),
        paused: AtomicBool::new(false),
        reached: Notify::new(),
        release: Notify::new(),
    });
    let old = hardware.device.system2_snapshot();
    drop(input.conn);
    let mut state = state_for(
        data,
        data.join("caption-products.sqlite"),
        project,
        &hardware,
    );
    let trace = Arc::new(Trace::default());
    let observation = Arc::new(test_observation::Observation::default());
    {
        let state = Arc::get_mut(&mut state).unwrap();
        state.connector = Box::new(TracedConnector(hardware.clone(), trace.clone()));
        Arc::get_mut(&mut state.device_system2).unwrap().observation = Some(observation.clone());
    }
    let (supervision, _events) = settlement_observation::Observation::new();
    *state.device_system2.settlement_observer.lock().unwrap() = Some(supervision.clone());
    let router = crate::app(state.clone(), None);
    let preview = prepare(&router, f::DEVICE).await;
    let revision = crate::domain::current_project_revision(&state);
    let (entered, acknowledge) = observation.rendezvous("binding-recheck");
    let (release, wait) = mpsc::channel::<bool>();
    let (ready, held) = tokio::sync::oneshot::channel();
    let backstop = Arc::new(AtomicBool::new(false));
    let expired = backstop.clone();
    let copy = state.clone();
    let holder = std::thread::spawn(move || {
        // Broad lib runs can queue the source/evidence work before this phase.
        // This setup budget is separate from the held-lock liveness backstop.
        entered
            .recv_timeout(Duration::from_secs(180))
            .expect("final binding phase reached");
        let mut guard = copy.project.lock().unwrap();
        acknowledge.send(()).unwrap();
        let _ = ready.send(());
        // Backstop only: even an inline-lock regression releases and joins.
        // No sleep establishes ordering; normal release is a runtime message.
        let command = wait.recv_timeout(Duration::from_secs(3));
        let timed_out = command.is_err();
        expired.store(timed_out, Ordering::SeqCst);
        if matches!(command, Ok(true)) {
            let project = guard.as_mut().unwrap();
            copy.command_stack
                .lock()
                .unwrap()
                .do_command(
                    project,
                    knx_core::Command::RenameDevice {
                        device: f::DEVICE,
                        name: "Changed during final binding wait".into(),
                    },
                )
                .unwrap();
            crate::domain::next_project_revision(&copy);
        }
        (timed_out, guard.as_ref().unwrap().clone())
    });
    let mut ownership = owned::Owned::new(&state, &hardware, &supervision);
    let result = owned::catch(async {
        let started = http(&router, "/api/device-system2/start", Some(start(&preview)), 200).await;
        assert_eq!(started["state"], "running");
        held.await.expect("native holder owns final binding mutex");
        assert!(!backstop.load(Ordering::SeqCst), "runtime blocked at final binding until native backstop");
        // This wake is serviced while the native holder still owns project.
        // An inline mutex blocks this current-thread runtime until its backstop.
        assert!(matches!(state.project.try_lock(), Err(std::sync::TryLockError::WouldBlock)),
            "runtime blocked at final binding: native backstop already released project");
        assert!(!observation.executor_entered(), "executor advanced while project was held");
        assert_eq!(trace.mutations.load(Ordering::SeqCst), 0);
        assert_eq!(trace.connects.load(Ordering::SeqCst), 1);
        assert!(trace.frames.load(Ordering::SeqCst) > 0, "real initial observation reached the fake");
        assert!(state.device_download.try_lock().is_err(), "worker retains tunnel reservation");
        match end {
            End::Cancel => {
                let cancelled = http(&router, "/api/device-system2/cancel", Some(json!({"operationId":started["operationId"]})), 200).await;
                assert_eq!(cancelled["state"], "running", "cooperative cancel is not a finished task");
            }
            End::Stop => {
                crate::close_process_admission(&state);
                assert!(!state.device_system2.control.snapshot().accepting);
                assert!(state.stop_admission.admit().is_err());
            }
            _ => {}
        }
        let frames_before_release = trace.frames.load(Ordering::SeqCst);
        release.send(matches!(end, End::Stale)).unwrap();
        let expected = match end { End::Release => "finished", End::Stale => "interrupted", End::Cancel | End::Stop => "cancelled" };
        // Completion after lock release includes the real journal's disk work;
        // use the existing load-tolerant helper, not the held-lock backstop.
        let status = if matches!(end, End::Release) {
            super::super::failure_diagnostic::settled_slow_status(&state, &router, expected).await
        } else {
            settled(&state, &router, expected).await
        };
        assert_eq!(status["refusal"], Value::Null, "existing journal-derived terminal publication");
        if matches!(end, End::Release) {
            assert_eq!(status["downloadResult"], "verified");
            assert_eq!(status["backupAvailable"], true);
            assert!(trace.mutations.load(Ordering::SeqCst) > 0);
            assert_eq!(flags(&state)[..4], [true; 4]);
            assert!(observation.executor_entered());
            assert_eq!(observation.first_failure(), None);
        } else {
            assert_eq!(status["downloadResult"], "nothingWritten");
            assert_eq!(status["mutationMayHaveStarted"], false);
            assert_eq!(status["backupAvailable"], false);
            assert_eq!(status["projectSettlement"], "noMutation");
            assert_eq!(trace.mutations.load(Ordering::SeqCst), 0, "no mutating service after refusal/cancellation");
            assert_eq!(trace.frames.load(Ordering::SeqCst), frames_before_release,
                "no service at all after stale binding or pending cooperative cancellation");
            assert_eq!(hardware.device.system2_snapshot(), old);
            assert_eq!(flags(&state)[..4], [false; 4], "no false programming credit");
            if matches!(end, End::Cancel | End::Stop) {
                // The retained worker remains authoritative. The existing App entry
                // checkpoints Cancelled before opening another management connection;
                // cooperative cancel is not an aborted/dropped async worker.
                assert!(observation.executor_entered());
                assert_eq!(observation.first_failure(), Some(("executor", "io")));
            }
            if matches!(end, End::Stale) {
                assert_eq!(observation.first_failure(), Some(("binding-recheck", "binding-stale")));
                assert!(!observation.executor_entered(), "stale binding must refuse before executor");
                assert_eq!(crate::domain::current_project_revision(&state), revision + 1);
            }
        }
        assert_eq!(trace.disconnects.load(Ordering::SeqCst), 1);
        let expected = match end { End::Release => LocalOutcome::Finished, End::Stale => LocalOutcome::Interrupted, End::Cancel | End::Stop => LocalOutcome::Cancelled };
        assert!(matches!(state.device_system2.control.snapshot().settlement, System2Settlement::Settled { outcome, .. } if outcome == expected));
        assert!(state.device_download.try_lock().is_ok());
        assert!(state.address_programming.try_lock().is_ok());
        // Cross-process ownership also ends; no actual gateway is opened.
        let lease = knx_app::device_diagnostics::lease_in(state.diagnostic_lease_dir.as_ref().unwrap()).unwrap();
        drop(lease);
    }).await;
    // Release and join even on a failed assertion, including the baseline replay.
    let _ = release.send(false);
    let joined = tokio::task::spawn_blocking(move || holder.join())
        .await
        .unwrap();
    let cleanup = ownership.finish(result.is_err()).await;
    owned::propagate(result, cleanup);
    assert!(
        ownership.service_joined,
        "retained supervisor and actual child joined"
    );
    assert_eq!(supervision.worker_finished(), Some(true));
    let (timed_out, expected_project) = joined.unwrap();
    assert!(
        !timed_out,
        "runtime blocked at final binding until native backstop"
    );
    if !matches!(end, End::Release) {
        assert_eq!(
            state.project.lock().unwrap().as_ref().unwrap(),
            &expected_project,
            "non-mutating termination preserves the complete current project"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn final_binding_wait_leaves_runtime_free_and_release_finishes() {
    run(End::Release).await;
}
#[tokio::test(flavor = "current_thread")]
async fn final_binding_wait_samples_changed_project_under_guard() {
    run(End::Stale).await;
}
#[tokio::test(flavor = "current_thread")]
async fn final_binding_wait_honours_accepted_cancel_before_any_write() {
    run(End::Cancel).await;
}
#[tokio::test(flavor = "current_thread")]
async fn final_binding_wait_honours_process_stop_before_any_write() {
    run(End::Stop).await;
}
