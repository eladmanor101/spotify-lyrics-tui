use serde::Deserialize;
use color_eyre::{Result, eyre::{eyre, WrapErr}};

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

use regex::Regex;

use crate::models::Track;

pub async fn get_romanized_lyrics(track: Track) -> Result<Vec<String>> {
    let client = reqwest::Client::new();
    
    let response = client
        .get("https://lrclib.net/api/get")
        .query(&[("artist_name", track.artist.to_owned()), ("track_name", track.title.to_owned())])
        .send()
        .await
        .wrap_err("failed to send request to lrclib")?
        .error_for_status()
        .wrap_err("lrclib returned an error status")?;

    let lyrics_data = response
        .json::<LrcResponse>()
        .await
        .wrap_err("failed to parse lrclib response")?;

    let raw_lyrics = lyrics_data.synced_lyrics
        .or(lyrics_data.plain_lyrics)
        .ok_or_else(|| eyre!("no lyrics found for '{}' by '{}'", track.title, track.artist))?;

    Ok(process_lyrics(&raw_lyrics))
}

fn process_lyrics(raw: &str) -> Vec<String> {
    let timestamp_re = Regex::new(r"\[\d{2}:\d{2}\.\d{2}\]").unwrap();
    let metadata_re = Regex::new(r"^\[.*\]$").unwrap();

    raw.lines()
        .filter(|line| !metadata_re.is_match(line.trim()))
        .map(|line| {
            let cleaned = timestamp_re.replace_all(line, "");
            let trimmed = cleaned.trim();
            kakasi::convert(trimmed).romaji
        })
        .collect()
}