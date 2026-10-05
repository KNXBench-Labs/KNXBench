//! Expose the shared application lifecycle to server adapters without a second journal engine.
pub(crate) use knx_app::commissioning_activity::{
    DownloadGuard, HistoryActivity, OneShotActivity, OneShotLog, WriteOutcome,
};
