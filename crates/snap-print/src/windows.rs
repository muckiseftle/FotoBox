//! Windows printing backend.
//!
//! Enumerates printers via PowerShell (WMI) and prints images silently with
//! `mspaint /pt <file> <printer>` (the classic headless image-print path).
//!
//! NOTE: validated only structurally on the dev host — silent printing depends
//! on `mspaint.exe` being present (it ships with Windows 10/11). Verify with the
//! real dye-sub/photo printer; if needed the print command can be swapped later.

use crate::{JobState, PrintError, PrinterInfo, PrinterState};
use std::path::Path;
use tokio::process::Command;

/// Parse the JSON from `Get-CimInstance Win32_Printer | Select Name,Default,WorkOffline`.
pub fn parse_printers(json: &str) -> Vec<PrinterInfo> {
    let value: serde_json::Value =
        serde_json::from_str(json.trim()).unwrap_or(serde_json::Value::Null);
    let items = match value {
        serde_json::Value::Array(a) => a,
        serde_json::Value::Object(_) => vec![value], // a single printer is not wrapped in an array
        _ => Vec::new(),
    };
    items
        .iter()
        .filter_map(|p| {
            let name = p.get("Name")?.as_str()?.to_string();
            let is_default = p.get("Default").and_then(|d| d.as_bool()).unwrap_or(false);
            let offline = p
                .get("WorkOffline")
                .and_then(|d| d.as_bool())
                .unwrap_or(false);
            let (state, message) = if offline {
                (PrinterState::Stopped, "Drucker offline".to_string())
            } else {
                (PrinterState::Idle, "bereit".to_string())
            };
            Some(PrinterInfo {
                name,
                is_default,
                state,
                message,
            })
        })
        .collect()
}

pub async fn list() -> Result<Vec<PrinterInfo>, PrintError> {
    let out = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_Printer | Select-Object Name,Default,WorkOffline | ConvertTo-Json -Compress",
        ])
        .output()
        .await
        .map_err(|e| PrintError::Failed(format!("PowerShell nicht verfügbar: {e}")))?;
    Ok(parse_printers(&String::from_utf8_lossy(&out.stdout)))
}

pub async fn submit(
    printer: &str,
    file: &Path,
    opts: &crate::PrintOptions,
) -> Result<String, PrintError> {
    let path = file.to_string_lossy().to_string();
    for _ in 0..opts.copies.max(1) {
        let status = Command::new("mspaint")
            .arg("/pt")
            .arg(&path)
            .arg(printer)
            .status()
            .await
            .map_err(|e| PrintError::Failed(format!("Druck (mspaint) nicht möglich: {e}")))?;
        if !status.success() {
            return Err(PrintError::Failed("Druckauftrag abgelehnt".to_string()));
        }
    }
    Ok(format!("win-{printer}"))
}

pub async fn job_status(_printer: &str, _job_id: &str) -> Result<JobState, PrintError> {
    // The silent-print path does not expose a job id; treat as done once spooled.
    Ok(JobState::Done)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_array_and_single() {
        let arr = r#"[{"Name":"DNP","Default":true,"WorkOffline":false},{"Name":"PDF","Default":false,"WorkOffline":true}]"#;
        let p = parse_printers(arr);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].name, "DNP");
        assert!(p[0].is_default);
        assert_eq!(p[0].state, PrinterState::Idle);
        assert_eq!(p[1].state, PrinterState::Stopped);

        let single = r#"{"Name":"OnlyOne","Default":true,"WorkOffline":false}"#;
        let p = parse_printers(single);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].name, "OnlyOne");
    }
}
