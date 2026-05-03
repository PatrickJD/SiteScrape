use crate::browser::cookies::Cookie;
use async_trait::async_trait;
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::network::{CookieParam, TimeSinceEpoch};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use futures::StreamExt;
use std::sync::Arc;

/// Result of navigating to a page. Fields are populated by both Engine and HttpClient.
/// Some fields (url, title) are extracted but not currently consumed by the crawler,
/// which re-extracts title via html_to_markdown. Kept for future use.
#[allow(dead_code)]
pub struct PageResult {
    pub url: String,
    pub html: String,
    pub title: String,
    pub links: Vec<String>,
}

#[async_trait]
pub trait Navigate: Send + Sync {
    async fn navigate(&self, url: &str) -> Result<PageResult>;
}

// ── Browser engine (chromiumoxide) ────────────────────────────────────────────

pub struct Engine {
    browser: Browser,
    _handler: tokio::task::JoinHandle<()>,
}

impl Engine {
    pub async fn new() -> Result<Self> {
        let (browser, mut handler) = Browser::launch(
            BrowserConfig::builder().build().map_err(|e| eyre!("{}", e))?,
        )
        .await
        .map_err(|e| eyre!("Failed to launch Chrome: {}", e))?;

        let handle = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        Ok(Engine {
            browser,
            _handler: handle,
        })
    }

    pub async fn set_cookies(&self, cookies: &[Cookie]) -> Result<()> {
        let mut params = Vec::with_capacity(cookies.len());
        let mut skipped = 0usize;
        for c in cookies {
            // CDP requires name+value. Use url as fallback when domain has leading dot.
            let url_str = format!("https://{}{}", c.domain.trim_start_matches('.'), c.path);
            match CookieParam::builder()
                .name(&c.name)
                .value(&c.value)
                .url(&url_str)
                .domain(&c.domain)
                .path(&c.path)
                .secure(c.secure)
                .http_only(c.http_only)
                .expires(TimeSinceEpoch::new(c.expires))
                .build()
            {
                Ok(p) => params.push(p),
                Err(_) => skipped += 1,
            }
        }
        if skipped > 0 {
            eprintln!("Warning: {} cookies failed to build, skipped", skipped);
        }
        self.browser
            .set_cookies(params)
            .await
            .map_err(|e| eyre!("Failed to set cookies: {}", e))?;
        Ok(())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self._handler.abort();
    }
}

#[async_trait]
impl Navigate for Engine {
    async fn navigate(&self, url: &str) -> Result<PageResult> {
        let page = self
            .browser
            .new_page(url)
            .await
            .map_err(|e| eyre!("Failed to navigate to {}: {}", url, e))?;

        // Phase 1: wait for DOM to finish loading (up to 10s).
        for _ in 0..20 {
            let ready: bool = page
                .evaluate("document.readyState === 'complete'")
                .await
                .ok()
                .and_then(|r| r.into_value::<bool>().ok())
                .unwrap_or(false);
            if ready { break; }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        // Phase 2: wait for SPA content to render (up to 10s).
        // Try specific content IDs first (more targeted), then semantic elements.
        // Use the first candidate with >100 chars rather than "most text" to
        // avoid picking a broad wrapper (e.g. <main> that includes nav/sidebar).
        for _ in 0..20 {
            let has_content: bool = page
                .evaluate(
                    r#"(() => {
                        const sels = ['#main-col-body','#main-content','article','[role="main"]','main','.content'];
                        for (const s of sels) {
                            const el = document.querySelector(s);
                            if (el && el.innerText.trim().length > 100) return true;
                        }
                        return document.body && document.body.innerText.trim().length > 100;
                    })()"#,
                )
                .await
                .ok()
                .and_then(|r| r.into_value::<bool>().ok())
                .unwrap_or(false);
            if has_content { break; }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        let final_url = page
            .url()
            .await
            .map_err(|e| eyre!("{}", e))?
            .unwrap_or_else(|| url.to_string());

        let html = {
            // Try to get simplified content from the main content area via JS.
            // SPAs with web components produce HTML that
            // markdown converters can't parse. The browser's DOM API gives us
            // clean HTML by walking the rendered DOM and replacing custom
            // elements with their children.
            let simplified: Option<String> = page
                .evaluate(
                    r#"(() => {
                        const sels = ['#main-col-body','#main-content','article','[role="main"]','main','.content'];
                        let target = document.body;
                        for (const s of sels) {
                            const el = document.querySelector(s);
                            if (el && el.innerText.trim().length > 100) { target = el; break; }
                        }
                        if (!target) return null;
                        const clone = target.cloneNode(true);
                        let changed = true;
                        while (changed) {
                            changed = false;
                            for (const el of clone.querySelectorAll('*')) {
                                if (el.tagName.includes('-')) {
                                    while (el.firstChild) el.parentNode.insertBefore(el.firstChild, el);
                                    el.parentNode.removeChild(el);
                                    changed = true;
                                    break;
                                }
                            }
                        }
                        return clone.innerHTML;
                    })()"#,
                )
                .await
                .ok()
                .and_then(|r| r.into_value::<String>().ok());
            match simplified {
                Some(h) if !h.trim().is_empty() => {
                    // Wrap in basic HTML so htmd can parse it
                    format!("<html><body>{}</body></html>", h)
                }
                _ => page
                    .content()
                    .await
                    .map_err(|e| eyre!("Failed to get content: {}", e))?,
            }
        };

        let title = page
            .evaluate("document.title")
            .await
            .ok()
            .and_then(|r| r.into_value::<String>().ok())
            .unwrap_or_default();

        let links: Vec<String> = page
            .evaluate(
                "JSON.stringify(Array.from(document.querySelectorAll('a[href]')).map(a=>a.href))",
            )
            .await
            .ok()
            .and_then(|r| r.into_value::<String>().ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Ok(PageResult {
            url: final_url,
            html,
            title,
            links,
        })
    }
}

// ── HTTP-only engine (--no-browser) ──────────────────────────────────────────

pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    pub fn new(cookies: &[Cookie]) -> Result<Self> {
        let jar = Arc::new(reqwest::cookie::Jar::default());
        for c in cookies {
            let cookie_str = format!(
                "{}={}; Domain={}; Path={}",
                c.name, c.value, c.domain, c.path
            );
            if let Ok(url) =
                reqwest::Url::parse(&format!("https://{}", c.domain.trim_start_matches('.')))
            {
                jar.add_cookie_str(&cookie_str, &url);
            }
        }
        let client = reqwest::Client::builder()
            .cookie_provider(jar)
            .user_agent("Mozilla/5.0 sitescrape/0.1")
            .build()
            .map_err(|e| eyre!("{}", e))?;
        Ok(HttpClient { client })
    }
}

#[async_trait]
impl Navigate for HttpClient {
    async fn navigate(&self, url: &str) -> Result<PageResult> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| eyre!("{}", e))?;
        let status = resp.status();
        let final_url = resp.url().to_string();

        if status == 401 || status == 403 {
            return Err(eyre!("AUTH:{}", status.as_u16()));
        }
        if !status.is_success() {
            return Err(eyre!("HTTP {}: {}", status.as_u16(), url));
        }

        let html = resp.text().await.map_err(|e| eyre!("{}", e))?;
        let title = extract_title(&html);
        let links = extract_links(&html, &final_url);

        Ok(PageResult {
            url: final_url,
            html,
            title,
            links,
        })
    }
}

fn extract_title(html: &str) -> String {
    let lower = html.to_lowercase();
    if let Some(start) = lower.find("<title>") {
        if let Some(end) = lower[start..].find("</title>") {
            return html[start + 7..start + end].trim().to_string();
        }
    }
    String::new()
}

fn extract_links(html: &str, base_url: &str) -> Vec<String> {
    use url::Url;
    let base = match Url::parse(base_url) {
        Ok(u) => u,
        Err(_) => return vec![],
    };
    let mut links = Vec::new();
    let lower = html.to_lowercase();

    for (needle, quote) in [("href=\"", '"'), ("href=\'", '\'')] {
        let mut pos = 0;
        while let Some(idx) = lower[pos..].find(needle) {
            let start = pos + idx + needle.len();
            if let Some(end) = html[start..].find(quote) {
                let href = &html[start..start + end];
                if let Ok(abs) = base.join(href) {
                    links.push(abs.to_string());
                }
            }
            pos = start;
        }
    }
    links
}
