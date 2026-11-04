//! Adapt validated Linux TP-UART configuration to the shared server bus tunnel seam.

use knx_core::IndividualAddress;
use knx_net::{ApplicationService, BusError, Destination, Tpci, TpuartClient, TpuartMode};
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin};
use tokio::sync::broadcast;

use crate::bus::{BusSessionError, BusTunnel, TunnelEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SerialMode {
    Passive,
    Active,
}

/// An explicit server-local target; never derived from an IP gateway string.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SerialTarget {
    pub path: String,
    pub source: String,
    pub mode: SerialMode,
}

impl SerialTarget {
    /// Pure validation before connector admission; uniqueness is the caller's responsibility.
    pub fn validate(&self) -> Result<IndividualAddress, String> {
        if self.path.trim().is_empty()
            || self.path != self.path.trim()
            || self.path.chars().any(char::is_control)
        {
            return Err("serial path must be explicit, nonempty and contain no control characters or surrounding whitespace".into());
        }
        let source = self
            .source
            .parse::<IndividualAddress>()
            .map_err(|_| "serial source must be area.line.device".to_string())?;
        if source.to_string() != self.source {
            return Err("serial source must use canonical area.line.device notation".into());
        }
        TpuartClient::validate_source(source).map_err(|error| error.to_string())?;
        Ok(source)
    }
}

pub(crate) async fn open(target: &SerialTarget) -> Result<Box<dyn BusTunnel>, BusSessionError> {
    let source = target
        .validate()
        .map_err(|error| BusSessionError::Transport(BusError::Protocol(error)))?;
    let mode = match target.mode {
        SerialMode::Passive => TpuartMode::BusMonitor,
        SerialMode::Active => TpuartMode::Active,
    };
    let client = TpuartClient::open(&target.path, source, mode)
        .await
        .map_err(BusSessionError::Transport)?;
    Ok(Box::new(SerialTunnel(client)))
}

struct SerialTunnel(TpuartClient);
impl BusTunnel for SerialTunnel {
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
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send + '_>> {
        Box::pin(async move {
            self.0
                .send(destination, service)
                .await
                .map_err(BusSessionError::Transport)
        })
    }
    fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + '_>> {
        Box::pin(async move { self.0.send_frame(destination, transport, service).await })
    }
    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send>> {
        Box::pin(async move {
            self.0
                .disconnect()
                .await
                .map_err(BusSessionError::Transport)
        })
    }
}
