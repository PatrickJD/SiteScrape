use crate::browser::engine::Navigate;
use crate::scraper::converter::html_to_markdown;
use crate::scraper::output::{url_to_filepath, write_page};
use color_eyre::Result;
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use url::Url;

const ASSET_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "svg", "css", "js", "woff", "pdf", "zip", "ico", "woff2", "ttf",
    "eot",
];

#[allow(dead_code)]
pub enum CrawlEvent {
    Visiting(String),
    Saved(String, String),
    AuthFail(String, u16),
    Error(String, String),
    Progress {
        visited: usize,
        saved: usize,
        queue: usize,
    },
    Done {
        saved: usize,
        failed: usize,
        auth_failed: usize,
    },
}

pub struct CrawlConfig {
    pub max_pages: usize,
    pub prefix: Option<String>,
    pub delay_ms: u64,
    pub selector: Option<String>,
    pub output_dir: String,
    pub base_url: String,
}

pub async fn crawl<N: Navigate>(
    engine: &N,
    config: &CrawlConfig,
    stop: Option<&AtomicBool>,
    mut on_event: impl FnMut(CrawlEvent),
) -> Result<usize> {
    let base = Url::parse(&config.base_url)?;
    let origin = base.origin().ascii_serialization();
    let mut visited = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(config.base_url.clone());
    let (mut saved, mut failed, mut auth_failed) = (0usize, 0usize, 0usize);

    while let Some(url) = queue.pop_front() {
        if let Some(sf) = stop {
            if sf.load(Ordering::Relaxed) {
                break;
            }
        }
        if saved >= config.max_pages {
            break;
        }
        if visited.contains(&url) {
            continue;
        }
        visited.insert(url.clone());

        let u = match Url::parse(&url) {
            Ok(u) => u,
            Err(_) => continue,
        };
        if u.origin().ascii_serialization() != origin {
            continue;
        }
        if let Some(ext) = u.path().rsplit('.').next() {
            if ASSET_EXTS.contains(&ext.to_lowercase().as_str()) {
                continue;
            }
        }
        if let Some(ref pfx) = config.prefix {
            if !u.path().starts_with(pfx.as_str()) {
                continue;
            }
        }

        on_event(CrawlEvent::Visiting(url.clone()));

        let result = match engine.navigate(&url).await {
            Ok(r) => r,
            Err(e) => {
                let msg = e.to_string();
                if let Some(code_str) = msg.strip_prefix("AUTH:") {
                    let code: u16 = code_str.parse().unwrap_or(401);
                    on_event(CrawlEvent::AuthFail(url.clone(), code));
                    auth_failed += 1;
                } else {
                    on_event(CrawlEvent::Error(url.clone(), msg));
                    failed += 1;
                }
                continue;
            }
        };

        for link in &result.links {
            if let Ok(lu) = Url::parse(link) {
                let mut clean = lu.clone();
                clean.set_fragment(None);
                let s = clean.to_string();
                if lu.origin().ascii_serialization() == origin && !visited.contains(&s) {
                    if let Some(ref pfx) = config.prefix {
                        if !lu.path().starts_with(pfx.as_str()) {
                            continue;
                        }
                    }
                    queue.push_back(s);
                }
            }
        }

        match html_to_markdown(&result.html, config.selector.as_deref()) {
            Ok(page) => {
                let fp = url_to_filepath(&url, &config.base_url);
                // Prefer title from browser's document.title (clean) over HTML-parsed title
                let title = if !result.title.is_empty() {
                    &result.title
                } else {
                    &page.title
                };
                match write_page(&config.output_dir, &fp, title, &url, &page.markdown) {
                    Ok(_) => {
                        saved += 1;
                        on_event(CrawlEvent::Saved(url.clone(), fp));
                    }
                    Err(e) => {
                        failed += 1;
                        on_event(CrawlEvent::Error(url.clone(), e.to_string()));
                    }
                }
            }
            Err(e) => {
                failed += 1;
                on_event(CrawlEvent::Error(url.clone(), e.to_string()));
            }
        }

        if visited.len() % 20 == 0 {
            on_event(CrawlEvent::Progress {
                visited: visited.len(),
                saved,
                queue: queue.len(),
            });
        }

        if config.delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(config.delay_ms)).await;
        }
    }

    on_event(CrawlEvent::Done {
        saved,
        failed,
        auth_failed,
    });
    Ok(saved)
}
