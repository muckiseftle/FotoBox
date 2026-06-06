//! Shared application state handed to every Axum handler.

use snap_camera::CameraHandle;
use snap_core::{Config, Db};
use snap_print::PrintService;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Set of valid admin session tokens (in-memory; cleared on restart).
pub type Sessions = Arc<Mutex<HashSet<String>>>;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub camera: CameraHandle,
    pub printer: Arc<PrintService>,
    pub config: Arc<Config>,
    pub sessions: Sessions,
}
