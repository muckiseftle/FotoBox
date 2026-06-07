//! Shared application state handed to every Axum handler.

use snap_camera::CameraHandle;
use snap_core::{Config, Db};
use snap_print::PrintService;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long an admin session stays valid after login.
pub const SESSION_TTL: Duration = Duration::from_secs(24 * 60 * 60);
/// Failed logins from one IP before it is temporarily locked out.
const MAX_LOGIN_FAILS: u32 = 5;
/// Lockout duration once the failure threshold is hit.
const LOGIN_LOCKOUT: Duration = Duration::from_secs(30);

#[derive(Default)]
struct LoginGate {
    fails: u32,
    locked_until: Option<Instant>,
}

/// In-memory admin session + brute-force tracking (cleared on restart).
#[derive(Default)]
pub struct SessionStore {
    /// Active session tokens → expiry instant.
    tokens: HashMap<String, Instant>,
    /// Per-IP failed-login tracking for brute-force throttling.
    login_gates: HashMap<IpAddr, LoginGate>,
}

impl SessionStore {
    /// Register a freshly issued session token.
    pub fn insert(&mut self, token: String) {
        self.prune();
        self.tokens.insert(token, Instant::now() + SESSION_TTL);
    }

    /// Whether a token is present and not expired.
    pub fn is_valid(&mut self, token: &str) -> bool {
        match self.tokens.get(token) {
            Some(&exp) if exp > Instant::now() => true,
            Some(_) => {
                self.tokens.remove(token);
                false
            }
            None => false,
        }
    }

    /// Invalidate a session (logout).
    pub fn remove(&mut self, token: &str) {
        self.tokens.remove(token);
    }

    /// Drop expired sessions so the map cannot grow without bound.
    fn prune(&mut self) {
        let now = Instant::now();
        self.tokens.retain(|_, &mut exp| exp > now);
        self.login_gates
            .retain(|_, g| g.fails > 0 || g.locked_until.is_some_and(|u| u > now));
    }

    /// Seconds an IP must wait before another login attempt (None = allowed now).
    pub fn login_retry_after(&self, ip: IpAddr) -> Option<u64> {
        self.login_gates.get(&ip).and_then(|g| {
            g.locked_until
                .filter(|&u| u > Instant::now())
                .map(|u| (u - Instant::now()).as_secs() + 1)
        })
    }

    /// Record a failed login; locks the IP out once the threshold is reached.
    pub fn note_login_failure(&mut self, ip: IpAddr) {
        let g = self.login_gates.entry(ip).or_default();
        g.fails += 1;
        if g.fails >= MAX_LOGIN_FAILS {
            g.fails = 0;
            g.locked_until = Some(Instant::now() + LOGIN_LOCKOUT);
        }
    }

    /// Clear failure tracking for an IP after a successful login.
    pub fn note_login_success(&mut self, ip: IpAddr) {
        self.login_gates.remove(&ip);
    }
}

/// Shared admin session store.
pub type Sessions = Arc<Mutex<SessionStore>>;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub camera: CameraHandle,
    pub printer: Arc<PrintService>,
    pub config: Arc<Config>,
    pub sessions: Sessions,
}
