use anyhow::Result;
use std::io::{self, Write};

use super::NotificationPayload;

/// Format an OSC 777 notification escape sequence.
/// Format: `\x1b]777;notify;<summary>;<body>\x1b\\`
/// Supported by Kitty, Alacritty, Foot, WezTerm, etc.
pub fn format_osc777(summary: &str, body: &str) -> String {
    // Sanitize any escape characters or semicolons in title/body to avoid breaking sequence
    let safe_summary = summary.replace('\x1b', "").replace(';', " - ");
    let safe_body = body.replace('\x1b', "").replace(';', " - ");
    format!("\x1b]777;notify;{};{}\x1b\\", safe_summary, safe_body)
}

/// Format an OSC 9 notification escape sequence.
/// Format: `\x1b]9;<summary>: <body>\x1b\\`
/// Supported by iTerm2, Windows Terminal, ConEmu, etc.
pub fn format_osc9(summary: &str, body: &str) -> String {
    let safe_summary = summary.replace('\x1b', "");
    let safe_body = body.replace('\x1b', "");
    if safe_body.is_empty() {
        format!("\x1b]9;{}\x1b\\", safe_summary)
    } else {
        format!("\x1b]9;{}: {}\x1b\\", safe_summary, safe_body)
    }
}

/// Format an OSC 99 notification escape sequence.
/// Format: `\x1b]99;i=1:d=0;<body>\x1b\\`
/// Supported by Kitty and newer terminals.
pub fn format_osc99(summary: &str, body: &str) -> String {
    let safe_summary = summary.replace('\x1b', "");
    let safe_body = body.replace('\x1b', "");
    let content = if safe_body.is_empty() {
        safe_summary
    } else {
        format!("{}: {}", safe_summary, safe_body)
    };
    format!("\x1b]99;i=1:d=0;{}\x1b\\", content)
}

/// Write terminal notification escape sequences to a generic writer.
pub fn write_terminal_notification(
    writer: &mut impl Write,
    payload: &NotificationPayload,
) -> Result<()> {
    let osc777 = format_osc777(&payload.summary, &payload.body);
    let osc9 = format_osc9(&payload.summary, &payload.body);

    writer.write_all(osc777.as_bytes())?;
    writer.write_all(osc9.as_bytes())?;
    writer.flush()?;
    Ok(())
}

/// Send notification escape sequences directly to stdout.
pub fn send_terminal_notification(payload: &NotificationPayload) -> Result<()> {
    let mut stdout = io::stdout().lock();
    write_terminal_notification(&mut stdout, payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_osc777() {
        let osc = format_osc777("F1 - Bahrain GP", "Race starts in 15 minutes!");
        assert_eq!(
            osc,
            "\x1b]777;notify;F1 - Bahrain GP;Race starts in 15 minutes!\x1b\\"
        );
    }

    #[test]
    fn test_format_osc777_sanitizes_escapes() {
        let osc = format_osc777("F1 \x1b[31mRed\x1b[0m", "Alert; please check");
        assert_eq!(
            osc,
            "\x1b]777;notify;F1 [31mRed[0m;Alert -  please check\x1b\\"
        );
    }

    #[test]
    fn test_format_osc9() {
        let osc = format_osc9("WEC - 24h Le Mans", "Qualifying starting soon");
        assert_eq!(
            osc,
            "\x1b]9;WEC - 24h Le Mans: Qualifying starting soon\x1b\\"
        );
    }

    #[test]
    fn test_format_osc99() {
        let osc = format_osc99("MotoGP", "Sprint Race Live");
        assert_eq!(osc, "\x1b]99;i=1:d=0;MotoGP: Sprint Race Live\x1b\\");
    }

    #[test]
    fn test_write_terminal_notification() {
        let mut buf = Vec::new();
        let payload = NotificationPayload::new("NASCAR", "Green flag in 5m");
        write_terminal_notification(&mut buf, &payload).expect("write notification");

        let output = String::from_utf8(buf).expect("utf-8 output");
        assert!(output.contains("\x1b]777;notify;NASCAR;Green flag in 5m\x1b\\"));
        assert!(output.contains("\x1b]9;NASCAR: Green flag in 5m\x1b\\"));
    }
}
