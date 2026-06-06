//! Shared domain types, configuration, database access and errors for SnapStation.
//!
//! Design rule: *infrastructure* configuration (paths, bind address) comes from
//! the environment via [`Config`]; *user-facing* settings live in the database
//! [`settings`] table and are edited only through the web interface.

pub mod config;
pub mod db;
pub mod error;
pub mod ids;
pub mod models;
pub mod settings;

pub use config::Config;
pub use db::Db;
pub use error::{Error, Result};

/// SnapStation version string, surfaced by the status API.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
