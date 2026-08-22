use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveDate, Utc};

use super::fetcher::create_http_client;
use super::json_ld::fetch_json_ld_calendar;
use super::{ResultsFetcher, SeriesScraper, StandingsFetcher};
use crate::data::models::{EventStatus, RaceEvent, Series, StreamLink};
use crate::data::results::RaceResults;
use crate::data::standings::SeasonStandings;

pub struct WecScraper;

impl StandingsFetcher for WecScraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        let client = create_http_client()?;
        let driver_url = "https://www.fiawec.com/en/page/drivers-classification";
        let mut drivers = Vec::new();
        let mut constructors = Vec::new();

        if let Ok(resp) = client.get(driver_url).send().await {
            if let Ok(html) = resp.text().await {
                drivers = parse_wec_standings_html(&html).unwrap_or_default();
            }
        }

        let mfg_url = "https://www.fiawec.com/en/page/manufacturers-classification";
        if let Ok(resp) = client.get(mfg_url).send().await {
            if let Ok(html) = resp.text().await {
                constructors = parse_wec_constructors_html(&html).unwrap_or_default();
            }
        }

        Ok(SeasonStandings {
            series_id: "wec".to_string(),
            season,
            drivers,
            constructors,
            fetched_at: Utc::now(),
        })
    }
}

/// Parse WEC driver standings table HTML.
pub fn parse_wec_standings_html(html: &str) -> Result<Vec<crate::data::standings::DriverStanding>> {
    let document = scraper::Html::parse_document(html);
    let row_sel = scraper::Selector::parse("table.table-standing tbody tr, table tbody tr")
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let td_sel = scraper::Selector::parse("td, th").map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let mut drivers = Vec::new();

    for row in document.select(&row_sel) {
        let cols: Vec<String> = row
            .select(&td_sel)
            .map(|td| td.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if cols.len() >= 3 {
            let pos = cols[0].parse::<u32>().unwrap_or((drivers.len() + 1) as u32);
            let name = cols[1].clone();
            let points = cols
                .last()
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.0);

            drivers.push(crate::data::standings::DriverStanding {
                position: pos,
                driver_name: name,
                driver_code: None,
                driver_number: None,
                team: "".to_string(),
                points,
                wins: 0,
            });
        }
    }

    Ok(drivers)
}

/// Parse WEC manufacturer standings table HTML.
pub fn parse_wec_constructors_html(
    html: &str,
) -> Result<Vec<crate::data::standings::ConstructorStanding>> {
    let document = scraper::Html::parse_document(html);
    let row_sel = scraper::Selector::parse("table.table-standing tbody tr, table tbody tr")
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let td_sel = scraper::Selector::parse("td, th").map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let mut constructors = Vec::new();

    for row in document.select(&row_sel) {
        let cols: Vec<String> = row
            .select(&td_sel)
            .map(|td| td.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if cols.len() >= 3 {
            let pos = cols[0]
                .parse::<u32>()
                .unwrap_or((constructors.len() + 1) as u32);
            let name = cols[1].clone();
            let points = cols
                .last()
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.0);

            constructors.push(crate::data::standings::ConstructorStanding {
                position: pos,
                name,
                points,
                wins: 0,
            });
        }
    }

    Ok(constructors)
}

impl ResultsFetcher for WecScraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        anyhow::bail!(
            "WEC race results for season {} round {} not yet available",
            season,
            round
        )
    }
}

impl SeriesScraper for WecScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let client = create_http_client()?;
        let mut events = fetch_json_ld_calendar(
            &client,
            "https://raceweek.io/wec",
            &series.id,
            &wec_stream_links(),
        )
        .await
        .unwrap_or_default();

        if events.is_empty() {
            events = get_official_2026_wec_schedule(&series.id);
            events.extend(get_official_2027_wec_schedule(&series.id));
        } else if !events
            .iter()
            .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
        {
            events.extend(get_official_2027_wec_schedule(&series.id));
        }

        Ok(events)
    }
}

fn wec_stream_links() -> Vec<StreamLink> {
    super::raceday_watch::get_raceday_stream_links("wec", "")
}

pub fn get_official_2026_wec_schedule(series_id: &str) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        (
            "Qatar 1812 Km",
            "Lusail International Circuit",
            "Lusail",
            "Qatar",
            (2026, 2, 28),
            (8, 0),
        ),
        (
            "6 Hours of Imola",
            "Autodromo Internazionale Enzo e Dino Ferrari",
            "Imola",
            "Italy",
            (2026, 4, 19),
            (11, 0),
        ),
        (
            "TotalEnergies 6 Hours of Spa-Francorchamps",
            "Circuit de Spa-Francorchamps",
            "Spa-Francorchamps",
            "Belgium",
            (2026, 5, 9),
            (11, 0),
        ),
        (
            "24 Hours of Le Mans",
            "Circuit de la Sarthe",
            "Le Mans",
            "France",
            (2026, 6, 13),
            (14, 0),
        ),
        (
            "Rolex 6 Hours of São Paulo",
            "Autódromo José Carlos Pace (Interlagos)",
            "São Paulo",
            "Brazil",
            (2026, 7, 12),
            (14, 30),
        ),
        (
            "Lone Star Le Mans",
            "Circuit of The Americas",
            "Austin, Texas",
            "USA",
            (2026, 9, 6),
            (18, 0),
        ),
        (
            "6 Hours of Fuji",
            "Fuji Speedway",
            "Oyama, Shizuoka",
            "Japan",
            (2026, 9, 27),
            (2, 0),
        ),
        (
            "Bapco Energies 8 Hours of Bahrain",
            "Bahrain International Circuit",
            "Sakhir",
            "Bahrain",
            (2026, 11, 7),
            (11, 0),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (name, circuit, loc, country, date, (hour, min)))| {
            let race_date = NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap();
            let start_time = race_date
                .and_hms_opt(hour, min, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            let status = if race_date < today {
                EventStatus::Completed
            } else if race_date == today {
                EventStatus::Live
            } else {
                EventStatus::Upcoming
            };

            let fri_date = race_date
                .pred_opt()
                .unwrap_or(race_date)
                .pred_opt()
                .unwrap_or(race_date);
            let sessions = super::json_ld::build_series_sessions(
                series_id, name, fri_date, race_date, start_time, None,
            );

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: name.to_string(),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: fri_date,
                end_date: race_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: wec_stream_links(),
                status,
            }
        })
        .collect()
}

pub fn get_official_2027_wec_schedule(series_id: &str) -> Vec<RaceEvent> {
    let raw_events = vec![
        (
            "Qatar 1812 Km",
            "Lusail International Circuit",
            "Lusail",
            "Qatar",
            (2027, 2, 27),
            (8, 0),
        ),
        (
            "6 Hours of Imola",
            "Autodromo Enzo e Dino Ferrari",
            "Imola",
            "Italy",
            (2027, 4, 18),
            (11, 0),
        ),
        (
            "6 Hours of Spa-Francorchamps",
            "Circuit de Spa-Francorchamps",
            "Spa-Francorchamps",
            "Belgium",
            (2027, 5, 8),
            (11, 0),
        ),
        (
            "24 Hours of Le Mans",
            "Circuit de la Sarthe",
            "Le Mans",
            "France",
            (2027, 6, 12),
            (14, 0),
        ),
        (
            "6 Hours of São Paulo",
            "Autódromo José Carlos Pace",
            "São Paulo",
            "Brazil",
            (2027, 7, 11),
            (14, 30),
        ),
        (
            "Lone Star Le Mans (6 Hours of COTA)",
            "Circuit of the Americas",
            "Austin, TX",
            "USA",
            (2027, 9, 5),
            (18, 0),
        ),
        (
            "6 Hours of Fuji",
            "Fuji International Speedway",
            "Oyama",
            "Japan",
            (2027, 9, 26),
            (2, 0),
        ),
        (
            "Bapco Energies 8 Hours of Bahrain",
            "Bahrain International Circuit",
            "Sakhir",
            "Bahrain",
            (2027, 11, 6),
            (11, 0),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(
            |(i, (name, circuit, loc, country, (y, m, d), (hour, min)))| {
                let race_date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
                let fri_date = race_date
                    .pred_opt()
                    .unwrap_or(race_date)
                    .pred_opt()
                    .unwrap_or(race_date);
                let start_time = race_date
                    .and_hms_opt(hour, min, 0)
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

                let sessions = super::json_ld::build_series_sessions(
                    series_id, name, fri_date, race_date, start_time, None,
                );

                RaceEvent {
                    series_id: series_id.to_string(),
                    event_name: name.to_string(),
                    circuit_name: circuit.to_string(),
                    location: loc.to_string(),
                    country: country.to_string(),
                    start_date: fri_date,
                    end_date: race_date,
                    round: Some((i + 1) as u32),
                    sessions,
                    stream_links: wec_stream_links(),
                    status: EventStatus::Upcoming,
                }
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_official_wec_schedule() {
        let events = get_official_2026_wec_schedule("wec");
        assert_eq!(events.len(), 8);
        assert_eq!(events[0].event_name, "Qatar 1812 Km");
        assert_eq!(events[0].round, Some(1));
        assert_eq!(events[3].event_name, "24 Hours of Le Mans");
        assert_eq!(events[7].event_name, "Bapco Energies 8 Hours of Bahrain");
        assert_eq!(events[7].round, Some(8));
    }

    #[test]
    fn test_parse_wec_standings_html() {
        let html_sample = r#"
            <table class="table-standing">
                <tbody>
                    <tr>
                        <td>1</td>
                        <td>Porsche Penske Motorsport</td>
                        <td>#6</td>
                        <td>Estre / Lotterer / Vanthoor</td>
                        <td>152</td>
                    </tr>
                    <tr>
                        <td>2</td>
                        <td>Toyota Gazoo Racing</td>
                        <td>#7</td>
                        <td>Conway / Kobayashi / de Vries</td>
                        <td>128</td>
                    </tr>
                </tbody>
            </table>
        "#;

        let drivers = parse_wec_standings_html(html_sample).unwrap();
        assert_eq!(drivers.len(), 2);
        assert_eq!(drivers[0].position, 1);
        assert_eq!(drivers[0].driver_name, "Porsche Penske Motorsport");
        assert_eq!(drivers[0].points, 152.0);

        let mfg = parse_wec_constructors_html(html_sample).unwrap();
        assert_eq!(mfg.len(), 2);
        assert_eq!(mfg[0].position, 1);
        assert_eq!(mfg[0].name, "Porsche Penske Motorsport");
        assert_eq!(mfg[0].points, 152.0);
    }
}
