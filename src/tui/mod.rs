pub mod app;
pub mod commands;
pub mod events;
pub mod ui;

pub use app::App;

use crate::tui::app::{AppState, LogEntry, LogStatus};
use crate::tui::events::{poll, Event};
use color_eyre::Result;
use crossterm::{
    event::{KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tokio::sync::mpsc::UnboundedReceiver;

pub enum TuiMessage {
    Visiting(String),
    Saved {
        url: String,
        filepath: String,
    },
    AuthFail {
        url: String,
        code: u16,
    },
    Error {
        url: String,
        message: String,
    },
    Progress {
        visited: usize,
        total: usize,
    },
    Done {
        saved: usize,
        failed: usize,
        auth_failed: usize,
    },
}

/// RAII guard — restores terminal even on panic.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

pub async fn run_tui(mut app: App, mut rx: UnboundedReceiver<TuiMessage>) -> Result<()> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    loop {
        while let Ok(msg) = rx.try_recv() {
            match msg {
                TuiMessage::Visiting(url) => {
                    app.push_log(LogEntry {
                        status: LogStatus::Visiting,
                        url,
                        filepath: None,
                        message: None,
                    });
                }
                TuiMessage::Saved { url, filepath } => {
                    app.push_log(LogEntry {
                        status: LogStatus::Ok,
                        url,
                        filepath: Some(filepath),
                        message: None,
                    });
                }
                TuiMessage::AuthFail { url, code } => {
                    app.push_log(LogEntry {
                        status: LogStatus::AuthFail(code),
                        url,
                        filepath: None,
                        message: Some("auth failed, skipped".to_string()),
                    });
                }
                TuiMessage::Error { url, message } => {
                    app.push_log(LogEntry {
                        status: LogStatus::Error,
                        url,
                        filepath: None,
                        message: Some(message),
                    });
                }
                TuiMessage::Progress { visited, total } => {
                    app.set_progress(visited, total);
                }
                TuiMessage::Done {
                    saved,
                    failed,
                    auth_failed,
                } => {
                    app.set_done(saved, failed, auth_failed);
                }
            }
        }

        terminal.draw(|f| ui::draw(f, &app))?;

        let event = tokio::task::spawn_blocking(poll).await??;
        match event {
            Event::Key(key) => {
                let ctrl_c =
                    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c');
                if ctrl_c || key.code == KeyCode::Char('q') {
                    break;
                }
                match key.code {
                    KeyCode::Char('p') => app.toggle_pause(),
                    KeyCode::Up | KeyCode::Char('k') => app.scroll_up(),
                    KeyCode::Down | KeyCode::Char('j') => app.scroll_down(),
                    _ => {}
                }
            }
            Event::Tick => {
                app.spinner_frame = app.spinner_frame.wrapping_add(1);
            }
        }
    }

    Ok(())
}

// ── Interactive mode ──────────────────────────────────────────────────────────

use crate::browser::cookies::{self, Browser, Cookie};
use crate::browser::engine::{Engine, HttpClient};
use crate::scraper::crawler::{crawl, CrawlConfig, CrawlEvent};
use crate::tui::commands::{parse_command, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

pub enum EngineKind {
    BrowserLazy,
    Http,
}

pub async fn run_interactive(
    mut app: App,
    engine_kind: EngineKind,
    browser_kind: Browser,
    cookies: Vec<Cookie>,
) -> Result<()> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    app.push_banner("".into());
    app.push_banner("  _____ _ _       _____                            ".into());
    app.push_banner(" / ____(_) |     / ____|                           ".into());
    app.push_banner("| (___  _| |_ ___| (___   ___ _ __ __ _ _ __   ___".into());
    app.push_banner(" \\___ \\| | __/ _ \\\\___ \\ / __| '__/ _` | '_ \\ / _ \\".into());
    app.push_banner(" ____) | | ||  __/____) | (__| | | (_| | |_) |  __/".into());
    app.push_banner("|_____/|_|\\__\\___|_____/ \\___|_|  \\__,_| .__/ \\___|".into());
    app.push_banner("                                       | |         ".into());
    app.push_banner("                                       |_|         ".into());
    app.push_banner("".into());
    app.push_info("Ready. Type /help for commands.".into());

    let mut active_rx: Option<UnboundedReceiver<TuiMessage>> = None;
    let mut active_handle: Option<tokio::task::JoinHandle<()>> = None;
    let mut stop_flag: Option<Arc<AtomicBool>> = None;
    let mut current_cookies = cookies;

    loop {
        // Drain scrape messages
        if let Some(ref mut rx) = active_rx {
            let mut done = false;
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    TuiMessage::Visiting(url) => {
                        app.push_log(LogEntry {
                            status: LogStatus::Visiting,
                            url,
                            filepath: None,
                            message: None,
                        });
                    }
                    TuiMessage::Saved { url, filepath } => {
                        app.push_log(LogEntry {
                            status: LogStatus::Ok,
                            url,
                            filepath: Some(filepath),
                            message: None,
                        });
                    }
                    TuiMessage::AuthFail { url, code } => {
                        app.push_log(LogEntry {
                            status: LogStatus::AuthFail(code),
                            url,
                            filepath: None,
                            message: Some("auth failed, skipped".into()),
                        });
                    }
                    TuiMessage::Error { url, message } => {
                        app.push_log(LogEntry {
                            status: LogStatus::Error,
                            url,
                            filepath: None,
                            message: Some(message),
                        });
                    }
                    TuiMessage::Progress { visited, total } => {
                        app.set_progress(visited, total);
                    }
                    TuiMessage::Done {
                        saved,
                        failed,
                        auth_failed,
                    } => {
                        app.set_done(saved, failed, auth_failed);
                        done = true;
                        break;
                    }
                }
            }
            if done {
                active_rx = None;
                active_handle = None;
                stop_flag = None;
            }
            if !done && active_handle.as_ref().is_some_and(|h| h.is_finished()) {
                active_rx = None;
                active_handle = None;
                stop_flag = None;
                app.set_done(app.saved, app.failed, app.auth_failed);
            }
        }

        if matches!(app.state, AppState::Done) && active_rx.is_none() {
            app.back_to_idle();
        }

        terminal.draw(|f| ui::draw(f, &app))?;

        let event = tokio::task::spawn_blocking(poll).await??;
        match event {
            Event::Tick => {
                app.spinner_frame = app.spinner_frame.wrapping_add(1);
            }
            Event::Key(key) => {
                let ctrl_c =
                    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c');

                match app.state {
                    AppState::Idle => {
                        if ctrl_c {
                            break;
                        }
                        match key.code {
                            KeyCode::Char(c) => app.input_char(c),
                            KeyCode::Backspace => app.input_backspace(),
                            KeyCode::Delete => app.input_delete(),
                            KeyCode::Left => app.move_cursor_left(),
                            KeyCode::Right => app.move_cursor_right(),
                            KeyCode::Esc => app.clear_input(),
                            KeyCode::Up => app.scroll_up(),
                            KeyCode::Down => app.scroll_down(),
                            KeyCode::Enter => {
                                let input = app.submit_input();
                                if input.is_empty() {
                                    continue;
                                }
                                app.push_info(format!("> {}", input));
                                match parse_command(&input) {
                                    Command::Scrape {
                                        url,
                                        max_pages,
                                        selector,
                                        prefix,
                                        delay,
                                        no_browser,
                                    } => {
                                        if !url.starts_with("http://")
                                            && !url.starts_with("https://")
                                        {
                                            app.push_info(
                                                "Invalid URL: must start with http:// or https://"
                                                    .into(),
                                            );
                                        } else {
                                            let effective_selector =
                                                selector.or_else(|| app.selector.clone());
                                            let crawl_cfg = CrawlConfig {
                                                max_pages: max_pages.unwrap_or(app.max_pages),
                                                prefix,
                                                delay_ms: delay.unwrap_or(app.delay_ms),
                                                selector: effective_selector,
                                                output_dir: app.output_dir.clone(),
                                                base_url: url.clone(),
                                            };
                                            let (tx, rx) =
                                                tokio::sync::mpsc::unbounded_channel::<TuiMessage>(
                                                );
                                            let sf = Arc::new(AtomicBool::new(false));
                                            let use_browser = !no_browser
                                                && matches!(engine_kind, EngineKind::BrowserLazy);

                                            let cookies_clone = current_cookies.clone();
                                            let sf_clone = Arc::clone(&sf);

                                            let handle = tokio::spawn(async move {
                                                let result: Result<(), color_eyre::Report> =
                                                    async {
                                                        if use_browser {
                                                            let _ = tx.send(TuiMessage::Visiting(
                                                                "Launching browser...".into(),
                                                            ));
                                                            if sf_clone.load(Ordering::Relaxed) {
                                                                return Err(
                                                                    color_eyre::eyre::eyre!(
                                                                        "Stopped"
                                                                    ),
                                                                );
                                                            }
                                                            let engine = Engine::new().await?;
                                                            engine
                                                                .set_cookies(&cookies_clone)
                                                                .await?;
                                                            if sf_clone.load(Ordering::Relaxed) {
                                                                return Err(
                                                                    color_eyre::eyre::eyre!(
                                                                        "Stopped"
                                                                    ),
                                                                );
                                                            }
                                                            crawl(
                                                                &engine,
                                                                &crawl_cfg,
                                                                Some(&*sf_clone),
                                                                |ev| {
                                                                    send_crawl_event(
                                                                        &tx, ev, &sf_clone,
                                                                    )
                                                                },
                                                            )
                                                            .await?;
                                                        } else {
                                                            let client =
                                                                HttpClient::new(&cookies_clone)?;
                                                            crawl(
                                                                &client,
                                                                &crawl_cfg,
                                                                Some(&*sf_clone),
                                                                |ev| {
                                                                    send_crawl_event(
                                                                        &tx, ev, &sf_clone,
                                                                    )
                                                                },
                                                            )
                                                            .await?;
                                                        }
                                                        Ok(())
                                                    }
                                                    .await;
                                                if let Err(e) = result {
                                                    let _ = tx.send(TuiMessage::Error {
                                                        url: "engine".into(),
                                                        message: e.to_string(),
                                                    });
                                                    let _ = tx.send(TuiMessage::Done {
                                                        saved: 0,
                                                        failed: 0,
                                                        auth_failed: 0,
                                                    });
                                                }
                                            });

                                            active_rx = Some(rx);
                                            active_handle = Some(handle);
                                            stop_flag = Some(sf);
                                            app.reset_for_scrape(url);
                                        }
                                    }
                                    Command::Stop => app.push_info("No scrape in progress.".into()),
                                    Command::Status => app.push_info("Idle.".into()),
                                    Command::Config => {
                                        app.push_info(format!("browser: {}", app.browser_name));
                                        app.push_info(format!("output: {}", app.output_dir));
                                        app.push_info(format!("max_pages: {}", app.max_pages));
                                        app.push_info(format!("delay: {}ms", app.delay_ms));
                                        if let Some(ref sel) = app.selector {
                                            app.push_info(format!("selector: {}", sel));
                                        }
                                    }
                                    Command::Set { key, value } => match key.as_str() {
                                        "output" => {
                                            app.output_dir = value;
                                            app.push_info("output updated.".into());
                                        }
                                        "delay" => {
                                            if let Ok(d) = value.parse() {
                                                app.delay_ms = d;
                                                app.push_info("delay updated.".into());
                                            } else {
                                                app.push_info("Invalid delay value.".into());
                                            }
                                        }
                                        "max_pages" | "max-pages" => {
                                            if let Ok(m) = value.parse() {
                                                app.max_pages = m;
                                                app.push_info("max_pages updated.".into());
                                            } else {
                                                app.push_info("Invalid max_pages value.".into());
                                            }
                                        }
                                        "selector" => {
                                            app.selector = Some(value);
                                            app.push_info("selector updated.".into());
                                        }
                                        _ => app.push_info(format!("Unknown config key: {}", key)),
                                    },
                                    Command::Cookies => {
                                        app.push_info("Reloading cookies...".into());
                                        match cookies::extract_cookies(&browser_kind, None) {
                                            Ok(new_cookies) => {
                                                app.cookie_count = new_cookies.len();
                                                current_cookies = new_cookies;
                                                app.push_info(format!(
                                                    "{} cookies loaded.",
                                                    app.cookie_count
                                                ));
                                            }
                                            Err(e) => app
                                                .push_info(format!("Cookie reload failed: {}", e)),
                                        }
                                    }
                                    Command::Clear => {
                                        app.log.clear();
                                        app.scroll_offset = 0;
                                    }
                                    Command::Help => {
                                        app.push_info("/scrape <url> [-m N] [-s sel] [-p pfx] [-d ms] [--no-browser]".into());
                                        app.push_info(
                                            "/stop or [Esc] — stop current scrape".into(),
                                        );
                                        app.push_info("/status — show scrape status".into());
                                        app.push_info("/config — show current config".into());
                                        app.push_info("/set <key> <value> — change config (output, delay, max_pages, selector)".into());
                                        app.push_info("/cookies — reload browser cookies".into());
                                        app.push_info("/clear — clear log".into());
                                        app.push_info("/quit — exit".into());
                                    }
                                    Command::Quit => break,
                                    Command::Unknown(msg) => {
                                        if !msg.is_empty() {
                                            app.push_info(msg);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    AppState::Crawling | AppState::Paused => {
                        if ctrl_c {
                            break;
                        }
                        match key.code {
                            KeyCode::Esc => {
                                if let Some(ref sf) = stop_flag {
                                    sf.store(true, Ordering::Relaxed);
                                }
                                app.push_info("Stopping...".into());
                            }
                            KeyCode::Char('p') => app.toggle_pause(),
                            KeyCode::Up => app.scroll_up(),
                            KeyCode::Down => app.scroll_down(),
                            _ => {}
                        }
                    }
                    _ => {
                        if ctrl_c {
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn send_crawl_event(tx: &UnboundedSender<TuiMessage>, ev: CrawlEvent, stop: &AtomicBool) {
    if stop.load(Ordering::Relaxed)
        && !matches!(ev, CrawlEvent::Done { .. } | CrawlEvent::Error(..))
    {
        return;
    }
    let msg = match ev {
        CrawlEvent::Visiting(url) => TuiMessage::Visiting(url),
        CrawlEvent::Saved(url, fp) => TuiMessage::Saved { url, filepath: fp },
        CrawlEvent::AuthFail(url, code) => TuiMessage::AuthFail { url, code },
        CrawlEvent::Error(url, msg) => TuiMessage::Error { url, message: msg },
        CrawlEvent::Progress { visited, queue, .. } => TuiMessage::Progress {
            visited,
            total: visited + queue,
        },
        CrawlEvent::Done {
            saved,
            failed,
            auth_failed,
        } => TuiMessage::Done {
            saved,
            failed,
            auth_failed,
        },
    };
    let _ = tx.send(msg);
}
