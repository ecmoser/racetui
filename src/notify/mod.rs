use anyhow::Result;

pub mod desktop;
pub mod scheduler;
pub mod terminal;

/// Structured notification payload containing the notification content and metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationPayload {
    /// Title / summary of the notification (e.g. "F1 - Bahrain Grand Prix")
    pub summary: String,
    /// Detailed body text (e.g. "Race session starting in 15 minutes!")
    pub body: String,
    /// Name of the application triggering the notification
    pub app_name: String,
    /// Optional icon name or file path
    pub icon: Option<String>,
    /// Optional notification sound name
    pub sound_name: Option<String>,
    /// Display timeout in milliseconds (defaults to 5000ms)
    pub timeout_ms: Option<u32>,
}

impl NotificationPayload {
    /// Create a new notification payload with default app name ("racetui") and 5s timeout.
    pub fn new(summary: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            body: body.into(),
            app_name: "racetui".to_string(),
            icon: None,
            sound_name: None,
            timeout_ms: Some(5000),
        }
    }

    /// Set custom app name.
    pub fn with_app_name(mut self, app_name: impl Into<String>) -> Self {
        self.app_name = app_name.into();
        self
    }

    /// Set notification icon.
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set notification sound name.
    pub fn with_sound(mut self, sound_name: impl Into<String>) -> Self {
        self.sound_name = Some(sound_name.into());
        self
    }

    /// Set notification display timeout in milliseconds.
    pub fn with_timeout_ms(mut self, timeout_ms: u32) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

/// Notification backend selection strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationBackend {
    /// Try desktop notification (D-Bus) first; if it fails (e.g. headless environment), fall back to terminal OSC sequences.
    #[default]
    Auto,
    /// Send desktop notification (D-Bus / notify-rust) only.
    DesktopOnly,
    /// Send terminal escape sequence notification (OSC 777 / OSC 9) only.
    TerminalOnly,
}

/// Send a notification using the specified backend strategy.
/// When using `NotificationBackend::Auto`, if `desktop::send_desktop_notification` fails,
/// it will log a debug message and fall back to `terminal::send_terminal_notification`.
pub fn send_notification(
    payload: &NotificationPayload,
    backend: NotificationBackend,
) -> Result<()> {
    match backend {
        NotificationBackend::Auto => {
            if let Err(e) = desktop::send_desktop_notification(payload) {
                tracing::debug!(
                    "Desktop notification failed ({:?}), falling back to terminal notification",
                    e
                );
                terminal::send_terminal_notification(payload)?;
            }
            Ok(())
        }
        NotificationBackend::DesktopOnly => desktop::send_desktop_notification(payload),
        NotificationBackend::TerminalOnly => terminal::send_terminal_notification(payload),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_payload_defaults() {
        let payload = NotificationPayload::new("IndyCar", "Indy 500 starting");
        assert_eq!(payload.summary, "IndyCar");
        assert_eq!(payload.body, "Indy 500 starting");
        assert_eq!(payload.app_name, "racetui");
        assert_eq!(payload.icon, None);
        assert_eq!(payload.sound_name, None);
        assert_eq!(payload.timeout_ms, Some(5000));
    }

    #[test]
    fn test_notification_payload_builder() {
        let payload = NotificationPayload::new("WEC", "6 Hours of Spa")
            .with_app_name("racetui-daemon")
            .with_icon("alarm")
            .with_sound("bell")
            .with_timeout_ms(10000);

        assert_eq!(payload.app_name, "racetui-daemon");
        assert_eq!(payload.icon.as_deref(), Some("alarm"));
        assert_eq!(payload.sound_name.as_deref(), Some("bell"));
        assert_eq!(payload.timeout_ms, Some(10000));
    }

    #[test]
    fn test_send_notification_terminal_backend() {
        let payload = NotificationPayload::new("WRC", "Rally Finland Stage 1");
        let res = send_notification(&payload, NotificationBackend::TerminalOnly);
        assert!(res.is_ok());
    }

    #[test]
    fn test_send_notification_auto_backend() {
        let payload = NotificationPayload::new("Super Formula", "Suzuka Qualifying");
        // Auto backend will attempt desktop notification and fall back to terminal notification if D-Bus fails
        let res = send_notification(&payload, NotificationBackend::Auto);
        assert!(res.is_ok());
    }
}
