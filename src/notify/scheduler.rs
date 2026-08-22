use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

use super::{send_notification, NotificationBackend, NotificationPayload};
use crate::config::settings::UserConfig;
use crate::data::models::{RaceEvent, Series};

/// Keeps track of sessions that have already triggered notifications
/// to prevent duplicate alerts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotificationTracker {
    notified_keys: HashSet<String>,
}

impl NotificationTracker {
    /// Create a new empty NotificationTracker.
    pub fn new() -> Self {
        Self {
            notified_keys: HashSet::new(),
        }
    }

    /// Generate a unique cache key for a session.
    pub fn session_key(
        series_id: &str,
        round: Option<u32>,
        session_name: &str,
        start_time: &DateTime<Utc>,
    ) -> String {
        format!(
            "{}:{}:{}:{}",
            series_id,
            round.unwrap_or(0),
            session_name,
            start_time.to_rfc3339()
        )
    }

    /// Check if a session has already been notified.
    pub fn is_notified(
        &self,
        series_id: &str,
        round: Option<u32>,
        session_name: &str,
        start_time: &DateTime<Utc>,
    ) -> bool {
        let key = Self::session_key(series_id, round, session_name, start_time);
        self.notified_keys.contains(&key)
    }

    /// Mark a session as notified.
    pub fn record(
        &mut self,
        series_id: &str,
        round: Option<u32>,
        session_name: &str,
        start_time: &DateTime<Utc>,
    ) {
        let key = Self::session_key(series_id, round, session_name, start_time);
        self.notified_keys.insert(key);
    }

    /// Clear all notified keys.
    pub fn clear(&mut self) {
        self.notified_keys.clear();
    }

    /// Total count of notified sessions tracked.
    pub fn len(&self) -> usize {
        self.notified_keys.len()
    }

    /// Whether tracker has no notified records.
    pub fn is_empty(&self) -> bool {
        self.notified_keys.is_empty()
    }
}

/// Check all upcoming sessions against daemon configuration thresholds and filters,
/// send notifications for eligible sessions, and record them in the tracker.
/// Returns the list of `NotificationPayload` items that were dispatched.
pub fn check_and_notify(
    events: &HashMap<String, Vec<RaceEvent>>,
    series_registry: &HashMap<String, Series>,
    config: &UserConfig,
    tracker: &mut NotificationTracker,
    backend: NotificationBackend,
) -> Vec<NotificationPayload> {
    check_and_notify_at(
        events,
        series_registry,
        config,
        tracker,
        backend,
        Utc::now(),
    )
}

/// Core evaluation logic accepting an explicit timestamp (useful for testing and deterministic scheduling).
pub fn check_and_notify_at(
    events: &HashMap<String, Vec<RaceEvent>>,
    series_registry: &HashMap<String, Series>,
    config: &UserConfig,
    tracker: &mut NotificationTracker,
    backend: NotificationBackend,
    now: DateTime<Utc>,
) -> Vec<NotificationPayload> {
    let mut sent_notifications = Vec::new();
    let threshold = chrono::Duration::minutes(config.daemon.notify_minutes_before as i64);

    for (series_id, race_events) in events {
        // 1. Check series filter (favorites, all, or specific series ID)
        if !config
            .daemon
            .should_notify_series(series_id, &config.favorites, &config.hidden_series)
        {
            continue;
        }

        let series_name = series_registry
            .get(series_id)
            .map(|s| s.short_name.as_str())
            .unwrap_or(series_id.as_str());

        for event in race_events {
            for session in &event.sessions {
                // 2. Skip completed sessions or sessions without start time
                if session.start_time.is_none() || session.is_completed(series_id) {
                    continue;
                }

                // 3. Check session type filter
                if !config.daemon.should_notify_session(&session.name) {
                    continue;
                }

                let start_time = session.start_time.unwrap();
                let time_until = start_time.signed_duration_since(now);

                // 4. Check if within notification threshold (0 < time_until <= threshold)
                if time_until > chrono::Duration::zero() && time_until <= threshold {
                    if tracker.is_notified(series_id, event.round, &session.name, &start_time) {
                        continue;
                    }

                    let total_mins = time_until.num_minutes();
                    let time_str = if total_mins <= 0 {
                        "starting now!".to_string()
                    } else if total_mins == 1 {
                        "starting in 1 minute".to_string()
                    } else if total_mins < 60 {
                        format!("starting in {} minutes", total_mins)
                    } else {
                        let hours = total_mins / 60;
                        let mins = total_mins % 60;
                        if mins == 0 {
                            format!("starting in {}h", hours)
                        } else {
                            format!("starting in {}h {}m", hours, mins)
                        }
                    };

                    let summary = format!("{} - {}", series_name, event.event_name);
                    let body = format!("{} {}", session.name, time_str);

                    let mut payload = NotificationPayload::new(summary, body)
                        .with_app_name("racetui")
                        .with_icon("dialog-information");

                    if config.daemon.notify_sound {
                        payload = payload.with_sound("message-new-instant");
                    }

                    // Attempt dispatch
                    if let Err(e) = send_notification(&payload, backend) {
                        tracing::warn!("Failed to dispatch notification: {:?}", e);
                    }

                    tracker.record(series_id, event.round, &session.name, &start_time);
                    sent_notifications.push(payload);
                }
            }
        }
    }

    sent_notifications
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::models::{CarStyle, EventStatus, Session, SessionType};

    fn make_test_series(id: &str, short_name: &str) -> Series {
        Series {
            id: id.to_string(),
            name: format!("Series {}", short_name),
            short_name: short_name.to_string(),
            car_style: CarStyle::OpenWheel,
            color: (255, 0, 0),
            region: "International".to_string(),
            calendar_url: "https://example.com".to_string(),
            requires_js: false,
        }
    }

    #[test]
    fn test_notification_tracker_basic() {
        let mut tracker = NotificationTracker::new();
        let now = Utc::now();

        assert!(!tracker.is_notified("f1", Some(1), "Race", &now));
        assert_eq!(tracker.len(), 0);

        tracker.record("f1", Some(1), "Race", &now);
        assert!(tracker.is_notified("f1", Some(1), "Race", &now));
        assert_eq!(tracker.len(), 1);

        tracker.clear();
        assert!(!tracker.is_notified("f1", Some(1), "Race", &now));
        assert!(tracker.is_empty());
    }

    #[test]
    fn test_check_and_notify_upcoming_session() {
        let mut registry = HashMap::new();
        registry.insert("f1".to_string(), make_test_series("f1", "F1"));

        let now = Utc::now();
        let session_start = now + chrono::Duration::minutes(20);

        let event = RaceEvent {
            series_id: "f1".to_string(),
            event_name: "Bahrain Grand Prix".to_string(),
            circuit_name: "Sakhir".to_string(),
            location: "Sakhir".to_string(),
            country: "Bahrain".to_string(),
            start_date: session_start.date_naive(),
            end_date: session_start.date_naive(),
            round: Some(1),
            sessions: vec![
                Session {
                    name: "Practice 1".to_string(),
                    session_type: SessionType::Practice,
                    start_time: Some(now - chrono::Duration::hours(2)),
                    end_time: Some(now - chrono::Duration::hours(1)),
                },
                Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: Some(session_start),
                    end_time: None,
                },
            ],
            stream_links: vec![],
            status: EventStatus::Upcoming,
        };

        let mut events = HashMap::new();
        events.insert("f1".to_string(), vec![event]);

        let mut config = UserConfig::default();
        config.favorites.insert("f1".to_string());
        config.daemon.notify_minutes_before = 30;
        config.daemon.notify_session_types = vec!["qualifying".to_string(), "race".to_string()];
        config.daemon.notify_series_filter = "favorites".to_string();

        let mut tracker = NotificationTracker::new();

        // 1. First run: should trigger Qualifying notification
        let sent = check_and_notify_at(
            &events,
            &registry,
            &config,
            &mut tracker,
            NotificationBackend::TerminalOnly,
            now,
        );

        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].summary, "F1 - Bahrain Grand Prix");
        assert!(sent[0].body.contains("Qualifying"));
        assert!(sent[0].body.contains("starting in 20 minutes"));

        // 2. Second run: already tracked, should not re-trigger
        let sent_second = check_and_notify_at(
            &events,
            &registry,
            &config,
            &mut tracker,
            NotificationBackend::TerminalOnly,
            now,
        );
        assert_eq!(sent_second.len(), 0);
    }

    #[test]
    fn test_check_and_notify_session_filters_and_hidden_series() {
        let mut registry = HashMap::new();
        registry.insert("f1".to_string(), make_test_series("f1", "F1"));
        registry.insert("nascar".to_string(), make_test_series("nascar", "NASCAR"));

        let now = Utc::now();
        let session_start = now + chrono::Duration::minutes(15);

        let event_f1 = RaceEvent {
            series_id: "f1".to_string(),
            event_name: "Monaco GP".to_string(),
            circuit_name: "Monaco".to_string(),
            location: "Monte Carlo".to_string(),
            country: "Monaco".to_string(),
            start_date: session_start.date_naive(),
            end_date: session_start.date_naive(),
            round: Some(6),
            sessions: vec![Session {
                name: "Practice 1".to_string(), // Practice, should be ignored by filter
                session_type: SessionType::Practice,
                start_time: Some(session_start),
                end_time: None,
            }],
            stream_links: vec![],
            status: EventStatus::Upcoming,
        };

        let event_nascar = RaceEvent {
            series_id: "nascar".to_string(),
            event_name: "Daytona 500".to_string(),
            circuit_name: "Daytona".to_string(),
            location: "Daytona".to_string(),
            country: "USA".to_string(),
            start_date: session_start.date_naive(),
            end_date: session_start.date_naive(),
            round: Some(1),
            sessions: vec![Session {
                name: "Race".to_string(),
                session_type: SessionType::Race,
                start_time: Some(session_start),
                end_time: None,
            }],
            stream_links: vec![],
            status: EventStatus::Upcoming,
        };

        let mut events = HashMap::new();
        events.insert("f1".to_string(), vec![event_f1]);
        events.insert("nascar".to_string(), vec![event_nascar]);

        let mut config = UserConfig::default();
        config.daemon.notify_minutes_before = 30;
        config.daemon.notify_session_types = vec!["race".to_string()]; // only Race
        config.daemon.notify_series_filter = "all".to_string();
        config.hidden_series.insert("nascar".to_string()); // nascar is hidden

        let mut tracker = NotificationTracker::new();

        let sent = check_and_notify_at(
            &events,
            &registry,
            &config,
            &mut tracker,
            NotificationBackend::TerminalOnly,
            now,
        );

        // F1 session was Practice (filtered out); NASCAR was hidden (filtered out)
        assert_eq!(sent.len(), 0);
    }
}
