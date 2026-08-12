//! HTTP server implementation for bookmarklet
//!
//! Runs a simple HTTP server on localhost to receive job data from browser bookmarklets.

use super::pending::{
    new_pending_bookmarklet_imports, pending_bookmarklet_import_previews,
    PendingBookmarkletImportPreview, PendingBookmarkletImports,
};
use super::{BookmarkletRepository, CompanionPairing};
use chrono::Utc;
#[cfg(test)]
use jobsentinel_domain::calculate_job_hash;
use jobsentinel_security::validate_credential_free_external_https_url;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;

mod imports;
mod listener;
mod pairing_state;

use imports::handle_import_request;
pub use imports::{confirm_pending_bookmarklet_imports, discard_pending_bookmarklet_imports};
use listener::bind_bookmarklet_listener;
use pairing_state::*;

const CONTENT_LENGTH_HEADER: &str = "content-length";
const HEADER_BODY_SEPARATOR: &[u8] = b"\r\n\r\n";
const MAX_BOOKMARKLET_REQUEST_BYTES: usize = 24 * 1024;
const MAX_BOOKMARKLET_CONNECTIONS: usize = 16;
const MAX_BOOKMARKLET_JOBS_PER_REQUEST: usize = 12;
#[cfg(not(test))]
const BOOKMARKLET_READ_TIMEOUT: Duration = Duration::from_secs(3);
#[cfg(test)]
const BOOKMARKLET_READ_TIMEOUT: Duration = Duration::from_millis(50);
const MAX_BOOKMARKLET_TITLE_LENGTH: usize = 500;
const MAX_BOOKMARKLET_COMPANY_LENGTH: usize = 200;
const MAX_BOOKMARKLET_URL_LENGTH: usize = 2000;
const MAX_BOOKMARKLET_LOCATION_LENGTH: usize = 200;
const MAX_BOOKMARKLET_DESCRIPTION_LENGTH: usize = 50000;
const INVALID_BOOKMARKLET_PAYLOAD_MESSAGE: &str =
    "Invalid bookmarklet payload. Reload the page and try again.";
const BOOKMARKLET_DATABASE_FAILURE_MESSAGE: &str =
    "JobSentinel could not save this job. Restart the app and try again.";
const BOOKMARKLET_UNAUTHORIZED_MESSAGE: &str =
    "Browser import code expired. Copy the browser button again and retry.";
const BOOKMARKLET_AUTHORIZATION_FAILURE_MESSAGE: &str =
    "Browser Import authorization changed. Copy a fresh browser button and try again.";

/// Bookmarklet server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkletConfig {
    pub port: u16,
}

impl Default for BookmarkletConfig {
    fn default() -> Self {
        Self { port: 4321 }
    }
}

/// Bookmarklet server errors
#[derive(Debug, Error)]
pub enum BookmarkletError {
    #[error("Server is already running")]
    AlreadyRunning,

    #[error("Server is not running")]
    NotRunning,

    #[error("Failed to bind to port {port}: {source}")]
    BindError { port: u16, source: std::io::Error },

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Invalid job data: {0}")]
    InvalidData(String),
}

/// HTTP server for receiving bookmarklet data
pub struct BookmarkletServer {
    config: BookmarkletConfig,
    active_pairing: ActiveCompanionPairing,
    pending_imports: PendingBookmarkletImports,
    server_handle: Option<JoinHandle<()>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl BookmarkletServer {
    /// Create a new bookmarklet server
    pub fn new(config: BookmarkletConfig) -> Self {
        Self {
            config,
            active_pairing: new_active_pairing(),
            pending_imports: new_pending_bookmarklet_imports(),
            server_handle: None,
            shutdown_tx: None,
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &BookmarkletConfig {
        &self.config
    }

    /// Set new configuration (only when server is stopped)
    pub fn set_config(&mut self, config: BookmarkletConfig) {
        self.config = config;
        revoke_active_pairing(&self.active_pairing);
    }

    /// Replace the only active local browser pairing.
    pub fn replace_pairing(&mut self, pairing: CompanionPairing) -> Result<(), BookmarkletError> {
        if !self.is_running() {
            return Err(BookmarkletError::NotRunning);
        }
        replace_active_pairing(&self.active_pairing, pairing);
        Ok(())
    }

    /// Report whether a usable browser pairing is active.
    pub fn pairing_is_current(&self) -> bool {
        active_pairing_is_current(&self.active_pairing, Utc::now())
    }

    /// Clone the in-memory review queue for command handlers.
    pub fn pending_import_store(&self) -> PendingBookmarkletImports {
        self.pending_imports.clone()
    }

    /// List imports waiting for explicit user confirmation.
    pub fn pending_imports(&self) -> Vec<PendingBookmarkletImportPreview> {
        pending_bookmarklet_import_previews(&self.pending_imports)
    }

    /// Check if server is running
    pub fn is_running(&self) -> bool {
        self.server_handle.is_some()
    }

    /// Start the HTTP server
    pub async fn start(
        &mut self,
        config: BookmarkletConfig,
        repository: Arc<dyn BookmarkletRepository>,
    ) -> Result<u16, BookmarkletError> {
        if self.is_running() {
            return Err(BookmarkletError::AlreadyRunning);
        }

        self.config = config;
        let requested_port = self.config.port;
        let (listener, port) = bind_bookmarklet_listener(requested_port).await?;
        self.config.port = port;
        revoke_active_pairing(&self.active_pairing);
        let active_pairing = self.active_pairing.clone();
        let pending_imports = self.pending_imports.clone();

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        // Spawn server task
        let handle = tokio::spawn(async move {
            if let Err(e) = run_server(
                listener,
                active_pairing,
                pending_imports,
                repository,
                shutdown_rx,
            )
            .await
            {
                tracing::error!(error = %bookmarklet_error_label(&e), "Bookmarklet server error");
            }
        });

        self.server_handle = Some(handle);
        self.shutdown_tx = Some(shutdown_tx);

        tracing::info!(port = port, "Bookmarklet server started");
        Ok(port)
    }

    /// Stop the HTTP server
    pub async fn stop(&mut self) -> Result<(), BookmarkletError> {
        if !self.is_running() {
            return Err(BookmarkletError::NotRunning);
        }

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Wait for server to stop
        if let Some(handle) = self.server_handle.take() {
            let _ = handle.await;
        }
        revoke_active_pairing(&self.active_pairing);

        tracing::info!("Bookmarklet server stopped");
        Ok(())
    }
}

impl Default for BookmarkletServer {
    fn default() -> Self {
        Self::new(BookmarkletConfig::default())
    }
}

async fn run_server(
    listener: tokio::net::TcpListener,
    active_pairing: ActiveCompanionPairing,
    pending_imports: PendingBookmarkletImports,
    repository: Arc<dyn BookmarkletRepository>,
    mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<(), BookmarkletError> {
    if let Ok(address) = listener.local_addr() {
        tracing::info!(address = %address, "Listening on bookmarklet import port");
    }

    let connection_limit = Arc::new(Semaphore::new(MAX_BOOKMARKLET_CONNECTIONS));

    // Accept connections until shutdown
    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let Some(connection_permit) =
                            try_bookmarklet_connection_permit(&connection_limit)
                        else {
                            tracing::warn!("Bookmarklet connection limit reached");
                            continue;
                        };
                        let repository = repository.clone();
                        let pairing = active_pairing.clone();
                        let pending = pending_imports.clone();
                        tokio::spawn(async move {
                            let _connection_permit = connection_permit;
                            if let Err(_e) =
                                handle_connection(stream, pairing, pending, repository).await
                            {
                                tracing::error!("Bookmarklet connection failed");
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!(
                            error_kind = ?e.kind(),
                            "Bookmarklet connection accept failed"
                        );
                    }
                }
            }
            _ = &mut shutdown_rx => {
                tracing::info!("Shutdown signal received");
                break;
            }
        }
    }

    Ok(())
}

fn try_bookmarklet_connection_permit(
    connection_limit: &Arc<Semaphore>,
) -> Option<OwnedSemaphorePermit> {
    Arc::clone(connection_limit).try_acquire_owned().ok()
}

/// Handle a single HTTP connection
async fn handle_connection(
    stream: tokio::net::TcpStream,
    active_pairing: ActiveCompanionPairing,
    pending_imports: PendingBookmarkletImports,
    repository: Arc<dyn BookmarkletRepository>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::AsyncWriteExt;

    let request = read_bookmarklet_request(&stream).await?;
    let local_addr = stream.local_addr()?;

    // Parse request
    let (response, content_type) = if !has_valid_bookmarklet_host(&request, local_addr.port()) {
        (
            json_error_response("Invalid browser import host"),
            "application/json".to_string(),
        )
    } else if !has_allowed_bookmarklet_origin(&request) {
        (
            json_error_response("Invalid browser import origin"),
            "application/json".to_string(),
        )
    } else if is_bookmarklet_import_request(&request) {
        handle_import_request(&request, &active_pairing, pending_imports, repository).await
    } else if request.starts_with("OPTIONS") {
        ("OK".to_string(), "text/plain".to_string())
    } else {
        (
            json_error_response("Not found"),
            "application/json".to_string(),
        )
    };

    // Send response with CORS headers
    let status = if response.starts_with("{\"error\"") {
        "400 Bad Request"
    } else {
        "200 OK"
    };

    let response_data = http_response_data(status, &content_type, &response);

    let mut writable_stream = stream;
    writable_stream.write_all(response_data.as_bytes()).await?;
    writable_stream.flush().await?;

    Ok(())
}

async fn read_bookmarklet_request(
    stream: &tokio::net::TcpStream,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let bytes = tokio::time::timeout(
        BOOKMARKLET_READ_TIMEOUT,
        read_bookmarklet_request_bytes(stream),
    )
    .await
    .map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "Bookmarklet request timed out",
        )
    })??;

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

async fn read_bookmarklet_request_bytes(
    stream: &tokio::net::TcpStream,
) -> std::io::Result<Vec<u8>> {
    let mut buffer = vec![0u8; MAX_BOOKMARKLET_REQUEST_BYTES];
    let mut total_read = 0;

    loop {
        match stream.try_read(&mut buffer[total_read..]) {
            Ok(0) => break,
            Ok(n) => {
                total_read += n;
                if total_read >= buffer.len() {
                    break;
                }
                if request_buffer_should_stop_reading(&buffer[..total_read], buffer.len()) {
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(e) => return Err(e),
        }
    }

    buffer.truncate(total_read);
    Ok(buffer)
}

mod protocol;

use protocol::*;

#[cfg(test)]
mod tests;
