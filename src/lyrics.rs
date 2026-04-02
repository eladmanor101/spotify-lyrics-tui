use std::time::Duration;

use serde::Deserialize;
use color_eyre::{Result, eyre::{eyre, WrapErr}};

use regex::Regex;

use crate::models::{LyricsContent, SyncLine, Track};

#[allow(unused)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrcResponse {
    id: u32,
    track_name: String,
    artist_name: String,
    album_name: String,
    duration: f64,
    instrumental: bool,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

pub async fn get_romanized_lyrics(track: Track) -> Result<LyricsContent> {
    let client = reqwest::Client::new();
    
    let response = client
        .get("https://lrclib.net/api/get")
        .query(&[("artist_name", &*track.artist), ("track_name", &*track.title)])
        .send()
        .await
        .wrap_err("failed to send request to lrclib")?
        .error_for_status()
        .wrap_err("lrclib returned an error status")?;

    let lyrics_data = response
        .json::<LrcResponse>()
        .await
        .wrap_err("failed to parse lrclib response")?;

    if let Some(raw_lyrics) = lyrics_data.synced_lyrics {
        Ok(process_lyrics(&raw_lyrics, true))
    } else if let Some(raw_lyrics) = lyrics_data.plain_lyrics {
        tracing::warn!("synced lyrics not found for track {}, fetching plain lyrics instead", track);
        Ok(process_lyrics(&raw_lyrics, false))
    } else {
        Err(eyre!("no lyrics found for '{}' by '{}'", track.title, track.artist))
    }
}

fn process_lyrics(raw: &str, is_sync: bool) -> LyricsContent {
    let metadata_re = Regex::new(r"^\[.*[a-zA-Z].*\]$").unwrap();

    if is_sync {
        let timestamp_re = Regex::new(r"^\[(?<min>\d{2}):(?<sec>\d{2})\.(?<ms>\d{2})\]").unwrap();

        let mut lines = raw.lines()
            .filter_map(|line| {
                let line = line.trim();

                if metadata_re.is_match(line) {
                    return None;
                }

                let caps = timestamp_re.captures(line)?;
                let mins: u64 = caps["min"].parse().ok()?;
                let secs: u64 = caps["sec"].parse().ok()?;
                let ms: u64 = caps["ms"].parse().ok()?;
                let position = Duration::from_mins(mins) + Duration::from_secs(secs) + Duration::from_millis(ms * 10);

                let whole_match = caps.get(0).unwrap();
                let text_part = line[whole_match.end()..].trim();

                Some(SyncLine {
                    start_time: position,
                    text: kakasi::convert(text_part).romaji
                })
            })
            .collect::<Vec<SyncLine>>();
        
        if lines.first().map_or(true, |first| first.start_time > Duration::ZERO) {
            lines.insert(0, SyncLine { start_time: Duration::ZERO, text: String::from("...") });
        }

        LyricsContent::Synced(lines)
    } else {
        LyricsContent::Unsynced(
            raw.lines()
                .filter_map(|line| {
                    let trimmed = line.trim();
                    if metadata_re.is_match(trimmed) {
                        return None;
                    }
                    Some(kakasi::convert(trimmed).romaji)
                })
                .collect()
        )
    }
}