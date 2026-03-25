use serde::Deserialize;

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

pub async fn get_romanized_lyrics(track_name: &str, artist_name: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client.get("https://lrclib.net/api/get")
        .query(&[("artist_name", artist_name), ("track_name", track_name)])
        .send()
        .await?
        .error_for_status()?;

    let lyrics_data = response.json::<LrcResponse>().await?;
    
    let raw_lyrics = lyrics_data.synced_lyrics
        .or(lyrics_data.plain_lyrics)
        .ok_or("No lyrics found")?;

    // Remove timestamps
    let re = Regex::new(r"\[\d{2}:\d{2}\.\d{2}\]")?;
    let cleaned_lyrics = re.replace_all(&raw_lyrics, "");

    let romanized_block = kakasi::convert(cleaned_lyrics.as_ref()).romaji;

    // Split into lines
    let lines = romanized_block
        .lines()
        .map(|s| s.trim().to_string())
        .collect();

    Ok(lines)
}