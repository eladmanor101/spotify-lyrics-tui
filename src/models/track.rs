use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub artist: String,
    pub title: String
}

impl Track {
    pub fn new(artist: impl Into<String>, title: impl Into<String>) -> Self {
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