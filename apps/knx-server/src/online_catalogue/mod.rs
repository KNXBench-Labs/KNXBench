//! Online product-catalogue sources: manual index sync, local search and credentialed download.

mod body;
mod download;
mod error;
mod index;
mod routes;
mod service;
mod source;
mod store;
mod transport;
mod vault;

#[cfg(test)]
mod test_server;
#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::Arc;

pub use error::OnlineError;
pub use routes::router;
pub use service::SourceService;

use transport::HttpsClient;

/// The configuration subdirectory name used when the server keeps its key inside the data volume;
/// no file route may reach it (`paths.rs`).
pub const CONFIG_DIR_NAME: &str = ".knxbench-config";
/// Private provider state must not be exposed by the project-file API.
pub const DATA_DIR_NAME: &str = ".knxbench-online-catalogue";

/// The sources this build offers, each with its own folder, account file and key file.
pub struct OnlineCatalogue {
    sources: Vec<Arc<SourceService>>,
}

impl OnlineCatalogue {
    /// The production catalogue: Mozilla trust roots, `data_dir` for state, `config_dir` for keys.
    pub fn open(data_dir: PathBuf, config_dir: PathBuf) -> Self {
        Self::with_client(data_dir, config_dir, Arc::new(HttpsClient::production()))
    }

    pub(crate) fn with_client(
        data_dir: PathBuf,
        config_dir: PathBuf,
        client: Arc<HttpsClient>,
    ) -> Self {
        let sources = source::SOURCES
            .iter()
            .map(|spec| {
                Arc::new(SourceService::open(
                    spec,
                    data_dir.clone(),
                    config_dir.join(format!("online-catalogue-{}.key", spec.id)),
                    Arc::clone(&client),
                ))
            })
            .collect();
        Self { sources }
    }

    /// The catalogue for this process's user directories, or `None` when none can be located.
    pub fn from_environment() -> Option<Self> {
        let data = knx_productdb::default_path()?.parent()?.join(DATA_DIR_NAME);
        let config = knx_app::legacy::default_remembered_password_path()?
            .parent()?
            .join(CONFIG_DIR_NAME);
        Some(Self::open(data, config))
    }

    pub fn source(&self, id: &str) -> Result<&Arc<SourceService>, OnlineError> {
        self.sources
            .iter()
            .find(|s| s.spec.id == id)
            .ok_or(OnlineError::UnknownSource)
    }

    pub fn sources(&self) -> &[Arc<SourceService>] {
        &self.sources
    }
}
