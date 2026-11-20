use std::path::PathBuf;

/// Everything that can go wrong discovering, instantiating or talking to a
/// plugin host process. Crashes at runtime are reported through
/// [`crate::InsertStatus`]/`ClientEvent`, not through this type.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// The `verdant-plughost` binary could not be found or spawned.
    #[error("plughost unavailable: {0}")]
    PlughostUnavailable(String),
    /// The shared-memory ring could not be created or mapped.
    #[error("shared memory: {0}")]
    Shm(String),
    /// A protocol-level problem with the host process.
    #[error("protocol: {0}")]
    Protocol(String),
    /// Invalid prepared automation or an unavailable bounded event transport.
    #[error("automation: {0}")]
    Automation(#[from] crate::proto::AutomationError),
    /// The host process answered a request with an error.
    #[error("host error: {0}")]
    Host(String),
    /// The plugin was not found while restoring a saved insert.
    #[error("plugin missing: {format} {id} ({path})")]
    Missing {
        format: String,
        id: String,
        path: PathBuf,
    },
    /// The plugin is installed in a different location than recorded, or no
    /// longer exists at all.
    #[error("plugin not found in catalog: {0}")]
    NotInCatalog(String),
    /// Timed out waiting for the host process.
    #[error("timeout: {0}")]
    Timeout(String),
    /// I/O failure.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// JSON framing failure on the control channel.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T, E = PluginError> = std::result::Result<T, E>;
