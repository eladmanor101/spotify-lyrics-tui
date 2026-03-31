use std::time::Duration;

use crate::models::Track;

#[derive(Clone)]
pub struct SyncLine {
    pub start_time: Duration,
    pub text: String
}

#[derive(Clone)]
pub enum LyricsContent {
    Synced(Vec<SyncLine>),
    Unsynced(Vec<String>)
}

#[derive(Clone)]
pub struct Lyrics {
    pub track: Track,
    pub content: LyricsContent,
}

impl Lyrics {
    pub fn new(track: Track, content: LyricsContent) -> Lyrics {
        Lyrics { track, content }
    }

    pub fn len(&self) -> usize {
        match &self.content {
            LyricsContent::Synced(sync_lines) => sync_lines.len(),
            LyricsContent::Unsynced(lines) => lines.len()
        }
    }

    #[allow(dead_code)]
    pub fn unsynced_lines(&self) -> Vec<String> {
        match &self.content {
            LyricsContent::Synced(sync_lines) => {
                sync_lines.iter().map(|sync_line| sync_line.text.clone()).collect()
            }
            LyricsContent::Unsynced(lines) => lines.clone()
        }
    }
}