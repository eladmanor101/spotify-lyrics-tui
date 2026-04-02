mod lyrics;
mod media;
mod models;
mod widgets;

use std::{io::Stdout, time::Duration};

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use ratatui::{
    Frame, Terminal, layout::Margin, prelude::CrosstermBackend, style::Stylize, text::Line, widgets::{Block, Paragraph, Wrap}
};
use tokio::{sync::mpsc, time};
use color_eyre::{Result, eyre::WrapErr};
use futures::StreamExt;

use crate::{
    lyrics::get_romanized_lyrics,
    media::{MediaSource, WindowsMediaSource},
    models::{Lyrics, PlaybackStatus, Track},
    widgets::LyricsView
};

enum Action {
    TrackChanged(Track),
    PlaybackStatusChanged(PlaybackStatus),
    FetchLyrics(Track),
    UpdatePlaybackPosition(Duration),

    LyricsFetched(Lyrics),
    LyricsFetchError(String),
    Quit
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
    playback_status: PlaybackStatus,
    track_position: Duration,
    lyrics_state: LyricsState,

    auto_refresh: bool
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let _guard = setup_tracing()?;

    let mut terminal = ratatui::init();

    let mut app = App::default();

    let (action_tx, action_rx) = mpsc::channel(32);

    let media_handle = tokio::spawn(media_task(action_tx.clone()));

    tokio::select! {
        result = media_handle => result?,
        result = run(&mut terminal, &mut app, action_rx, action_tx) => result
    }?;

    ratatui::restore();
    Ok(())
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    mut action_rx: mpsc::Receiver<Action>,
    action_tx: mpsc::Sender<Action>
) -> Result<()> {
    let tick_rate = Duration::from_secs_f64(1.0 / 20.0);
    let frame_rate = Duration::from_secs_f64(1.0 / 30.0);
    let mut tick_interval = time::interval(tick_rate);
    let mut frame_interval = time::interval(frame_rate);

    let mut stream = EventStream::new();

    loop {
        tokio::select! {
            _tick = tick_interval.tick() => {
                app.tick = app.tick.wrapping_add(1);
                
                if app.playback_status == PlaybackStatus::Playing {
                    app.track_position += tick_rate;
                }
            }
            _frame = frame_interval.tick() => {
                terminal.draw(|frame| render(frame, &app))?;
            }
            Some(Ok(event)) = stream.next() => {
                match event {
                    Event::Key(k) => match k.code {
                        KeyCode::Char('q') => action_tx.send(Action::Quit).await?,
                        KeyCode::Char('t') if k.kind == KeyEventKind::Press => {
                            app.auto_refresh = !app.auto_refresh;
                            if app.auto_refresh {
                                if let Some(track) = &app.current_track {
                                    action_tx.send(Action::FetchLyrics(track.clone())).await?;
                                }
                            }
                        }
                        _ => {}
                    }
                    _ => {}
                }
            }
            Some(action) = action_rx.recv() => {
                let tx = action_tx.clone();
                match action {
                    Action::TrackChanged(track) => {
                        app.current_track = Some(track.clone());
                        if app.auto_refresh {
                            let _ = tx.send(Action::FetchLyrics(track)).await;
                        }
                    }
                    Action::PlaybackStatusChanged(status) => {
                        app.playback_status = status;
                    },
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
                        app.track_position = position;
                    }
                    Action::LyricsFetched(lyrics) => {
                        tracing::info!("lyrics loaded: {} lines", lyrics.len());
                        app.lyrics_state = LyricsState::Loaded(lyrics);
                    }
                    Action::LyricsFetchError(error_str) => {
                        tracing::error!("lyrics fetch failed: {}", error_str);
                        app.lyrics_state = LyricsState::Error(error_str);
                    }
                    Action::Quit => {
                        return Ok(());
                    }
                }
            }
        }
    }
}

async fn media_task(tx: mpsc::Sender<Action>) -> Result<()> {
    let mut media = WindowsMediaSource::new().await.wrap_err("failed to create media source")?;

    let mut last_track = Track::default();
    let mut last_playback_status = PlaybackStatus::default();
    let mut last_playback_position = Duration::ZERO;

    let mut interval = time::interval(Duration::from_millis(50));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        interval.tick().await;

        if let Err(e) = media.refresh().await {
            tracing::warn!("failed to refresh session: {e}");
            continue;
        }

        match media.current_playback_status().await {
            Ok(status) => {
                if status != last_playback_status {
                    tracing::info!("playback status changed: {}", status);
                    last_playback_status = status;
                    let _ = tx.send(Action::PlaybackStatusChanged(status));
                }
            }
            Err(e) => tracing::warn!("media sync error: {e}")
        };

        match media.current_track().await {
            Ok(track) => {
                if track != last_track {
                    tracing::info!("track changed: {}", track);
                    last_track = track.clone();
                    let _ = tx.send(Action::TrackChanged(track)).await;
                }
            }
            Err(e) => tracing::warn!("media sync error: {e}")
        };

        match media.current_playback_position().await {
            Ok(position) => {
                if position != last_playback_position {
                    last_playback_position = position;
                    let _ = tx.send(Action::UpdatePlaybackPosition(position)).await;
                }
                
            }
            Err(e) => tracing::warn!("media sync error: {e}"),
        };
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
                lyrics.track.artist.cyan(),
                " — ".gray(),
                lyrics.track.title.white().bold(),
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
    frame.render_widget(&block, area);

    let area = block
        .inner(area) // Subtract border
        .inner(Margin::new(0, 1)); // Subtract margin of 1 pixel above and below

    const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

    match &app.lyrics_state {
        LyricsState::None => {
            frame.render_widget(
                Paragraph::new("No track playing")
                    .centered()
                    .wrap(Wrap { trim: false }),
                area
            );
        }
        LyricsState::Loading => {
            let spinner = SPINNER[app.tick % SPINNER.len()];
            frame.render_widget(
                Paragraph::new(format!("{spinner} Fetching lyrics..."))
                    .yellow()
                    .centered()
                    .wrap(Wrap { trim: false }),
                area
            );
        }
        LyricsState::Loaded(lyrics) => {
            frame.render_widget(
                LyricsView::new(lyrics, app.track_position),
                area
            );
        }
        LyricsState::Error(e) => {
            frame.render_widget(
                Paragraph::new(e.as_str())
                    .red()
                    .centered()
                    .wrap(Wrap { trim: false }),
                area
            );
        }
    }
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