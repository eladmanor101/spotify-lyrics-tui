use std::error::Error;

use windows::Media::Control::{GlobalSystemMediaTransportControlsSessionManager, GlobalSystemMediaTransportControlsSession};

pub struct MediaManager {
    session_manager: GlobalSystemMediaTransportControlsSessionManager,
    spotify_session: Option<GlobalSystemMediaTransportControlsSession>
}

impl MediaManager {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let session_manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.await?;

        Ok(Self {
            session_manager,
            spotify_session: None
        })
    }

    pub async fn refresh_session(&mut self) -> Result<(), Box<dyn Error>> {
        self.spotify_session = self.session_manager
            .GetSessions()?
            .into_iter()
            .find(|s| {
                match s.SourceAppUserModelId() {
                    Ok(s) => s.to_string().contains("Spotify"),
                    Err(_) => false
                }
            });
        
        Ok(())
    }

    pub async fn media_properties(&self) -> Result<(String, String), Box<dyn Error>> {
        let media_properties = self.spotify_session
            .as_ref()
            .ok_or("Spotify session not active 1")?
            .TryGetMediaPropertiesAsync()?.await?;

        Ok((media_properties.Artist()?.to_string(), media_properties.Title()?.to_string()))
    }

    #[allow(dead_code)]
    pub async fn timeline_position(&self) -> Result<i64, Box<dyn Error>> {
        let timeline = self.spotify_session
            .as_ref()
            .ok_or("Spotify session not active 2")?
            .GetTimelineProperties()?;

        Ok(timeline.Position()?.Duration / 10_000_000)
    }
}