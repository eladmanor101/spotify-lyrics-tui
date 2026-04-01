use std::time::Duration;

use color_eyre::Result;

use crate::models::{PlaybackStatus, Track};

pub trait MediaSource {
    async fn refresh(&mut self) -> Result<()>;

    async fn current_track(&self) -> Result<Track>;
    async fn current_playback_position(&self) -> Result<Duration>;
    async fn current_playback_status(&self) -> Result<PlaybackStatus>;
}