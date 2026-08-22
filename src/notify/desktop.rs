use anyhow::{Context, Result};
use notify_rust::{Notification, Timeout};

use super::NotificationPayload;

/// Send a desktop notification using notify-rust (D-Bus on Linux / native notification center).
pub fn send_desktop_notification(payload: &NotificationPayload) -> Result<()> {
    let mut notif = Notification::new();
    notif
        .appname(&payload.app_name)
        .summary(&payload.summary)
        .body(&payload.body);

    if let Some(ref icon) = payload.icon {
        notif.icon(icon);
    }
    if let Some(ref sound) = payload.sound_name {
        notif.sound_name(sound);
    }
    if let Some(ms) = payload.timeout_ms {
        notif.timeout(Timeout::Milliseconds(ms));
    }

    notif
        .show()
        .context("Failed to display desktop notification via notify-rust")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_notification_builder_syntax() {
        let payload = NotificationPayload::new("F1 - Italian GP", "FP1 starting now")
            .with_app_name("racetui")
            .with_icon("dialog-information")
            .with_sound("message-new-instant")
            .with_timeout_ms(4000);

        assert_eq!(payload.summary, "F1 - Italian GP");
        assert_eq!(payload.body, "FP1 starting now");
        assert_eq!(payload.app_name, "racetui");
        assert_eq!(payload.icon.as_deref(), Some("dialog-information"));
        assert_eq!(payload.sound_name.as_deref(), Some("message-new-instant"));
        assert_eq!(payload.timeout_ms, Some(4000));
    }
}
