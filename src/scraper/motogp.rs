use anyhow::Result;
use chrono::{Datelike, NaiveDate, Utc};
use serde::Deserialize;

use super::fetcher;
use super::sportstimes::fetch_sportstimes_calendar;
use super::{ResultsFetcher, SeriesScraper, StandingsFetcher};
use crate::data::models::{
    EventStatus, RaceEvent, Series, Session, SessionType, StreamAccess, StreamLink,
};
use crate::data::results::RaceResults;
use crate::data::standings::SeasonStandings;

pub struct MotoGpScraper {
    pub category: &'static str, // "motogp", "moto2", "moto3"
}

impl StandingsFetcher for MotoGpScraper {
    async fn fetch_standings(&self, season: u32) -> Result<SeasonStandings> {
        let client = fetcher::create_http_client()?;

        // 1. Fetch seasons list to resolve UUID
        let seasons_url = "https://api.motogp.pulselive.com/motogp/v1/results/seasons";
        let seasons_text = client.get(seasons_url).send().await?.text().await?;
        let seasons: Vec<MotoGpSeason> = serde_json::from_str(&seasons_text)?;

        let target_season = seasons
            .iter()
            .find(|s| s.year == season as i32)
            .or_else(|| seasons.iter().find(|s| s.current))
            .or_else(|| seasons.first())
            .ok_or_else(|| anyhow::anyhow!("No season found for year {}", season))?;

        // 2. Fetch categories for this season
        let cat_url = format!(
            "https://api.motogp.pulselive.com/motogp/v1/results/categories?seasonUuid={}",
            target_season.id
        );
        let cat_text = client.get(&cat_url).send().await?.text().await?;
        let categories: Vec<MotoGpCategory> = serde_json::from_str(&cat_text)?;

        let cat_target_name = match self.category {
            "moto2" => "Moto2",
            "moto3" => "Moto3",
            _ => "MotoGP",
        };

        let target_category = categories
            .iter()
            .find(|c| {
                c.name
                    .to_lowercase()
                    .contains(&cat_target_name.to_lowercase())
            })
            .or_else(|| categories.first())
            .ok_or_else(|| anyhow::anyhow!("No category found for {}", self.category))?;

        // 3. Fetch standings
        let standings_url = format!(
            "https://api.motogp.pulselive.com/motogp/v1/results/standings?seasonUuid={}&categoryUuid={}",
            target_season.id, target_category.id
        );
        let standings_text = client.get(&standings_url).send().await?.text().await?;
        parse_motogp_standings(&standings_text, self.category, season)
    }
}

/// Parse MotoGP standings JSON response into SeasonStandings.
pub fn parse_motogp_standings(json: &str, series_id: &str, season: u32) -> Result<SeasonStandings> {
    let resp: MotoGpStandingsResponse = serde_json::from_str(json)?;
    let mut drivers = Vec::new();
    let mut constructor_map: std::collections::BTreeMap<String, (f64, u32)> =
        std::collections::BTreeMap::new();

    for entry in resp.classification {
        let rider_name = entry
            .rider
            .as_ref()
            .map(|r| r.full_name.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        let rider_num = entry.rider.as_ref().and_then(|r| r.number);
        let team_name = entry
            .team
            .as_ref()
            .map(|t| t.name.clone())
            .unwrap_or_default();
        let wins = entry.race_wins.unwrap_or(0);

        if let Some(constructor) = entry.constructor {
            let stats = constructor_map.entry(constructor.name).or_insert((0.0, 0));
            stats.0 += entry.points;
            stats.1 += wins;
        }

        drivers.push(crate::data::standings::DriverStanding {
            position: entry.position,
            driver_name: rider_name,
            driver_code: None,
            driver_number: rider_num,
            team: team_name,
            points: entry.points,
            wins,
        });
    }

    let mut constructor_vec: Vec<(String, f64, u32)> = constructor_map
        .into_iter()
        .map(|(k, (pts, w))| (k, pts, w))
        .collect();
    constructor_vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let constructors: Vec<crate::data::standings::ConstructorStanding> = constructor_vec
        .into_iter()
        .enumerate()
        .map(
            |(idx, (name, points, wins))| crate::data::standings::ConstructorStanding {
                position: (idx + 1) as u32,
                name,
                points,
                wins,
            },
        )
        .collect();

    Ok(SeasonStandings {
        series_id: series_id.to_string(),
        season,
        drivers,
        constructors,
        fetched_at: Utc::now(),
    })
}

#[derive(Debug, Deserialize)]
struct MotoGpCategory {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct MotoGpStandingsResponse {
    #[serde(default)]
    classification: Vec<MotoGpStandingEntry>,
}

#[derive(Debug, Deserialize)]
struct MotoGpStandingEntry {
    position: u32,
    rider: Option<MotoGpRiderInfo>,
    team: Option<MotoGpTeamInfo>,
    constructor: Option<MotoGpConstructorInfo>,
    #[serde(default)]
    points: f64,
    #[serde(default)]
    race_wins: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct MotoGpRiderInfo {
    full_name: String,
    number: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct MotoGpTeamInfo {
    name: String,
}

#[derive(Debug, Deserialize)]
struct MotoGpConstructorInfo {
    name: String,
}

impl ResultsFetcher for MotoGpScraper {
    async fn fetch_results(&self, season: u32, round: u32) -> Result<RaceResults> {
        anyhow::bail!(
            "{} race results for season {} round {} not yet available",
            self.category,
            season,
            round
        )
    }
}

#[derive(Debug, Deserialize)]
struct MotoGpSeason {
    id: String,
    year: i32,
    current: bool,
}

#[derive(Debug, Deserialize)]
struct MotoGpEventResponse {
    name: String,
    date_start: Option<String>,
    date_end: Option<String>,
    country: Option<MotoGpCountry>,
    circuit: Option<MotoGpCircuit>,
}

#[derive(Debug, Deserialize)]
struct MotoGpCountry {
    name: String,
}

#[derive(Debug, Deserialize)]
struct MotoGpCircuit {
    name: String,
}

impl SeriesScraper for MotoGpScraper {
    async fn scrape(&self, series: &Series) -> Result<Vec<RaceEvent>> {
        let current_year = Utc::now().year();

        if let Ok(client) = fetcher::create_http_client() {
            // Try motogpcal.com dynamic API first for full practice, qual, sprint, race times
            if let Ok(mut events) = fetch_sportstimes_calendar(
                &client,
                "https://motogpcal.com",
                &series.id,
                &motogp_stream_links(),
            )
            .await
            {
                if !events.is_empty() {
                    // Customize sessions and event names for Moto2 and Moto3
                    if self.category == "moto2" {
                        for ev in &mut events {
                            ev.series_id = "moto2".to_string();
                            ev.event_name = ev.event_name.replace("Grand Prix", "Moto2 Grand Prix");
                            ev.sessions = transform_moto2_sessions(&ev.sessions);
                        }
                    } else if self.category == "moto3" {
                        for ev in &mut events {
                            ev.series_id = "moto3".to_string();
                            ev.event_name = ev.event_name.replace("Grand Prix", "Moto3 Grand Prix");
                            ev.sessions = transform_moto3_sessions(&ev.sessions);
                        }
                    }

                    if !events
                        .iter()
                        .any(|e| e.start_date.year() == 2027 || e.end_date.year() == 2027)
                    {
                        events.extend(get_official_2027_motogp_schedule(&series.id));
                    }

                    return Ok(events);
                }
            }

            // Step 1: Query all current and upcoming seasons from Pulselive
            let seasons_url = "https://api.motogp.pulselive.com/motogp/v1/results/seasons";
            if let Ok(seasons) =
                fetcher::fetch_json::<Vec<MotoGpSeason>>(&client, seasons_url).await
            {
                let target_seasons: Vec<&MotoGpSeason> = seasons
                    .iter()
                    .filter(|s| s.year >= current_year || s.current)
                    .collect();

                let mut all_events = Vec::new();
                let today = Utc::now().date_naive();

                for season in target_seasons {
                    let events_url = format!(
                        "https://api.motogp.pulselive.com/motogp/v1/results/events?seasonUuid={}",
                        season.id
                    );
                    if let Ok(api_events) =
                        fetcher::fetch_json::<Vec<MotoGpEventResponse>>(&client, &events_url).await
                    {
                        for (_i, ev) in api_events.into_iter().enumerate() {
                            let start_date_str =
                                ev.date_start.as_deref().or(ev.date_end.as_deref());
                            let end_date_str = ev.date_end.as_deref().or(ev.date_start.as_deref());

                            let date_str = end_date_str.or(start_date_str);
                            let race_date = match date_str
                                .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                            {
                                Some(d) => d,
                                None => continue,
                            };

                            let status = if race_date < today {
                                EventStatus::Completed
                            } else if race_date == today {
                                EventStatus::Live
                            } else {
                                EventStatus::Upcoming
                            };

                            let circuit_name = ev
                                .circuit
                                .map(|c| c.name)
                                .unwrap_or_else(|| "Grand Prix Circuit".to_string());
                            let country_name = ev
                                .country
                                .map(|c| c.name)
                                .unwrap_or_else(|| "International".to_string());

                            let sprint_date = race_date.pred_opt().unwrap_or(race_date);
                            let sprint_time = sprint_date.and_hms_opt(13, 0, 0).map(|ndt| {
                                chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)
                            });
                            let gp_time = race_date.and_hms_opt(12, 0, 0).map(|ndt| {
                                chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)
                            });

                            let sessions = vec![
                                Session {
                                    name: "Sprint Race".to_string(),
                                    session_type: SessionType::Sprint,
                                    start_time: sprint_time,
                                    end_time: None,
                                },
                                Session {
                                    name: "Grand Prix".to_string(),
                                    session_type: SessionType::Race,
                                    start_time: gp_time,
                                    end_time: None,
                                },
                            ];

                            all_events.push(RaceEvent {
                                series_id: series.id.clone(),
                                event_name: ev.name,
                                circuit_name,
                                location: country_name.clone(),
                                country: country_name,
                                start_date: sprint_date,
                                end_date: race_date,
                                round: Some((all_events.len() + 1) as u32),
                                sessions,
                                stream_links: motogp_stream_links(),
                                status,
                            });
                        }
                    }
                }

                if !all_events.is_empty() {
                    return Ok(all_events);
                }
            }
        }

        // Fallback schedule
        Ok(get_official_2026_motogp_schedule(&series.id))
    }
}

fn motogp_stream_links() -> Vec<StreamLink> {
    vec![
        StreamLink {
            platform: "MotoGP VideoPass".to_string(),
            url: "https://www.motogp.com/en/videopass".to_string(),
            access: StreamAccess::Paid,
        },
        StreamLink {
            platform: "TNT Sports / Max".to_string(),
            url: "https://www.max.com/sports".to_string(),
            access: StreamAccess::Paid,
        },
    ]
}

fn transform_moto2_sessions(motogp_sessions: &[Session]) -> Vec<Session> {
    let mut sessions = Vec::new();
    for s in motogp_sessions {
        match s.session_type {
            SessionType::Practice => {
                let name = if s.name.contains('1') || s.name.contains("FP1") {
                    "Practice 1"
                } else {
                    "Practice 2"
                };
                sessions.push(Session {
                    name: name.to_string(),
                    session_type: SessionType::Practice,
                    start_time: s.start_time.map(|t| t - chrono::Duration::hours(1)),
                    end_time: None,
                });
            }
            SessionType::Qualifying => {
                sessions.push(Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: s.start_time.map(|t| t - chrono::Duration::minutes(50)),
                    end_time: None,
                });
            }
            SessionType::Race => {
                sessions.push(Session {
                    name: "Moto2 Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: s.start_time.map(|t| t - chrono::Duration::minutes(105)),
                    end_time: s.start_time.map(|t| t + chrono::Duration::minutes(15)),
                });
            }
            _ => {}
        }
    }
    if sessions.is_empty() {
        motogp_sessions.to_vec()
    } else {
        sessions
    }
}

fn transform_moto3_sessions(motogp_sessions: &[Session]) -> Vec<Session> {
    let mut sessions = Vec::new();
    for s in motogp_sessions {
        match s.session_type {
            SessionType::Practice => {
                let name = if s.name.contains('1') || s.name.contains("FP1") {
                    "Practice 1"
                } else {
                    "Practice 2"
                };
                sessions.push(Session {
                    name: name.to_string(),
                    session_type: SessionType::Practice,
                    start_time: s.start_time.map(|t| t - chrono::Duration::hours(2)),
                    end_time: None,
                });
            }
            SessionType::Qualifying => {
                sessions.push(Session {
                    name: "Qualifying".to_string(),
                    session_type: SessionType::Qualifying,
                    start_time: s.start_time.map(|t| t - chrono::Duration::minutes(100)),
                    end_time: None,
                });
            }
            SessionType::Race => {
                sessions.push(Session {
                    name: "Moto3 Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: s.start_time.map(|t| t - chrono::Duration::hours(3)),
                    end_time: s.start_time.map(|t| t - chrono::Duration::hours(1)),
                });
            }
            _ => {}
        }
    }
    if sessions.is_empty() {
        motogp_sessions.to_vec()
    } else {
        sessions
    }
}

pub fn get_official_2026_motogp_schedule(series_id: &str) -> Vec<RaceEvent> {
    let today = Utc::now().date_naive();

    let raw_events = vec![
        (
            "Thai Grand Prix",
            "Chang International Circuit",
            "Buriram",
            "Thailand",
            (2026, 3, 1),
        ),
        (
            "Brazilian Grand Prix",
            "Autódromo Internacional de Goiânia",
            "Goiânia",
            "Brazil",
            (2026, 3, 22),
        ),
        (
            "Grand Prix of the Americas",
            "Circuit of The Americas",
            "Austin, Texas",
            "USA",
            (2026, 3, 29),
        ),
        (
            "Qatar Grand Prix",
            "Lusail International Circuit",
            "Lusail",
            "Qatar",
            (2026, 4, 12),
        ),
        (
            "Spanish Grand Prix",
            "Circuito de Jerez",
            "Jerez",
            "Spain",
            (2026, 4, 26),
        ),
        (
            "French Grand Prix",
            "Bugatti Circuit (Le Mans)",
            "Le Mans",
            "France",
            (2026, 5, 10),
        ),
        (
            "British Grand Prix",
            "Silverstone Circuit",
            "Silverstone",
            "UK",
            (2026, 5, 24),
        ),
        (
            "Italian Grand Prix",
            "Autodromo Internazionale del Mugello",
            "Scarperia",
            "Italy",
            (2026, 6, 7),
        ),
        (
            "Dutch TT",
            "TT Circuit Assen",
            "Assen",
            "Netherlands",
            (2026, 6, 28),
        ),
        (
            "German Grand Prix",
            "Sachsenring",
            "Hohenstein-Ernstthal",
            "Germany",
            (2026, 7, 12),
        ),
        (
            "Czech Grand Prix",
            "Automotodrom Brno",
            "Brno",
            "Czech Republic",
            (2026, 7, 19),
        ),
        (
            "Austrian Grand Prix",
            "Red Bull Ring",
            "Spielberg",
            "Austria",
            (2026, 8, 16),
        ),
        (
            "Hungarian Grand Prix",
            "Balaton Park Circuit",
            "Balatonfőkajár",
            "Hungary",
            (2026, 8, 23),
        ),
        (
            "Catalan Grand Prix",
            "Circuit de Barcelona-Catalunya",
            "Montmeló",
            "Spain",
            (2026, 9, 6),
        ),
        (
            "San Marino Grand Prix",
            "Misano World Circuit Marco Simoncelli",
            "Misano",
            "Italy",
            (2026, 9, 13),
        ),
        (
            "Japanese Grand Prix",
            "Mobility Resort Motegi",
            "Motegi",
            "Japan",
            (2026, 9, 27),
        ),
        (
            "Indonesian Grand Prix",
            "Mandalika International Circuit",
            "Lombok",
            "Indonesia",
            (2026, 10, 4),
        ),
        (
            "Australian Grand Prix",
            "Phillip Island Grand Prix Circuit",
            "Ventnor",
            "Australia",
            (2026, 10, 18),
        ),
        (
            "Malaysian Grand Prix",
            "Petronas Sepang International Circuit",
            "Sepang",
            "Malaysia",
            (2026, 10, 25),
        ),
        (
            "Portuguese Grand Prix",
            "Autódromo Internacional do Algarve",
            "Portimão",
            "Portugal",
            (2026, 11, 8),
        ),
        (
            "Valencia Grand Prix",
            "Circuit Ricardo Tormo",
            "Cheste",
            "Spain",
            (2026, 11, 15),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (name, circuit, loc, country, date))| {
            let race_date = NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap();
            let sprint_date = race_date.pred_opt().unwrap_or(race_date);
            let sprint_time = sprint_date
                .and_hms_opt(13, 0, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let gp_time = race_date
                .and_hms_opt(12, 0, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            let status = if race_date < today {
                EventStatus::Completed
            } else if race_date == today {
                EventStatus::Live
            } else {
                EventStatus::Upcoming
            };

            let sessions = vec![
                Session {
                    name: "Sprint Race".to_string(),
                    session_type: SessionType::Sprint,
                    start_time: sprint_time,
                    end_time: None,
                },
                Session {
                    name: "Grand Prix Race".to_string(),
                    session_type: SessionType::Race,
                    start_time: gp_time,
                    end_time: None,
                },
            ];

            RaceEvent {
                series_id: series_id.to_string(),
                event_name: name.to_string(),
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: race_date,
                end_date: race_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: motogp_stream_links(),
                status,
            }
        })
        .collect()
}

pub fn get_official_2027_motogp_schedule(series_id: &str) -> Vec<RaceEvent> {
    let raw_events = vec![
        (
            "Thai Grand Prix",
            "Chang International Circuit",
            "Buriram",
            "Thailand",
            (2027, 2, 28),
        ),
        (
            "Brazilian Grand Prix",
            "Autódromo Internacional de Goiânia",
            "Goiânia",
            "Brazil",
            (2027, 3, 21),
        ),
        (
            "Grand Prix of the Americas",
            "Circuit of The Americas",
            "Austin, Texas",
            "USA",
            (2027, 3, 28),
        ),
        (
            "Qatar Grand Prix",
            "Lusail International Circuit",
            "Lusail",
            "Qatar",
            (2027, 4, 11),
        ),
        (
            "Spanish Grand Prix",
            "Circuito de Jerez",
            "Jerez",
            "Spain",
            (2027, 4, 25),
        ),
        (
            "French Grand Prix",
            "Bugatti Circuit (Le Mans)",
            "Le Mans",
            "France",
            (2027, 5, 9),
        ),
        (
            "British Grand Prix",
            "Silverstone Circuit",
            "Silverstone",
            "UK",
            (2027, 5, 23),
        ),
        (
            "Italian Grand Prix",
            "Autodromo Internazionale del Mugello",
            "Scarperia",
            "Italy",
            (2027, 6, 6),
        ),
        (
            "Dutch TT",
            "TT Circuit Assen",
            "Assen",
            "Netherlands",
            (2027, 6, 27),
        ),
        (
            "German Grand Prix",
            "Sachsenring",
            "Hohenstein-Ernstthal",
            "Germany",
            (2027, 7, 11),
        ),
        (
            "Czech Grand Prix",
            "Automotodrom Brno",
            "Brno",
            "Czech Republic",
            (2027, 7, 18),
        ),
        (
            "Austrian Grand Prix",
            "Red Bull Ring",
            "Spielberg",
            "Austria",
            (2027, 8, 15),
        ),
        (
            "Hungarian Grand Prix",
            "Balaton Park Circuit",
            "Balatonfőkajár",
            "Hungary",
            (2027, 8, 22),
        ),
        (
            "Catalan Grand Prix",
            "Circuit de Barcelona-Catalunya",
            "Montmeló",
            "Spain",
            (2027, 9, 5),
        ),
        (
            "San Marino Grand Prix",
            "Misano World Circuit Marco Simoncelli",
            "Misano",
            "Italy",
            (2027, 9, 12),
        ),
        (
            "Japanese Grand Prix",
            "Mobility Resort Motegi",
            "Motegi",
            "Japan",
            (2027, 9, 26),
        ),
        (
            "Indonesian Grand Prix",
            "Mandalika International Circuit",
            "Lombok",
            "Indonesia",
            (2027, 10, 3),
        ),
        (
            "Australian Grand Prix",
            "Phillip Island Grand Prix Circuit",
            "Ventnor",
            "Australia",
            (2027, 10, 17),
        ),
        (
            "Malaysian Grand Prix",
            "Petronas Sepang International Circuit",
            "Sepang",
            "Malaysia",
            (2027, 10, 24),
        ),
        (
            "Portuguese Grand Prix",
            "Autódromo Internacional do Algarve",
            "Portimão",
            "Portugal",
            (2027, 11, 7),
        ),
        (
            "Valencia Grand Prix",
            "Circuit Ricardo Tormo",
            "Cheste",
            "Spain",
            (2027, 11, 14),
        ),
    ];

    raw_events
        .into_iter()
        .enumerate()
        .map(|(i, (name, circuit, loc, country, date))| {
            let race_date = NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap();
            let sprint_date = race_date.pred_opt().unwrap_or(race_date);

            let sprint_time = sprint_date
                .and_hms_opt(13, 0, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            let gp_time = race_date
                .and_hms_opt(12, 0, 0)
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            let sessions = match series_id {
                "moto2" => vec![
                    Session {
                        name: "Qualifying".to_string(),
                        session_type: SessionType::Qualifying,
                        start_time: sprint_date.and_hms_opt(13, 45, 0).map(|ndt| {
                            chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)
                        }),
                        end_time: None,
                    },
                    Session {
                        name: "Moto2 Race".to_string(),
                        session_type: SessionType::Race,
                        start_time: race_date.and_hms_opt(10, 15, 0).map(|ndt| {
                            chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)
                        }),
                        end_time: None,
                    },
                ],
                "moto3" => vec![
                    Session {
                        name: "Qualifying".to_string(),
                        session_type: SessionType::Qualifying,
                        start_time: sprint_date.and_hms_opt(12, 50, 0).map(|ndt| {
                            chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)
                        }),
                        end_time: None,
                    },
                    Session {
                        name: "Moto3 Race".to_string(),
                        session_type: SessionType::Race,
                        start_time: race_date.and_hms_opt(9, 0, 0).map(|ndt| {
                            chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)
                        }),
                        end_time: None,
                    },
                ],
                _ => vec![
                    Session {
                        name: "Sprint Race".to_string(),
                        session_type: SessionType::Sprint,
                        start_time: sprint_time,
                        end_time: None,
                    },
                    Session {
                        name: "Grand Prix Race".to_string(),
                        session_type: SessionType::Race,
                        start_time: gp_time,
                        end_time: None,
                    },
                ],
            };

            let event_name = match series_id {
                "moto2" => name.replace("Grand Prix", "Moto2 Grand Prix"),
                "moto3" => name.replace("Grand Prix", "Moto3 Grand Prix"),
                _ => name.to_string(),
            };

            RaceEvent {
                series_id: series_id.to_string(),
                event_name,
                circuit_name: circuit.to_string(),
                location: loc.to_string(),
                country: country.to_string(),
                start_date: sprint_date,
                end_date: race_date,
                round: Some((i + 1) as u32),
                sessions,
                stream_links: motogp_stream_links(),
                status: EventStatus::Upcoming,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_official_motogp_schedule() {
        let events = get_official_2026_motogp_schedule("motogp");
        assert_eq!(events.len(), 21);
        assert_eq!(events[0].event_name, "Thai Grand Prix");
        assert_eq!(events[0].round, Some(1));
        assert_eq!(events[20].event_name, "Valencia Grand Prix");
        assert_eq!(events[20].round, Some(21));
    }

    #[test]
    fn test_parse_motogp_standings() {
        let sample = r#"{
            "classification": [
                {
                    "position": 1,
                    "rider": {
                        "full_name": "Marc Marquez",
                        "number": 93
                    },
                    "team": {
                        "name": "Ducati Lenovo Team"
                    },
                    "constructor": {
                        "name": "Ducati"
                    },
                    "points": 545.0,
                    "race_wins": 11
                },
                {
                    "position": 2,
                    "rider": {
                        "full_name": "Jorge Martin",
                        "number": 89
                    },
                    "team": {
                        "name": "Aprilia Racing"
                    },
                    "constructor": {
                        "name": "Aprilia"
                    },
                    "points": 508.0,
                    "race_wins": 3
                }
            ]
        }"#;

        let standings = parse_motogp_standings(sample, "motogp", 2026).unwrap();
        assert_eq!(standings.series_id, "motogp");
        assert_eq!(standings.season, 2026);
        assert_eq!(standings.drivers.len(), 2);
        assert_eq!(standings.drivers[0].driver_name, "Marc Marquez");
        assert_eq!(standings.drivers[0].driver_number, Some(93));
        assert_eq!(standings.drivers[0].points, 545.0);
        assert_eq!(standings.drivers[0].wins, 11);
        assert_eq!(standings.constructors.len(), 2);
        assert_eq!(standings.constructors[0].name, "Ducati");
    }
}
