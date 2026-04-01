use std::time::Duration;

use color_eyre::{Result, eyre::{eyre, WrapErr}};

use windows::Media::Control::{GlobalSystemMediaTransportControlsSessionManager as WSessionManager, GlobalSystemMediaTransportControlsSession as WMediaSession};

use crate::{media::MediaSource, models::{PlaybackStatus, Track}};

pub struct WindowsMediaSource {
    session_manager: WSessionManager,
    spotify_session: Option<WMediaSession>
}

impl WindowsMediaSource {
    pub async fn new() -> Result<Self> {
        let session_manager = WSessionManager::RequestAsync()?.await?;
        let mut this = Self { session_manager, spotify_session: None };
        this.refresh().await?;

        Ok(this)
    }

    fn extract_session(&self) -> Result<&WMediaSession> {
        self.spotify_session
            .as_ref()
            .ok_or_else(|| eyre!("spotify session not active"))
    }
}

impl MediaSource for WindowsMediaSource {
    async fn refresh(&mut self) -> Result<()> {
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
    
    async fn current_track(&self) -> Result<Track> {
        let session = self.extract_session()?;

        let media_properties = session
            .TryGetMediaPropertiesAsync()
            .wrap_err("failed to get media properties async")?
            .await
            .wrap_err("failed to await media properties")?;

        Ok(Track::new(
            media_properties.Artist().wrap_err("failed to get artist")?.to_string(),
            media_properties.Title().wrap_err("failed to get title")?.to_string()
        ))
    }
    
    async fn current_playback_position(&self) -> Result<Duration> {
        let session = self.extract_session()?;

        let timeline = session
            .GetTimelineProperties()
            .wrap_err("failed to get timeline properties")?;

        Ok(Duration::from_millis((timeline.Position().wrap_err("failed to get position")?.Duration / 10_000) as u64))
    }
    
    async fn current_playback_status(&self) -> Result<PlaybackStatus> {
        let session = self.extract_session()?;

        Ok(PlaybackStatus::from(session.GetPlaybackInfo()?.PlaybackStatus()?))
    }
}