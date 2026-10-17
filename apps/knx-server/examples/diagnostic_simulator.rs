//! Loopback-only server whose tunnel is the regular simulator, for diagnostic acceptance.
//!
//! Explicit software-only acceptance fixture. No real connector is constructed.
//! Run only on loopback, preferably inside an isolated network namespace.
use knx_core::{
    DeviceId, DeviceInstance, IndividualAddress, Installation, InstallationId, Language, Project,
    SourceRef, Topology,
};
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
use knx_net::{
    ApplicationService, BusError, Destination, DiscoveredGateway, ManagementTransport, Tpci,
    TunnelEvent,
};
use knx_server::{BusSessionError, BusTunnel, GatewayConnector};
use std::{future::Future, net::SocketAddrV4, path::PathBuf, pin::Pin, sync::Arc, time::Duration};
use tokio::sync::broadcast;
struct SyntheticTunnel(Arc<SimulatedDevice>);
impl BusTunnel for SyntheticTunnel {
    fn diagnostics_are_simulated(&self) -> bool {
        true
    }
    fn assigned_address(&self) -> IndividualAddress {
        self.0.assigned_address()
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.0.subscribe()
    }
    fn send(
        &self,
        _: Destination,
        _: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send + '_>> {
        panic!("synthetic diagnostics must not send group telegrams")
    }
    fn send_frame(
        &self,
        d: Destination,
        t: Tpci,
        s: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + '_>> {
        Box::pin(async move {
            assert!(
                matches!(
                    s,
                    ApplicationService::NoApplicationPdu
                        | ApplicationService::DeviceDescriptorRead { descriptor_type: 0 }
                        | ApplicationService::PropertyValueRead {
                            object_index: 0,
                            property_id: 12 | 78,
                            nr_of_elem: 1,
                            start_index: 1
                        }
                ),
                "forbidden simulated service"
            );
            tokio::time::sleep(Duration::from_millis(250)).await;
            self.0.send_frame(d, t, s).await
        })
    }
    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send>> {
        Box::pin(async { Ok(()) })
    }
}
struct SyntheticConnector(Arc<SimulatedDevice>);
impl GatewayConnector for SyntheticConnector {
    fn connect_tunnel(
        &self,
        _: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        Box::pin(async { Ok(Box::new(SyntheticTunnel(self.0.clone())) as Box<dyn BusTunnel>) })
    }
    fn discover(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<DiscoveredGateway>, BusSessionError>> + Send + '_>>
    {
        Box::pin(async { Ok(vec![]) })
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listen: SocketAddrV4 = std::env::var("KNX_DIAG_SIM_LISTEN")?.parse()?;
    if !listen.ip().is_loopback() {
        return Err("synthetic fixture must bind loopback".into());
    }
    let data =
        PathBuf::from(std::env::var_os("KNX_DIAG_SIM_DATA").ok_or("KNX_DIAG_SIM_DATA required")?);
    let web =
        PathBuf::from(std::env::var_os("KNX_DIAG_SIM_WEB").ok_or("KNX_DIAG_SIM_WEB required")?);
    let d = Arc::new(SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0701,
        ..Default::default()
    }));
    d.preset_property(0, 12, &[0x12, 0x34]);
    d.preset_property(0, 78, &[1, 2, 3, 4, 5, 6]);
    let state = Arc::new(knx_server::AppState {
        connector: Box::new(SyntheticConnector(d)),
        ..knx_server::AppState::new(data)
    });
    let mut p = Project::new(Language("en".into()));
    p.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: String::new(),
            ets_id: String::new(),
        },
        name: "Synthetic diagnostic device".into(),
        description: None,
        address: Some(IndividualAddress::new(1, 1, 25)?),
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    p.installations.push(Installation {
        id: InstallationId(0),
        name: "Synthetic fixture".into(),
        default_line: None,
        multicast_address: None,
        completion: knx_core::commissioning::CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    *state.project.lock().unwrap() = Some(p);
    let app = knx_server::app(state, Some(web));
    let listener = tokio::net::TcpListener::bind(listen).await?;
    println!("SIMULATED diagnostics only: {listen}; no real gateway connector exists.");
    axum::serve(listener, app).await?;
    Ok(())
}
