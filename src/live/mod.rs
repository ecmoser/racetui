pub mod event;
pub mod manager;
pub mod track_map;
pub mod weather;

use anyhow::Result;
pub use event::LiveEvent;
use crate::live::event::LiveTimingData;

/// Trait that all series live timing providers must implement.
pub trait LiveProvider: Send + Sync {
    /// Check if a live session is currently active for this series.
    fn is_session_live(&self) -> impl std::future::Future<Output = Result<bool>> + Send;

    /// Fetch the latest live timing data snapshot.
    fn fetch_timing(&self) -> impl std::future::Future<Output = Result<LiveTimingData>> + Send;
}

/// Object-safe version of LiveProvider for dynamic dispatch.
pub trait LiveProviderBoxed: Send + Sync {
    fn is_session_live_boxed<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool>> + Send + 'a>>;

    fn fetch_timing_boxed<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LiveTimingData>> + Send + 'a>>;
}

impl<T: LiveProvider> LiveProviderBoxed for T {
    fn is_session_live_boxed<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool>> + Send + 'a>> {
        Box::pin(self.is_session_live())
    }

    fn fetch_timing_boxed<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LiveTimingData>> + Send + 'a>> {
        Box::pin(self.fetch_timing())
    }
}

/// Get the live timing provider for a series ID.
/// Returns None if no live timing provider is implemented yet for this series.
pub fn get_live_provider(series_id: &str) -> Option<Box<dyn LiveProviderBoxed>> {
    match series_id {
        "f1" => Some(Box::new(crate::scraper::f1_live::F1LiveProvider::new())),
        "nascar_cup" => Some(Box::new(crate::scraper::nascar_live::NascarLiveProvider::new(
            1,
            "nascar_cup",
        ))),
        "nascar_xfinity" => Some(Box::new(crate::scraper::nascar_live::NascarLiveProvider::new(
            2,
            "nascar_xfinity",
        ))),
        "nascar_trucks" => Some(Box::new(crate::scraper::nascar_live::NascarLiveProvider::new(
            3,
            "nascar_trucks",
        ))),
        "arca" => Some(Box::new(crate::scraper::nascar_live::NascarLiveProvider::new(
            4,
            "arca",
        ))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    struct DummyLiveProvider;
    impl LiveProvider for DummyLiveProvider {
        async fn is_session_live(&self) -> Result<bool> {
            Ok(true)
        }

        async fn fetch_timing(&self) -> Result<LiveTimingData> {
            Ok(LiveTimingData {
                series_id: "test".to_string(),
                session_name: "Race".to_string(),
                event_name: "Test GP".to_string(),
                circuit_name: "Test Circuit".to_string(),
                total_laps: Some(50),
                current_lap: Some(10),
                time_remaining: None,
                session_status: "Green".to_string(),
                drivers: vec![],
                weather: None,
                updated_at: Utc::now(),
            })
        }
    }

    #[tokio::test]
    async fn test_live_provider_boxed_dispatch() {
        let dummy = DummyLiveProvider;
        let boxed: Box<dyn LiveProviderBoxed> = Box::new(dummy);
        let is_live = boxed.is_session_live_boxed().await;
        assert!(is_live.is_ok());
        assert_eq!(is_live.unwrap(), true);

        let timing = boxed.fetch_timing_boxed().await;
        assert!(timing.is_ok());
        assert_eq!(timing.unwrap().series_id, "test");
    }

    #[test]
    fn test_get_live_provider() {
        assert!(get_live_provider("f1").is_some());
        assert!(get_live_provider("nascar_cup").is_some());
        assert!(get_live_provider("nascar_xfinity").is_some());
        assert!(get_live_provider("nascar_trucks").is_some());
        assert!(get_live_provider("arca").is_some());
        assert!(get_live_provider("unknown_series").is_none());
    }
}
