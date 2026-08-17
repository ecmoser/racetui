use anyhow::{Context, Result};
use std::process::Command;

/// Open a URL using the configured open command.
/// The command is run in the background (detached) so it doesn't block the TUI.
pub fn open_url(open_command: &str, url: &str) -> Result<()> {
    Command::new(open_command)
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .with_context(|| {
            format!(
                "Failed to open URL '{}' with command '{}'",
                url, open_command
            )
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_url_with_echo() {
        // Using "echo" or "true" which exists on unix systems
        let res = open_url("true", "https://example.com");
        assert!(res.is_ok());
    }

    #[test]
    fn test_open_url_nonexistent_command() {
        let res = open_url("nonexistent_command_12345", "https://example.com");
        assert!(res.is_err());
    }
}
