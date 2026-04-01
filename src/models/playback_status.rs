use std::fmt::Display;

use windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus as WindowsPlaybackStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackStatus {
    #[default]
    Closed,
    Opened,
    Changing,
    Stopped,
    Playing,
    Paused
}

impl From<WindowsPlaybackStatus> for PlaybackStatus {
    fn from(value: WindowsPlaybackStatus) -> Self {
        match value {
            WindowsPlaybackStatus::Closed => PlaybackStatus::Closed,
            WindowsPlaybackStatus::Opened => PlaybackStatus::Opened,
            WindowsPlaybackStatus::Changing => PlaybackStatus::Changing,
            WindowsPlaybackStatus::Stopped => PlaybackStatus::Stopped,
            WindowsPlaybackStatus::Playing => PlaybackStatus::Playing,
            WindowsPlaybackStatus::Paused => PlaybackStatus::Paused,
            _ => PlaybackStatus::Closed
        }
    }
}

impl Display for PlaybackStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}