use std::time::Duration;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    text::Line,
    widgets::{Paragraph, Widget, Wrap},
};

use crate::models::{Lyrics, LyricsContent};

pub struct LyricsView<'a> {
    pub lyrics: &'a Lyrics,
    pub playback_pos: Duration,
}

impl<'a> Widget for LyricsView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match &self.lyrics.content {
            LyricsContent::Synced(lines) => {
                let mut rendered_lines = Vec::new();
                
                let active_index = lines
                    .iter()
                    .position(|line| line.start_time > self.playback_pos)
                    .map(|idx| idx.saturating_sub(1))
                    .unwrap_or(lines.len().saturating_sub(1));

                for (i, line) in lines.iter().enumerate() {
                    let total_secs = line.start_time.as_secs();
                    let mins = total_secs / 60;
                    let secs = total_secs % 60;
                    let timestamp = format!("[{:02}:{:02}] ", mins, secs);

                    let mut spans = vec![timestamp.dark_gray()];

                    if i == active_index {
                        spans.push(line.text.as_str().cyan().bold());
                    } else if i < active_index {
                        spans.push(line.text.as_str().dark_gray());
                    } else {
                        spans.push(line.text.as_str().white());
                    }

                    rendered_lines.push(Line::from(spans));
                }
                Paragraph::new(rendered_lines)
                    .wrap(Wrap { trim: false })
                    .centered()
                    .render(area, buf);
            }
            LyricsContent::Unsynced(lines) => {
                let spans: Vec<Line> = lines.iter()
                    .map(|s| Line::from(s.as_str()))
                    .collect();
                Paragraph::new(spans)
                    .wrap(Wrap { trim: false })
                    .centered()
                    .render(area, buf);
            }
        }
    }
}