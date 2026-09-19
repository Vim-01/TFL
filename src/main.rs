use clap::Parser;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::path::Path;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;
use tfl::app::{ActiveTab, App};
use tfl::collector::CollectorEngine;
use tfl::config::Config;
use tfl::ui;
use tokio::sync::mpsc;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::parse();

    // Setup logging according to config.log_path
    let log_path = Path::new(&config.log_path);
    let log_dir = log_path.parent().unwrap_or_else(|| Path::new("."));
    let log_file = log_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("tfl.log");

    let file_appender = rolling::never(log_dir, log_file);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let _ = tracing_subscriber::registry()
        .with(EnvFilter::new("info"))
        .with(fmt::layer().with_writer(non_blocking))
        .try_init();

    // Install panic hook to restore terminal cleanly on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        original_hook(panic_info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::cursor::Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, config).await;

    // Restore terminal cleanly
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::cursor::Show
    );
    let _ = terminal.show_cursor();

    if let Err(err) = res {
        eprintln!("Error running TFL: {err:?}");
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    config: Config,
) -> anyhow::Result<()> {
    let poll_interval = Arc::new(AtomicU64::new(config.interval_ms));
    let mut app = App::with_poll_interval(poll_interval.clone());
    let (tx, mut rx) = mpsc::channel(128);

    // Spawn async background collectors with dynamic shared poll interval
    let _collectors = CollectorEngine::spawn(
        tx,
        config.endpoint_url.clone(),
        poll_interval,
    );

    let mut event_stream = EventStream::new();
    let mut render_interval = tokio::time::interval(Duration::from_millis(50));
    render_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            // Receive background metric updates
            metric_opt = rx.recv() => {
                match metric_opt {
                    Some(metric) => app.handle_metric_update(metric),
                    None => break, // Background collectors shut down
                }
            }

            // Receive keyboard / terminal events
            event_opt = event_stream.next() => {
                match event_opt {
                    Some(Ok(event)) => {
                        if let Event::Key(key) = event {
                            if key.kind != KeyEventKind::Release {
                                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                                    app.quit();
                                } else if app.options_menu_open {
                                    match key.code {
                                        KeyCode::Esc | KeyCode::Char('q') => app.toggle_options_menu(),
                                        KeyCode::Up | KeyCode::Char('k') => app.on_menu_up(),
                                        KeyCode::Down | KeyCode::Char('j') => app.on_menu_down(),
                                        KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => app.on_menu_left(),
                                        KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter | KeyCode::Char(' ') => app.on_menu_right_or_enter(),
                                        _ => {}
                                    }
                                } else {
                                    match key.code {
                                        KeyCode::Char('q') => app.quit(),
                                        KeyCode::Esc | KeyCode::Char('o') => app.toggle_options_menu(),
                                        KeyCode::Tab => app.on_tab_pressed(),
                                        KeyCode::BackTab => app.on_backtab_pressed(),
                                        KeyCode::Char('p') => app.toggle_pause(),
                                        KeyCode::Char('1') => app.active_tab = ActiveTab::Dashboard,
                                        KeyCode::Char('2') => app.active_tab = ActiveTab::GpuDetails,
                                        KeyCode::Char('3') => app.active_tab = ActiveTab::SlotsDetails,
                                        KeyCode::Char('4') | KeyCode::Char('?') => app.active_tab = ActiveTab::Help,
                                        KeyCode::Down | KeyCode::Char('j') => app.next_item(),
                                        KeyCode::Up | KeyCode::Char('k') => app.prev_item(),
                                        KeyCode::Right | KeyCode::Char('l') => app.on_tab_pressed(),
                                        KeyCode::Left | KeyCode::Char('h') => app.on_backtab_pressed(),
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    Some(Err(_)) => break,
                    None => break, // Event stream closed (e.g. EOF)
                }
            }

            // Periodic UI draw tick
            _ = render_interval.tick() => {
                terminal.draw(|frame| {
                    ui::draw(frame, &app);
                })?;
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
