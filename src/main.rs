use crate::browser::cookies::{detect, extract_cookies, Browser};
use crate::browser::engine::{Engine, HttpClient, Navigate};
use crate::config::Config;
use crate::scraper::crawler::{crawl, CrawlConfig, CrawlEvent};
use crate::tui::app::App;
use crate::tui::TuiMessage;
use clap::Parser;
use color_eyre::Result;
use indicatif::{ProgressBar, ProgressStyle};

mod browser;
mod config;
mod scraper;
mod tui;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = config::Cli::parse();
    let cfg = Config::resolve(cli)?;

    let browser_kind = match cfg.browser.to_lowercase().as_str() {
        "auto" => detect(),
        "firefox" => Ok(Browser::Firefox),
        "chrome" => Ok(Browser::Chrome),
        other => color_eyre::eyre::bail!("Invalid browser: {}", other),
    }?;

    let cookies = extract_cookies(&browser_kind, None)?;

    match cfg.url {
        Some(ref url) => {
            // ONE-SHOT MODE
            let parsed = url::Url::parse(url)?;
            if parsed.scheme() != "http" && parsed.scheme() != "https" {
                color_eyre::eyre::bail!("Only http:// and https:// URLs are supported, got: {}", parsed.scheme());
            }

            let crawl_cfg = CrawlConfig {
                max_pages: cfg.max_pages,
                prefix: cfg.prefix.clone(),
                delay_ms: cfg.delay_ms,
                selector: cfg.selector.clone(),
                output_dir: cfg.output.clone(),
                base_url: url.clone(),
            };

            let headless = cfg.headless || !is_tty();

            if !cfg.no_browser {
                let engine = Engine::new().await?;
                engine.set_cookies(&cookies).await?;
                if headless { run_headless(&engine, &crawl_cfg).await?; }
                else { run_with_tui(engine, &crawl_cfg, &cfg.browser, cookies.len()).await?; }
            } else {
                let client = HttpClient::new(&cookies)?;
                if headless { run_headless(&client, &crawl_cfg).await?; }
                else { run_with_tui(client, &crawl_cfg, &cfg.browser, cookies.len()).await?; }
            }
        }
        None => {
            // INTERACTIVE MODE
            if cfg.headless || !is_tty() {
                color_eyre::eyre::bail!("Interactive mode requires a terminal. Provide a URL for headless mode.");
            }

            let browser_name = format!("{:?}", browser_kind);
            let app = App::new_interactive(
                browser_name,
                cookies.len(),
                cfg.output.clone(),
                cfg.delay_ms,
                cfg.max_pages,
            );

            let engine_kind = if !cfg.no_browser {
                tui::EngineKind::BrowserLazy
            } else {
                tui::EngineKind::Http
            };

            tui::run_interactive(app, engine_kind, browser_kind, cookies).await?;
        }
    }

    Ok(())
}

async fn run_headless<N: Navigate>(engine: &N, cfg: &CrawlConfig) -> Result<()> {
    let pb = ProgressBar::new(cfg.max_pages as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} pages")?
            .progress_chars("=>-"),
    );

    crawl(engine, cfg, None, |event| match event {
        CrawlEvent::Saved(url, path) => {
            pb.println(format!("[OK] {} -> {}", url, path));
            pb.inc(1);
        }
        CrawlEvent::Error(url, err) => {
            pb.println(format!("[ERR] {} - {}", url, err));
            pb.inc(1);
        }
        CrawlEvent::AuthFail(url, code) => {
            pb.println(format!("[{}] {}", code, url));
            pb.inc(1);
        }
        CrawlEvent::Done { saved, failed, auth_failed } => {
            pb.finish_and_clear();
            println!("Done: {} saved, {} failed, {} auth errors", saved, failed, auth_failed);
        }
        _ => {}
    }).await?;

    Ok(())
}

async fn run_with_tui<N: Navigate + Send + 'static>(
    engine: N,
    cfg: &CrawlConfig,
    browser_name: &str,
    cookie_count: usize,
) -> Result<()> {
    let app = App::new(
        cfg.base_url.clone(),
        browser_name.to_string(),
        cookie_count,
        cfg.output_dir.clone(),
        cfg.delay_ms,
        cfg.max_pages,
    );
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<TuiMessage>();

    let crawl_cfg = CrawlConfig {
        max_pages: cfg.max_pages,
        prefix: cfg.prefix.clone(),
        delay_ms: cfg.delay_ms,
        selector: cfg.selector.clone(),
        output_dir: cfg.output_dir.clone(),
        base_url: cfg.base_url.clone(),
    };

    tokio::spawn(async move {
        let _ = crawl(&engine, &crawl_cfg, None, |event| {
            let msg = match event {
                CrawlEvent::Visiting(url) => TuiMessage::Visiting(url),
                CrawlEvent::Saved(url, filepath) => TuiMessage::Saved { url, filepath },
                CrawlEvent::AuthFail(url, code) => TuiMessage::AuthFail { url, code },
                CrawlEvent::Error(url, message) => TuiMessage::Error { url, message },
                CrawlEvent::Progress { visited, queue, .. } => TuiMessage::Progress { visited, total: visited + queue },
                CrawlEvent::Done { saved, failed, auth_failed } => TuiMessage::Done { saved, failed, auth_failed },
            };
            let _ = tx.send(msg);
        }).await;
    });

    tui::run_tui(app, rx).await
}

fn is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}
