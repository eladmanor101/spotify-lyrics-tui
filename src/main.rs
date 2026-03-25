mod lyrics;
mod media_manager;

use std::time::Duration;

use crossterm::event::{self, KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Margin,
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph, Wrap}
};
use tokio::sync::mpsc;

use crate::{lyrics::get_romanized_lyrics, media_manager::MediaManager};

enum Event {
    Key(KeyEvent),
    Tick
}

enum Action {
    FetchLyrics { artist: String, title: String },
    DisplayLyrics(Vec<String>),
    Quit
}

#[derive(Default)]
pub struct App {
    lyrics: Vec<String>,
    exit: bool
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    color_eyre::install()?;

    let mut terminal = ratatui::init();

    let mut app = App::default();

    let (event_tx, mut event_rx) = mpsc::channel(32);
    let (action_tx, mut action_rx) = mpsc::channel(32);
    
    let tx = event_tx.clone();
    tokio::spawn(async move {
        loop {
            if event::poll(Duration::from_millis(16)).unwrap() {
                if let event::Event::Key(key) = event::read().unwrap() {
                    let _ = tx.send(Event::Key(key)).await;
                }
            }
            let _ = tx.send(Event::Tick).await;
        }
    });

    let tx = action_tx.clone();
    tokio::spawn(async move {
        let mut last_title = String::new();

        loop {
            let media_result = async {
                let mut media = MediaManager::new().await
                    .map_err(|e| e.to_string())?; 
                media.refresh_session().await.map_err(|e| e.to_string())?;
                media.media_properties().await
                    .map_err(|e| e.to_string())
            }.await;

            match media_result {
                Ok((new_artist, new_title)) => {
                    if new_title != last_title {
                        last_title = new_title.clone();
                        let _ = tx.send(Action::FetchLyrics {
                            artist: new_artist,
                            title: new_title,
                        }).await;
                    }
                }
                Err(e) => {
                    eprintln!("Media Sync Error: {e}");
                }
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    while !app.exit {
        terminal.draw(|frame| render(frame, &app))?;

        if let Ok(event) = event_rx.try_recv() {
            match event {
                Event::Key(k) => match k.code {
                    KeyCode::Char('q') => action_tx.send(Action::Quit).await?,
                    KeyCode::Char('r') => {
                        action_tx.send(Action::FetchLyrics {
                            artist: "Kobo Kanaeru".to_string(),
                            title: "HELP!!".to_string()
                        }).await?;
                    }
                    _ => {}
                },
                Event::Tick => {}
            }
        }

        if let Ok(action) = action_rx.try_recv() {
            match action {
                Action::FetchLyrics { artist, title } => {
                    let tx = action_tx.clone();
                    tokio::spawn(async move {
                        let lyrics =  match get_romanized_lyrics(&title, &artist).await {
                            Ok(lyrics) => lyrics,
                            Err(error) => vec![format!("Error fetching lyrics: {error}")]
                        };
                        let _ = tx.send(Action::DisplayLyrics(lyrics)).await;
                    });
                }
                Action::DisplayLyrics(lyrics) => {
                    app.lyrics = lyrics
                }
                Action::Quit => app.exit = true,
            }
        }
    }

    ratatui::restore();
    Ok(())
}

fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    
    let title = Line::from("Romanji Lyrics".bold());
    let keybinds = Line::from(vec![
        " Quit ".into(),
        "<Q> ".blue().bold(),
    ]);

    let block = Block::bordered()
        .title(title.centered())
        .title_bottom(keybinds.centered())
        .border_set(ratatui::symbols::border::THICK);

    let inner_area = block.inner(area);
    
    frame.render_widget(block, area);

    let lyrics_area = inner_area.inner(Margin::new(inner_area.width / 4, 1));
    
    let lyrics_lines = app.lyrics
        .iter()
        .map(|s| Line::from(s.as_str()))
        .collect::<Vec<Line>>();

    let p = Paragraph::new(lyrics_lines)
        .wrap(Wrap { trim: false })
        .centered();

    frame.render_widget(p, lyrics_area);
}