pub mod btcc;
pub mod dtm;
pub mod f1;
pub mod f2;
pub mod f3;
pub mod fetcher;
pub mod formula_e;
pub mod imsa;
pub mod indycar;
pub mod json_ld;
pub mod motogp;
pub mod nascar;
pub mod sportstimes;
pub mod super_formula;
pub mod super_gt;
pub mod wec;
pub mod wrc;

use anyhow::Result;

use crate::data::models::{RaceEvent, Series};

/// Trait that all series scrapers must implement.
/// Each series has its own module that implements this trait.
///
/// The `scrape` method is called with the series metadata (which includes
/// the calendar URL and other config from series.toml).
/// It should return a Vec of RaceEvent for the current season.
pub trait SeriesScraper: Send + Sync {
    /// Scrape/fetch the race calendar for this series.
    /// The `series` parameter contains metadata from series.toml,
    /// including the calendar_url.
    fn scrape(
        &self,
        series: &Series,
    ) -> impl std::future::Future<Output = Result<Vec<RaceEvent>>> + Send;
}

/// Get the appropriate scraper for a series ID.
/// Returns None if no scraper is implemented yet for this series.
pub fn get_scraper(series_id: &str) -> Option<Box<dyn SeriesScraperBoxed>> {
    match series_id {
        "f1" => Some(Box::new(f1::F1Scraper)),
        "f2" => Some(Box::new(f2::F2Scraper)),
        "f3" => Some(Box::new(f3::F3Scraper)),
        "formula_e" => Some(Box::new(formula_e::FormulaEScraper)),
        "indycar" => Some(Box::new(indycar::IndyCarScraper)),
        "imsa" => Some(Box::new(imsa::ImsaScraper)),
        "wec" => Some(Box::new(wec::WecScraper)),
        "motogp" => Some(Box::new(motogp::MotoGpScraper { category: "motogp" })),
        "moto2" => Some(Box::new(motogp::MotoGpScraper { category: "moto2" })),
        "moto3" => Some(Box::new(motogp::MotoGpScraper { category: "moto3" })),
        "wrc" => Some(Box::new(wrc::WrcScraper)),
        "wrc2" => Some(Box::new(wrc::WrcScraper)),
        "btcc" => Some(Box::new(btcc::BtccScraper)),
        "dtm" => Some(Box::new(dtm::DtmScraper)),
        "super_formula" => Some(Box::new(super_formula::SuperFormulaScraper)),
        "super_gt" => Some(Box::new(super_gt::SuperGtScraper)),
        "nascar_cup" => Some(Box::new(nascar::NascarScraper {
            nascar_series_id: 1,
            racetui_series_id: "nascar_cup",
        })),
        "nascar_xfinity" => Some(Box::new(nascar::NascarScraper {
            nascar_series_id: 2,
            racetui_series_id: "nascar_xfinity",
        })),
        "nascar_trucks" => Some(Box::new(nascar::NascarScraper {
            nascar_series_id: 3,
            racetui_series_id: "nascar_trucks",
        })),
        _ => None,
    }
}

/// Object-safe version of SeriesScraper for dynamic dispatch.
/// This is needed because the base SeriesScraper trait uses
/// `impl Future` which isn't object-safe.
pub trait SeriesScraperBoxed: Send + Sync {
    fn scrape_boxed<'a>(
        &'a self,
        series: &'a Series,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<RaceEvent>>> + Send + 'a>>;
}

/// Blanket implementation: any type that implements SeriesScraper
/// automatically implements SeriesScraperBoxed.
impl<T: SeriesScraper> SeriesScraperBoxed for T {
    fn scrape_boxed<'a>(
        &'a self,
        series: &'a Series,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<RaceEvent>>> + Send + 'a>>
    {
        Box::pin(self.scrape(series))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::models::CarStyle;

    struct DummyScraper;
    impl SeriesScraper for DummyScraper {
        async fn scrape(&self, _series: &Series) -> Result<Vec<RaceEvent>> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_series_scraper_boxed_dispatch() {
        let dummy = DummyScraper;
        let boxed: Box<dyn SeriesScraperBoxed> = Box::new(dummy);
        let series = Series {
            id: "test".to_string(),
            name: "Test".to_string(),
            short_name: "T".to_string(),
            car_style: CarStyle::OpenWheel,
            color: (0, 0, 0),
            region: "Global".to_string(),
            calendar_url: "https://example.com".to_string(),
            requires_js: false,
        };

        let res = boxed.scrape_boxed(&series).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap().len(), 0);
    }

    #[test]
    fn test_get_scraper() {
        assert!(get_scraper("f1").is_some());
        assert!(get_scraper("f2").is_some());
        assert!(get_scraper("f3").is_some());
        assert!(get_scraper("formula_e").is_some());
        assert!(get_scraper("indycar").is_some());
        assert!(get_scraper("imsa").is_some());
        assert!(get_scraper("wec").is_some());
        assert!(get_scraper("motogp").is_some());
        assert!(get_scraper("moto2").is_some());
        assert!(get_scraper("moto3").is_some());
        assert!(get_scraper("wrc").is_some());
        assert!(get_scraper("wrc2").is_some());
        assert!(get_scraper("btcc").is_some());
        assert!(get_scraper("dtm").is_some());
        assert!(get_scraper("super_formula").is_some());
        assert!(get_scraper("super_gt").is_some());
        assert!(get_scraper("nascar_cup").is_some());
        assert!(get_scraper("nascar_xfinity").is_some());
        assert!(get_scraper("nascar_trucks").is_some());
        assert!(get_scraper("unknown_series").is_none());
    }
}

