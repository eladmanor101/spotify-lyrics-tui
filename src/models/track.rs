use std::{fmt::Display, sync::Arc};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub artist: Arc<str>,
    pub title: Arc<str>
}

impl Track {
    pub fn new(artist: String, title: String) -> Self {
        Track {
            artist: artist.into(),
            title: title.into()
        }
    }
}

impl Display for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.artist, self.title)
    }
}