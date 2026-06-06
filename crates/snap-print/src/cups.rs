//! CUPS backend driving the `lp`/`lpstat` command-line tools.
//!
//! These calls compile on every platform (the tools just won't exist off
//! Linux), so the pure parsing/mapping logic below is unit-tested here. The
//! exact `lpstat` output parsing should still be validated on real Ubuntu +
//! dye-sub hardware (project plan, Phase 3).

use crate::{JobState, PrintError, PrinterInfo, PrinterState};
use std::path::Path;
use tokio::process::Command;

/// Map a CUPS `printer-state-reason` token to a clear German message.
/// The trailing severity (`-error`/`-warning`/`-report`) is stripped first.
pub fn map_reason(reason: &str) -> Option<String> {
    let base = reason
        .trim()
        .trim_end_matches("-error")
        .trim_end_matches("-warning")
        .trim_end_matches("-report");
    let msg = match base {
        "none" | "" => return None,
        "media-empty" | "media-needed" => "kein Papier",
        "media-low" => "Papier fast leer",
        "media-jam" => "Papierstau",
        "cover-open" | "door-open" => "Abdeckung offen",
        "marker-supply-empty" | "toner-empty" => "Farbband/Toner leer",
        "marker-supply-low" | "toner-low" => "Farbband/Toner fast leer",
        "input-tray-missing" => "Papierfach fehlt",
        "output-area-full" | "output-tray-missing" => "Ausgabefach voll/fehlt",
        "offline" | "connecting-to-device" | "shutdown" => "Drucker offline",
        "paused" | "moving-to-paused" => "Drucker pausiert",
        "spool-area-full" => "Warteschlange voll",
        "timed-out" => "Drucker antwortet nicht",
        other => return Some(format!("Drucker meldet: {other}")),
    };
    Some(msg.to_string())
}

/// Parse `lpstat -p -d` output into printer infos.
pub fn parse_printers(stdout: &str, default: Option<&str>) -> Vec<PrinterInfo> {
    let mut printers = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("printer ") else {
            continue;
        };
        let mut parts = rest.splitn(2, ' ');
        let Some(name) = parts.next() else { continue };
        let tail = parts.next().unwrap_or("");

        let (state, message) = if tail.contains("now printing") {
            (PrinterState::Printing, "druckt …".to_string())
        } else if tail.contains("is idle") {
            (PrinterState::Idle, "bereit".to_string())
        } else if tail.contains("disabled") {
            // A reason sometimes follows " - "; map it if recognised.
            let reason = tail.split(" - ").nth(1).unwrap_or("").trim();
            let msg = map_reason(reason).unwrap_or_else(|| "angehalten".to_string());
            (PrinterState::Stopped, msg)
        } else {
            (PrinterState::Unknown, "unbekannt".to_string())
        };

        printers.push(PrinterInfo {
            is_default: default == Some(name),
            name: name.to_string(),
            state,
            message,
        });
    }
    printers
}

/// Extract the request id from `lp` output ("request id is PRINTER-42 (1 file)").
pub fn parse_request_id(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix("request id is "))
        .map(|s| s.split_whitespace().next().unwrap_or(s).to_string())
}

/// Parse the system default destination from `lpstat -d`.
fn parse_default(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|l| {
        l.trim()
            .strip_prefix("system default destination: ")
            .map(|s| s.trim().to_string())
    })
}

pub async fn list() -> Result<Vec<PrinterInfo>, PrintError> {
    let out = Command::new("lpstat")
        .args(["-p", "-d"])
        .output()
        .await
        .map_err(|e| PrintError::Failed(format!("lpstat nicht verfügbar: {e}")))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let default = parse_default(&stdout);
    Ok(parse_printers(&stdout, default.as_deref()))
}

pub async fn submit(
    printer: &str,
    file: &Path,
    opts: &crate::PrintOptions,
) -> Result<String, PrintError> {
    let mut cmd = Command::new("lp");
    cmd.arg("-d").arg(printer);
    cmd.arg("-n").arg(opts.copies.max(1).to_string());
    if let Some(media) = &opts.media {
        cmd.arg("-o").arg(format!("media={media}"));
    }
    if opts.fit {
        cmd.arg("-o").arg("fit-to-page");
    }
    cmd.arg(file);

    let out = cmd
        .output()
        .await
        .map_err(|e| PrintError::Failed(format!("lp nicht verfügbar: {e}")))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(PrintError::Failed(format!(
            "Druckauftrag abgelehnt: {}",
            err.trim()
        )));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    parse_request_id(&stdout)
        .ok_or_else(|| PrintError::Failed("Auftrags-ID nicht erkannt".to_string()))
}

pub async fn job_status(_printer: &str, job_id: &str) -> Result<JobState, PrintError> {
    // Active jobs appear in `lpstat -o`; absence means it completed.
    let out = Command::new("lpstat")
        .arg("-o")
        .output()
        .await
        .map_err(|e| PrintError::Failed(format!("lpstat nicht verfügbar: {e}")))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let active = stdout
        .lines()
        .any(|l| l.split_whitespace().next() == Some(job_id));
    Ok(if active {
        JobState::Printing
    } else {
        JobState::Done
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_reasons() {
        assert_eq!(
            map_reason("media-empty-error").as_deref(),
            Some("kein Papier")
        );
        assert_eq!(
            map_reason("media-jam-warning").as_deref(),
            Some("Papierstau")
        );
        assert_eq!(map_reason("cover-open").as_deref(), Some("Abdeckung offen"));
        assert_eq!(
            map_reason("marker-supply-low-report").as_deref(),
            Some("Farbband/Toner fast leer")
        );
        assert_eq!(map_reason("none"), None);
        assert!(map_reason("weird-vendor-thing")
            .unwrap()
            .contains("weird-vendor-thing"));
    }

    #[test]
    fn parses_printer_states_and_default() {
        let out = "printer DNP_DS620 is idle.  enabled since Mon\n\
                   printer Selphy now printing Selphy-7.  enabled since Tue\n\
                   printer OldOne disabled since Wed - media-empty-error\n\
                   system default destination: DNP_DS620\n";
        let printers = parse_printers(out, Some("DNP_DS620"));
        assert_eq!(printers.len(), 3);
        assert_eq!(printers[0].name, "DNP_DS620");
        assert_eq!(printers[0].state, PrinterState::Idle);
        assert!(printers[0].is_default);
        assert_eq!(printers[1].state, PrinterState::Printing);
        assert_eq!(printers[2].state, PrinterState::Stopped);
        assert_eq!(printers[2].message, "kein Papier");
    }

    #[test]
    fn parses_request_id() {
        assert_eq!(
            parse_request_id("request id is DNP_DS620-42 (1 file(s))").as_deref(),
            Some("DNP_DS620-42")
        );
        assert_eq!(parse_request_id("nope"), None);
    }
}
