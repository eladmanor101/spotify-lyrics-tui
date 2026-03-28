use color_eyre::{Result, eyre::{eyre, WrapErr}};

use windows::Media::Control::{GlobalSystemMediaTransportControlsSessionManager, GlobalSystemMediaTransportControlsSession};

pub struct MediaManager {
    session_manager: GlobalSystemMediaTransportControlsSessionManager,
    spotify_session: Option<GlobalSystemMediaTransportControlsSession>
}

impl MediaManager {
    pub async fn new() -> Result<Self> {
        let session_manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.await?;

        Ok(Self {
            session_manager,
            spotify_session: None
        })
    }

    pub async fn refresh_session(&mut self) -> Result<()> {
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

    pub async fn media_properties(&self) -> Result<(String, String)> {
        let session = self.spotify_session
            .as_ref()
            .ok_or_else(|| eyre!("spotify session not active"))?;

        let media_properties = session
            .TryGetMediaPropertiesAsync()
            .wrap_err("failed to get media properties async")?
            .await
            .wrap_err("failed to await media properties")?;

        Ok((
            media_properties.Artist().wrap_err("failed to get artist")?.to_string(),
            media_properties.Title().wrap_err("failed to get title")?.to_string(),
        ))
    }

    #[allow(dead_code)]
    pub async fn timeline_position(&self) -> Result<i64> {
        let session = self.spotify_session
            .as_ref()
            .ok_or_else(|| eyre!("spotify session not active"))?;

        let timeline = session
            .GetTimelineProperties()
            .wrap_err("failed to get timeline properties")?;

        Ok(timeline.Position().wrap_err("failed to get position")?.Duration / 10_000_000)
    }
}