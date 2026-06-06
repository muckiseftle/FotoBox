//! A virtual printer for development and tests.
//!
//! Exposes one printer and accepts jobs. A simulated fault can be injected via
//! `SNAP_PRINTER_FAULT=<cups-reason>` (e.g. `media-empty`) to exercise the
//! honest error-reporting UI without real hardware.

use crate::{JobState, PrintError, PrinterInfo, PrinterState};
use std::collections::HashMap;

pub struct MockState {
    counter: u32,
    jobs: HashMap<String, JobState>,
}

impl MockState {
    pub fn new() -> Self {
        Self {
            counter: 0,
            jobs: HashMap::new(),
        }
    }
}

impl Default for MockState {
    fn default() -> Self {
        Self::new()
    }
}

fn fault() -> Option<String> {
    std::env::var("SNAP_PRINTER_FAULT")
        .ok()
        .filter(|v| !v.is_empty())
}

const MOCK_NAME: &str = "Mock_Fotodrucker_DNP_DS-RX1";

pub fn list() -> Vec<PrinterInfo> {
    match fault() {
        Some(reason) => {
            let message = crate::map_reason(&reason).unwrap_or_else(|| "angehalten".to_string());
            vec![PrinterInfo {
                name: MOCK_NAME.to_string(),
                is_default: true,
                state: PrinterState::Stopped,
                message,
            }]
        }
        None => vec![PrinterInfo {
            name: MOCK_NAME.to_string(),
            is_default: true,
            state: PrinterState::Idle,
            message: "bereit".to_string(),
        }],
    }
}

pub fn submit(state: &mut MockState, _printer: &str) -> Result<String, PrintError> {
    if let Some(reason) = fault() {
        let message = crate::map_reason(&reason).unwrap_or_else(|| "angehalten".to_string());
        return Err(PrintError::Failed(format!("Drucker meldet: {message}")));
    }
    state.counter += 1;
    let job_id = format!("{MOCK_NAME}-{}", state.counter);
    state.jobs.insert(job_id.clone(), JobState::Done);
    Ok(job_id)
}

pub fn job_status(state: &MockState, job_id: &str) -> JobState {
    state.jobs.get(job_id).copied().unwrap_or(JobState::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_and_status_without_fault() {
        // Ensure no fault is set for this test.
        std::env::remove_var("SNAP_PRINTER_FAULT");
        let mut s = MockState::new();
        let printers = list();
        assert_eq!(printers[0].state, PrinterState::Idle);
        let job = submit(&mut s, MOCK_NAME).unwrap();
        assert_eq!(job_status(&s, &job), JobState::Done);
    }
}
