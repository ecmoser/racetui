use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::live::event::LiveEvent;
use crate::live::get_live_provider;

/// Manages live timing background polling tasks for active racing series.
pub struct LiveSessionManager {
    tx: mpsc::UnboundedSender<LiveEvent>,
    active_sessions: HashMap<String, JoinHandle<()>>,
}

impl LiveSessionManager {
    /// Create a new LiveSessionManager that dispatches events over the given channel.
    pub fn new(tx: mpsc::UnboundedSender<LiveEvent>) -> Self {
        Self {
            tx,
            active_sessions: HashMap::new(),
        }
    }

    /// Start polling live timing for a series at the specified interval in seconds.
    pub fn start_session(&mut self, series_id: &str, poll_interval_secs: u64) {
        // Stop any existing polling task for this series first
        self.stop_session(series_id);

        let provider = match get_live_provider(series_id) {
            Some(p) => p,
            None => {
                let _ = self.tx.send(LiveEvent::LiveError {
                    series_id: series_id.to_string(),
                    error: format!("No live timing provider available for series '{}'", series_id),
                });
                return;
            }
        };

        let sid = series_id.to_string();
        let tx = self.tx.clone();
        let interval = Duration::from_secs(poll_interval_secs.max(1));

        let handle = tokio::spawn(async move {
            let _ = tx.send(LiveEvent::SessionStarted {
                series_id: sid.clone(),
                session_name: "Live Session".to_string(),
            });

            loop {
                match provider.fetch_timing_boxed().await {
                    Ok(timing_data) => {
                        let is_finished = timing_data.session_status.eq_ignore_ascii_case("finished")
                            || timing_data.session_status.eq_ignore_ascii_case("completed");

                        let _ = tx.send(LiveEvent::TimingUpdate {
                            series_id: sid.clone(),
                            data: Box::new(timing_data),
                        });

                        if is_finished {
                            let _ = tx.send(LiveEvent::SessionEnded {
                                series_id: sid.clone(),
                            });
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(LiveEvent::LiveError {
                            series_id: sid.clone(),
                            error: e.to_string(),
                        });
                    }
                }

                tokio::time::sleep(interval).await;
            }
        });

        self.active_sessions.insert(series_id.to_string(), handle);
    }

    /// Stop live timing polling for a specific series.
    pub fn stop_session(&mut self, series_id: &str) {
        if let Some(handle) = self.active_sessions.remove(series_id) {
            handle.abort();
        }
    }

    /// Stop all active live timing polling tasks.
    pub fn stop_all(&mut self) {
        for (_, handle) in self.active_sessions.drain() {
            handle.abort();
        }
    }

    /// Returns true if live timing is currently being polled for the given series.
    pub fn is_active(&self, series_id: &str) -> bool {
        self.active_sessions.contains_key(series_id)
    }

    /// Returns the list of series IDs currently being actively polled.
    pub fn active_series(&self) -> Vec<String> {
        let mut list: Vec<String> = self.active_sessions.keys().cloned().collect();
        list.sort();
        list
    }
}

impl Drop for LiveSessionManager {
    fn drop(&mut self) {
        self.stop_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_live_session_manager_lifecycle() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut manager = LiveSessionManager::new(tx);

        assert!(!manager.is_active("f1"));
        assert!(manager.active_series().is_empty());

        // Start session for series without provider -> sends error event
        manager.start_session("f1", 2);
        assert!(!manager.is_active("f1"));

        if let Some(event) = rx.recv().await {
            match event {
                LiveEvent::LiveError { series_id, error } => {
                    assert_eq!(series_id, "f1");
                    assert!(error.contains("No live timing provider"));
                }
                _ => panic!("Expected LiveError event"),
            }
        } else {
            panic!("Expected an event from manager");
        }

        manager.stop_all();
    }
}
