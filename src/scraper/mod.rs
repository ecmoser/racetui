pub mod bsb;
pub mod btcc;
pub mod dakar;
pub mod dtm;
pub mod extreme_e;
pub mod f1;
pub mod f1_academy;
pub mod f2;
pub mod f3;
pub mod fetcher;
pub mod formula_e;
pub mod imsa;
pub mod indy_nxt;
pub mod indycar;
pub mod json_ld;
pub mod lmem;
pub mod motogp;
pub mod nascar;
pub mod nls;
pub mod porsche;
pub mod sportstimes;
pub mod sro;
pub mod super_formula;
pub mod super_gt;
pub mod supercars;
pub mod tcr;
pub mod wec;
pub mod worldsbk;
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
        "erc" => Some(Box::new(wrc::WrcScraper)),
        "btcc" => Some(Box::new(btcc::BtccScraper)),
        "dtm" => Some(Box::new(dtm::DtmScraper)),
        "super_formula" => Some(Box::new(super_formula::SuperFormulaScraper)),
        "super_gt" => Some(Box::new(super_gt::SuperGtScraper)),
        "gtwc_eu" => Some(Box::new(sro::SroScraper {
            category: "gtwc_eu",
        })),
        "gtwc_am" => Some(Box::new(sro::SroScraper {
            category: "gtwc_am",
        })),
        "igtc" => Some(Box::new(sro::SroScraper { category: "igtc" })),
        "elms" => Some(Box::new(lmem::LmemScraper { category: "elms" })),
        "aslms" => Some(Box::new(lmem::LmemScraper { category: "aslms" })),
        "f1_academy" => Some(Box::new(f1_academy::F1AcademyScraper)),
        "indy_nxt" => Some(Box::new(indy_nxt::IndyNxtScraper)),
        "worldsbk" => Some(Box::new(worldsbk::WorldSbkScraper)),
        "bsb" => Some(Box::new(bsb::BsbScraper)),
        "supercars" => Some(Box::new(supercars::SupercarsScraper)),
        "tcr_world" => Some(Box::new(tcr::TcrScraper)),
        "porsche_supercup" => Some(Box::new(porsche::PorscheScraper)),
        "nls" => Some(Box::new(nls::NlsScraper)),
        "dakar" => Some(Box::new(dakar::DakarScraper)),
        "extreme_e" => Some(Box::new(extreme_e::ExtremeEScraper)),
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
        "arca" => Some(Box::new(nascar::NascarScraper {
            nascar_series_id: 4,
            racetui_series_id: "arca",
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

use crate::data::results::RaceResults;
use crate::data::standings::SeasonStandings;

/// Trait for fetching championship standings for a series.
pub trait StandingsFetcher: Send + Sync {
    /// Fetch championship standings for the given season.
    fn fetch_standings(
        &self,
        season: u32,
    ) -> impl std::future::Future<Output = Result<SeasonStandings>> + Send;
}

/// Object-safe version of StandingsFetcher for dynamic dispatch.
pub trait StandingsFetcherBoxed: Send + Sync {
    fn fetch_standings_boxed<'a>(
        &'a self,
        season: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SeasonStandings>> + Send + 'a>>;
}

impl<T: StandingsFetcher> StandingsFetcherBoxed for T {
    fn fetch_standings_boxed<'a>(
        &'a self,
        season: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SeasonStandings>> + Send + 'a>>
    {
        Box::pin(self.fetch_standings(season))
    }
}

/// Get the standings fetcher for a series ID.
pub fn get_standings_fetcher(series_id: &str) -> Option<Box<dyn StandingsFetcherBoxed>> {
    match series_id {
        "f1" => Some(Box::new(f1::F1Scraper)),
        _ => None,
    }
}

/// Trait for fetching race results for a completed event.
pub trait ResultsFetcher: Send + Sync {
    /// Fetch results for a specific round in a season.
    fn fetch_results(
        &self,
        season: u32,
        round: u32,
    ) -> impl std::future::Future<Output = Result<RaceResults>> + Send;
}

/// Object-safe version of ResultsFetcher for dynamic dispatch.
pub trait ResultsFetcherBoxed: Send + Sync {
    fn fetch_results_boxed<'a>(
        &'a self,
        season: u32,
        round: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<RaceResults>> + Send + 'a>>;
}

impl<T: ResultsFetcher> ResultsFetcherBoxed for T {
    fn fetch_results_boxed<'a>(
        &'a self,
        season: u32,
        round: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<RaceResults>> + Send + 'a>> {
        Box::pin(self.fetch_results(season, round))
    }
}

/// Get the results fetcher for a series ID.
pub fn get_results_fetcher(_series_id: &str) -> Option<Box<dyn ResultsFetcherBoxed>> {
    None
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
        assert!(get_scraper("gtwc_eu").is_some());
        assert!(get_scraper("gtwc_am").is_some());
        assert!(get_scraper("igtc").is_some());
        assert!(get_scraper("elms").is_some());
        assert!(get_scraper("aslms").is_some());
        assert!(get_scraper("f1_academy").is_some());
        assert!(get_scraper("indy_nxt").is_some());
        assert!(get_scraper("worldsbk").is_some());
        assert!(get_scraper("bsb").is_some());
        assert!(get_scraper("supercars").is_some());
        assert!(get_scraper("tcr_world").is_some());
        assert!(get_scraper("porsche_supercup").is_some());
        assert!(get_scraper("nls").is_some());
        assert!(get_scraper("dakar").is_some());
        assert!(get_scraper("erc").is_some());
        assert!(get_scraper("extreme_e").is_some());
        assert!(get_scraper("arca").is_some());
        assert!(get_scraper("unknown_series").is_none());
    }

    struct DummyStandingsFetcher;
    impl StandingsFetcher for DummyStandingsFetcher {
        async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
            Ok(SeasonStandings {
                series_id: "test".to_string(),
                season,
                drivers: vec![],
                constructors: vec![],
                fetched_at: chrono::Utc::now(),
            })
        }
    }

    #[tokio::test]
    async fn test_standings_fetcher_boxed_dispatch() {
        let dummy = DummyStandingsFetcher;
        let boxed: Box<dyn StandingsFetcherBoxed> = Box::new(dummy);
        let res = boxed.fetch_standings_boxed(2026).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap().season, 2026);
    }

    #[test]
    fn test_get_standings_fetcher() {
        assert!(get_standings_fetcher("f1").is_some());
        assert!(get_standings_fetcher("unknown_series").is_none());
    }
}
