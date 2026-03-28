#[derive(Clone)]
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