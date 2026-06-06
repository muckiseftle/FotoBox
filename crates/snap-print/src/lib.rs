//! Printing for SnapStation.
//!
//! Like the camera, printing is abstracted behind a service with two backends:
//! a **CUPS** backend that drives `lp`/`lpstat` (so dye-sub printer drivers and
//! media handling are honoured) and a **Mock** printer that runs on every
//! platform, so the entire print flow — including honest fault reporting — is
//! developable and testable on Windows.
//!
//! CUPS `printer-state-reasons` are mapped to clear German messages
//! ("kein Papier", "Papierstau", …) so failures surface in the UI, never
//! silently.

mod cups;
mod mock;
mod windows;

pub use cups::map_reason;

use serde::Serialize;
use std::path::Path;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PrinterState {
    Idle,
    Printing,
    Stopped,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrinterInfo {
    pub name: String,
    pub is_default: bool,
    pub state: PrinterState,
    /// Human-readable status / fault, e.g. "bereit" or "Drucker meldet: kein Papier".
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
    Queued,
    Printing,
    Done,
    Error,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct PrintOptions {
    pub copies: u32,
    /// CUPS media name, e.g. `om_4x6_4x6in` or `Postcard`. None = printer default.
    pub media: Option<String>,
    pub fit: bool,
}

impl Default for PrintOptions {
    fn default() -> Self {
        Self {
            copies: 1,
            media: None,
            fit: true,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PrintError {
    #[error("kein Drucker verfügbar")]
    NoPrinter,
    #[error("{0}")]
    Failed(String),
}

/// The print service: CUPS on Linux/macOS, a native backend on Windows, or a
/// mock printer (forced via `SNAP_PRINTER=mock`, or where nothing else fits).
pub enum PrintService {
    Cups,
    Windows,
    Mock(Mutex<mock::MockState>),
}

impl PrintService {
    /// Choose the backend. `SNAP_PRINTER=mock` always forces the mock printer.
    pub fn detect() -> Self {
        if std::env::var("SNAP_PRINTER")
            .map(|v| v == "mock")
            .unwrap_or(false)
        {
            tracing::info!("print: using mock printer backend (SNAP_PRINTER=mock)");
            return PrintService::Mock(Mutex::new(mock::MockState::new()));
        }
        Self::detect_native()
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn detect_native() -> Self {
        tracing::info!("print: using CUPS backend");
        PrintService::Cups
    }

    #[cfg(target_os = "windows")]
    fn detect_native() -> Self {
        tracing::info!("print: using Windows backend");
        PrintService::Windows
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    fn detect_native() -> Self {
        tracing::info!("print: using mock printer backend");
        PrintService::Mock(Mutex::new(mock::MockState::new()))
    }

    pub async fn list_printers(&self) -> Result<Vec<PrinterInfo>, PrintError> {
        match self {
            PrintService::Cups => cups::list().await,
            PrintService::Windows => windows::list().await,
            PrintService::Mock(_) => Ok(mock::list()),
        }
    }

    /// Submit a file to a printer, returning a job id.
    pub async fn submit(
        &self,
        printer: &str,
        file: &Path,
        opts: &PrintOptions,
    ) -> Result<String, PrintError> {
        match self {
            PrintService::Cups => cups::submit(printer, file, opts).await,
            PrintService::Windows => windows::submit(printer, file, opts).await,
            PrintService::Mock(state) => {
                let mut guard = state.lock().await;
                mock::submit(&mut guard, printer)
            }
        }
    }

    pub async fn job_status(&self, printer: &str, job_id: &str) -> Result<JobState, PrintError> {
        match self {
            PrintService::Cups => cups::job_status(printer, job_id).await,
            PrintService::Windows => windows::job_status(printer, job_id).await,
            PrintService::Mock(state) => {
                let guard = state.lock().await;
                Ok(mock::job_status(&guard, job_id))
            }
        }
    }
}
