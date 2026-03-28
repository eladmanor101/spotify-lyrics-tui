use crate::models::Track;

#[derive(Clone)]
pub struct Lyrics {
    pub track: Track,
    pub text: Vec<String>
}

impl Lyrics {
    pub fn new(track: Track, text: Vec<String>) -> Lyrics {
        Lyrics { track, text }
    }
}