use std::time::Duration;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::Stylize,
    text::Line,
    widgets::{Paragraph, Widget, Wrap},
};

use crate::models::{Lyrics, LyricsContent, SyncLine};

pub struct LyricsView<'a> {
    pub lyrics: &'a Lyrics,
    pub playback_pos: Duration
}

impl<'a> LyricsView<'a> {
    pub fn new(lyrics: &'a Lyrics, playback_pos: Duration) -> Self {
        Self {
            lyrics,
            playback_pos
        }
    }
}

impl LyricsView<'_> {
    fn active_line_index(&self, lines: &Vec<SyncLine>) -> usize {
        lines
            .iter()
            .position(|line| line.start_time > self.playback_pos + Duration::from_millis(1000))
            .map(|idx| idx.saturating_sub(1))
            .unwrap_or(lines.len().saturating_sub(1))
    }

    fn layout(&self, area: Rect) -> [Rect; 3] {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),
                Constraint::Percentage(60),
                Constraint::Min(0)
            ])
            .flex(Flex::Center)
            .areas(area)
    }

    fn left_area(&self, area: Rect) -> Rect {
        self.layout(area)[0]
    }

    fn lyrics_area(&self, area: Rect) -> Rect {
        self.layout(area)[1]
    }

    fn right_area(&self, area: Rect) -> Rect {
        self.layout(area)[2]
    }

    fn lines_in_view(&self, total_lines: usize, active_index: usize, area_height: u16) -> (usize, usize) {
        let area_height = area_height as usize;
        let ideal_start = active_index.saturating_sub(area_height / 2);

        let max_start = total_lines.saturating_sub(area_height);

        let start = ideal_start.min(max_start);
        let end = (start + area_height).min(total_lines);

        (start, end)
    }
}

impl<'a> Widget for LyricsView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match &self.lyrics.content {
            LyricsContent::Synced(lines) => {
                let mut rendered_lines = Vec::new();
                let mut rendered_timestamps = Vec::new();
                
                let active_index = self.active_line_index(lines);
                let (start, end) = self.lines_in_view(lines.len(), active_index, area.height);

                for i in start..end {
                    let line = &lines[i];

                    let rendered_line = if i == active_index {
                        line.text.as_str().cyan().bold()
                    } else if i < active_index {
                        line.text.as_str().dark_gray()
                    } else {
                        line.text.as_str().white()
                    };

                    rendered_lines.push(Line::from(rendered_line));

                    let total_secs = line.start_time.as_secs();
                    let mins = total_secs / 60;
                    let secs = total_secs % 60;
                    let timestamp = format!("[{:02}:{:02}] ", mins, secs);

                    rendered_timestamps.push(Line::from(timestamp.dark_gray()));
                }

                Paragraph::new(rendered_lines)
                    .centered()
                    .render(self.lyrics_area(area), buf);

                Paragraph::new(rendered_timestamps)
                    .right_aligned()
                    .render(self.left_area(area), buf);
            }
            LyricsContent::Unsynced(lines) => {
                let spans: Vec<Line> = lines.iter()
                    .map(|s| Line::from(s.as_str()))
                    .collect();
                Paragraph::new(spans)
                    .wrap(Wrap { trim: false })
                    .centered()
                    .render(self.lyrics_area(area), buf);
            }
        }
    }
}