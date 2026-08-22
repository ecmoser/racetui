use crate::data::models::{StreamAccess, StreamLink};

/// Discover and provide live streaming / broadcast links (curated & fallback) inspired by raceday.watch.
pub fn get_raceday_stream_links(series_id: &str, event_name: &str) -> Vec<StreamLink> {
    let mut links = Vec::new();
    let lower_name = event_name.to_lowercase();

    match series_id {
        "elms" => {
            links.push(StreamLink {
                platform: "YouTube (European Le Mans Series Official - Free Live)".to_string(),
                url: "https://www.youtube.com/@EuropeanLeMansSeriesOfficial/streams".to_string(),
                access: StreamAccess::Free,
            });
            links.push(StreamLink {
                platform: "ELMS TV".to_string(),
                url: "https://www.europeanlemansseries.com/live".to_string(),
                access: StreamAccess::Free,
            });
            links.push(StreamLink {
                platform: "raceday.watch (Broadcast Schedule)".to_string(),
                url: "https://raceday.watch/".to_string(),
                access: StreamAccess::Free,
            });
        }
        "aslms" => {
            links.push(StreamLink {
                platform: "YouTube (Asian Le Mans Series Official - Free Live)".to_string(),
                url: "https://www.youtube.com/@AsianLeMansSeries/streams".to_string(),
                access: StreamAccess::Free,
            });
            links.push(StreamLink {
                platform: "Asian Le Mans Series Official Live".to_string(),
                url: "https://www.asianlemansseries.com/live".to_string(),
                access: StreamAccess::Free,
            });
        }
        "wec" => {
            links.push(StreamLink {
                platform: "FIAWEC.tv (Live Stream & Onboard)".to_string(),
                url: "https://fiawec.tv/".to_string(),
                access: StreamAccess::Paid,
            });
            links.push(StreamLink {
                platform: "Max / MotorTrend (US Live Broadcast)".to_string(),
                url: "https://www.max.com/sports".to_string(),
                access: StreamAccess::Paid,
            });
            links.push(StreamLink {
                platform: "Eurosport / Discovery+ (Europe)".to_string(),
                url: "https://www.eurosport.com/".to_string(),
                access: StreamAccess::Paid,
            });
            if lower_name.contains("le mans") {
                links.push(StreamLink {
                    platform: "24 Hours of Le Mans Official App/Web".to_string(),
                    url: "https://www.24h-lemans.com/en/live".to_string(),
                    access: StreamAccess::Paid,
                });
            }
        }
        "gtwc_eu" | "gtwc_am" | "igtc" => {
            links.push(StreamLink {
                platform: "YouTube (GTWorld Official - Free Live GT3 Racing)".to_string(),
                url: "https://www.youtube.com/@GTWorld/streams".to_string(),
                access: StreamAccess::Free,
            });
            links.push(StreamLink {
                platform: "SRO Motorsports Live".to_string(),
                url: "https://www.gt-world-challenge-europe.com/watch-live".to_string(),
                access: StreamAccess::Free,
            });
            links.push(StreamLink {
                platform: "Twitch (SRO Motorsports)".to_string(),
                url: "https://www.twitch.tv/sromotorsports".to_string(),
                access: StreamAccess::Free,
            });
        }
        "imsa" => {
            links.push(StreamLink {
                platform: "IMSA.tv (Free International Live Stream)".to_string(),
                url: "https://www.imsa.com/tvlive/".to_string(),
                access: StreamAccess::Free,
            });
            links.push(StreamLink {
                platform: "Peacock (US Live Broadcast)".to_string(),
                url: "https://www.peacocktv.com/sports".to_string(),
                access: StreamAccess::Paid,
            });
            links.push(StreamLink {
                platform: "YouTube (IMSA Official Replays & Highlights)".to_string(),
                url: "https://www.youtube.com/@imsaofficial".to_string(),
                access: StreamAccess::Free,
            });
        }
        "f1" => {
            links.push(StreamLink {
                platform: "F1 TV Pro (Live Timing, Onboards & Stream)".to_string(),
                url: "https://f1tv.formula1.com/".to_string(),
                access: StreamAccess::Paid,
            });
            links.push(StreamLink {
                platform: "ESPN / Sky Sports F1".to_string(),
                url: "https://www.espn.com/f1/".to_string(),
                access: StreamAccess::Paid,
            });
        }
        "indycar" => {
            links.push(StreamLink {
                platform: "IndyCar Live (International Stream)".to_string(),
                url: "https://www.indycarlive.com/".to_string(),
                access: StreamAccess::Paid,
            });
            links.push(StreamLink {
                platform: "Peacock / FOX Sports (US Live Broadcast)".to_string(),
                url: "https://www.peacocktv.com/sports".to_string(),
                access: StreamAccess::Paid,
            });
        }
        "worldsbk" => {
            links.push(StreamLink {
                platform: "WorldSBK VideoPass".to_string(),
                url: "https://www.worldsbk.com/en/videopass".to_string(),
                access: StreamAccess::Paid,
            });
        }
        "motogp" => {
            links.push(StreamLink {
                platform: "MotoGP VideoPass".to_string(),
                url: "https://www.motogp.com/en/videopass".to_string(),
                access: StreamAccess::Paid,
            });
        }
        "supercars" => {
            links.push(StreamLink {
                platform: "Supercars SuperView (International Stream)".to_string(),
                url: "https://www.supercars.com/superview/".to_string(),
                access: StreamAccess::Paid,
            });
            links.push(StreamLink {
                platform: "YouTube (Supercars Official)".to_string(),
                url: "https://www.youtube.com/@Supercars".to_string(),
                access: StreamAccess::Free,
            });
        }
        "nls" => {
            links.push(StreamLink {
                platform: "YouTube (Nürburgring Langstrecken-Serie - Free Live)".to_string(),
                url: "https://www.youtube.com/@vlninfo/streams".to_string(),
                access: StreamAccess::Free,
            });
        }
        "dtm" => {
            links.push(StreamLink {
                platform: "YouTube (DTM Official - Free International Live)".to_string(),
                url: "https://www.youtube.com/@DTMofficial/streams".to_string(),
                access: StreamAccess::Free,
            });
        }
        "btcc" => {
            links.push(StreamLink {
                platform: "ITV4 / ITVX (UK Live Stream)".to_string(),
                url: "https://www.itv.com/itvx".to_string(),
                access: StreamAccess::Free,
            });
            links.push(StreamLink {
                platform: "TikTok (@ITVSport BTCC Live Stream)".to_string(),
                url: "https://www.tiktok.com/@itvsport".to_string(),
                access: StreamAccess::Free,
            });
        }
        _ => {
            links.push(StreamLink {
                platform: "raceday.watch (Broadcast Directory)".to_string(),
                url: "https://raceday.watch/".to_string(),
                access: StreamAccess::Free,
            });
        }
    }

    links
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elms_raceday_streams() {
        let links = get_raceday_stream_links("elms", "4 Hours of Barcelona");
        assert!(!links.is_empty());
        assert!(links
            .iter()
            .any(|l| l.platform.contains("YouTube") && l.access == StreamAccess::Free));
    }

    #[test]
    fn test_gtwc_raceday_streams() {
        let links = get_raceday_stream_links("gtwc_eu", "24 Hours of Spa");
        assert!(!links.is_empty());
        assert!(links.iter().any(|l| l.platform.contains("GTWorld")));
    }
}
