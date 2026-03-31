mod lyrics;
mod media_manager;
mod models;
mod widgets;

use std::{io::Stdout, time::Duration};

use crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    Frame, Terminal, layout::Margin, prelude::CrosstermBackend, style::Stylize, text::Line, widgets::{Block, Paragraph}
};
use tokio::sync::mpsc;
use color_eyre::{Result, eyre::WrapErr};

use crate::{lyrics::get_romanized_lyrics, media_manager::MediaManager, models::{Lyrics, Track}, widgets::LyricsView};

enum Event {
    Key(KeyEvent),
    Tick
}

enum Action {
    TrackChanged(Track),
    FetchLyrics(Track),

    UpdatePlaybackPosition(Duration),

    LyricsFetched(Lyrics),
    LyricsFetchError(String),
    Quit
}

enum SpotifyAction {
    // RefreshTrack,
    // ToggleAutoRefresh
}

#[derive(Default)]
enum LyricsState {
    #[default]
    None,
    Loading,
    Loaded(Lyrics),
    Error(String),
}

#[derive(Default)]
pub struct App {
    tick: usize,

    current_track: Option<Track>,
    track_position: Duration,
    last_api_track_position: Duration,
    lyrics_state: LyricsState,

    auto_refresh: bool,
    exit: bool
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let _guard = setup_tracing()?;

    let mut terminal = ratatui::init();

    let mut app = App::default();

    let (event_tx, event_rx) = mpsc::channel(32);
    let (action_tx, action_rx) = mpsc::channel(32);
    let (spotify_action_tx, spotify_action_rx) = mpsc::channel(16);
    
    let event_handle = tokio::spawn(event_task(event_tx.clone()));
    let media_handle = tokio::spawn(media_task(spotify_action_rx, action_tx.clone()));

    // let local_set = tokio::task::LocalSet::new();
    // let test_handle = local_set.run_until(async { return 1; });

    tokio::select! {
        result = event_handle => result?,
        result = media_handle => result?,
        result = run(&mut terminal, &mut app, event_rx, action_rx, action_tx, spotify_action_tx) => result
    }?;

    ratatui::restore();
    Ok(())
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    mut event_rx: mpsc::Receiver<Event>,
    mut action_rx: mpsc::Receiver<Action>,
    action_tx: mpsc::Sender<Action>,
    _spotify_action_tx: mpsc::Sender<SpotifyAction>
) -> Result<()> {
    while !app.exit {
        terminal.draw(|frame| render(frame, &app))?;

        if let Ok(event) = event_rx.try_recv() {
            match event {
                Event::Key(k) => match k.code {
                    KeyCode::Char('q') => action_tx.send(Action::Quit).await?,
                    KeyCode::Char('r') => {
                        action_tx.send(Action::FetchLyrics(Track::new(
                            "Kobo Kanaeru",
                            "HELP!!"
                        ))).await?;
                    }
                    KeyCode::Char('t') if k.kind == KeyEventKind::Press => {
                        app.auto_refresh = !app.auto_refresh;
                        if app.auto_refresh {
                            if let Some(track) = &app.current_track {
                                action_tx.send(Action::FetchLyrics(track.clone())).await?;
                            }
                        }
                    }
                    // KeyCode::Char('y') if k.kind == KeyEventKind::Press => {
                    //     app.track_position = app.track_position + Duration::from_millis(500);
                    // }
                    _ => {}
                },
                Event::Tick => {
                    app.tick = app.tick.wrapping_add(1);
                    app.track_position += Duration::from_millis(50);
                }
            }
        }

        if let Ok(action) = action_rx.try_recv() {
            let tx = action_tx.clone();
            match action {
                Action::TrackChanged(track) => {
                    app.current_track = Some(track.clone());
                    if app.auto_refresh {
                        let _ = tx.send(Action::FetchLyrics(track)).await;
                    }
                }
                Action::FetchLyrics(track) => {
                    tracing::info!("fetching lyrics for {}", track);
                    app.lyrics_state = LyricsState::Loading;
                    tokio::spawn(async move {
                        match get_romanized_lyrics(track.clone()).await {
                            Ok(lyrics_text) => {
                                let _ = tx.send(Action::LyricsFetched(Lyrics::new(track, lyrics_text))).await;
                            }
                            Err(e) => {
                                let _ = tx.send(Action::LyricsFetchError(e.to_string())).await;
                            }
                        }
                    });
                }
                Action::UpdatePlaybackPosition(position) => {
                    if position != app.last_api_track_position {
                        app.track_position = position;
                        app.last_api_track_position = position;
                    }
                }
                Action::LyricsFetched(lyrics) => {
                    tracing::info!("lyrics loaded: {} lines", lyrics.len());
                    app.lyrics_state = LyricsState::Loaded(lyrics);
                }
                Action::LyricsFetchError(error_str) => {
                    tracing::error!("lyrics fetch failed: {}", error_str);
                    app.lyrics_state = LyricsState::Error(error_str);
                }
                Action::Quit => app.exit = true,
            }
        }
    }

    Ok(())
}

async fn event_task(tx: mpsc::Sender<Event>) -> Result<()> {
    let mut tick_interval = tokio::time::interval(Duration::from_millis(50));
    tick_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = tick_interval.tick() => {
                let _ = tx.send(Event::Tick).await;
            }
            res = tokio::task::spawn_blocking(|| {
                event::poll(Duration::from_millis(10))
            }) => {
                if res?? {
                    if let event::Event::Key(key) = event::read().wrap_err("failed to read event")? {
                        let _ = tx.send(Event::Key(key)).await;
                    }
                }
            }
        }
    }
}

async fn media_task(mut rx: mpsc::Receiver<SpotifyAction>, tx: mpsc::Sender<Action>) -> Result<()> {
    let mut last_track = Track::new("", "");
    let mut media = MediaManager::new().await.wrap_err("failed to create media manager")?;
    let mut interval = tokio::time::interval(Duration::from_millis(500));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                media.refresh_session().await.wrap_err("failed to refresh session")?;

                match media.media_properties().await {
                    Ok(track) => {
                        if track != last_track {
                            tracing::info!("track changed: {}", track);
                            last_track = track.clone();
                            let _ = tx.send(Action::TrackChanged(track)).await;
                        }
                    }
                    Err(e) => tracing::warn!("media sync error: {e}")
                };

                match media.timeline_position().await {
                    Ok(position) => {
                        let _ = tx.send(Action::UpdatePlaybackPosition(position)).await;
                    }
                    Err(e) => tracing::warn!("media sync error: {e}"),
                };
            }
            media_action = rx.recv() => {
                let Some(media_action) = media_action else { return Ok(()); };
                match media_action {}
            }
        }
    }
}

fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    
    let title = match &app.lyrics_state {
        LyricsState::Loaded(lyrics) => {
            let secs = app.track_position.as_secs();
            let pos = format!("{:02}:{:02}", secs / 60, secs % 60);

            Line::from(vec![
                " ".into(),
                lyrics.track.artist.as_str().cyan(),
                " — ".gray(),
                lyrics.track.title.as_str().white().bold(),
                format!(" [{pos}]").italic().dark_gray(),
                " ".into(),
            ])
        }
        _ => Line::from(" Romaji Lyrics ".bold()),
    };
    let auto_refresh_status = if app.auto_refresh {
        "On".green().bold()
    } else {
        "Off".red().bold()
    };
    let keybinds = Line::from(vec![
        " Quit ".into(),
        "<Q>".blue().bold(),
        "  Auto-refresh ".into(),
        "<T> ".blue().bold(),
        auto_refresh_status,
        " ".into()
    ]);

    let block = Block::bordered()
        .title(title.centered())
        .title_bottom(keybinds.centered())
        .border_set(ratatui::symbols::border::THICK);

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let lyrics_area = inner_area.inner(Margin::new(inner_area.width / 4, 1));

    const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    //let spinner = SPINNER[app.tick as usize % SPINNER.len()];

    match &app.lyrics_state {
        LyricsState::None => {
            frame.render_widget(
                Paragraph::new("No track playing").centered(),
                lyrics_area
            );
        }
        LyricsState::Loading => {
            let spinner = SPINNER[app.tick % SPINNER.len()];
            frame.render_widget(
                Paragraph::new(format!("{spinner} Fetching lyrics...")).yellow().centered(),
                lyrics_area
            );
        }
        LyricsState::Loaded(lyrics) => {
            frame.render_widget(
                LyricsView {
                    lyrics,
                    playback_pos: app.track_position,
                },
                lyrics_area
            );
        }
        LyricsState::Error(e) => {
            frame.render_widget(
                Paragraph::new(e.as_str()).red().centered(),
                lyrics_area
            );
        }
    }

    // let p = Paragraph::new(match &app.lyrics_state {
    //     LyricsState::None => vec!["No track playing".into()],
    //     LyricsState::Loading => vec![
    //         Line::from(format!("{spinner} Fetching lyrics...")).yellow()
    //     ],
    //     LyricsState::Loaded(lyrics) => lyrics.unsynced_lines()
    //         .into_iter()
    //         .map(|s| Line::from(s))
    //         .collect(),
    //     LyricsState::Error(e) => vec![Line::from(e.as_str()).red()],
    // })
    // .wrap(Wrap { trim: false })
    // .centered();

    //frame.render_widget(p, lyrics_area);
}

fn setup_tracing() -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let file_appender = tracing_appender::rolling::never(".", "app.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_max_level(tracing::Level::DEBUG)
        .with_ansi(false)
        .with_env_filter(tracing_subscriber::EnvFilter::new("spotify_lyrics_tui=debug"))
        .init();

    Ok(guard)
}