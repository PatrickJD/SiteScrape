use clap::Parser;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(name = "sitescrape", version, about = "Scrape internal sites to markdown")]
pub struct Cli {
    /// URL to scrape
    pub url: Option<String>,

    /// Cookie source browser (auto, firefox, chrome)
    #[arg(short, long)]
    pub browser: Option<String>,

    /// Output directory
    #[arg(short, long)]
    pub output: Option<String>,

    /// Max pages to crawl
    #[arg(short, long)]
    pub max_pages: Option<usize>,

    /// CSS selector for main content
    #[arg(short, long)]
    pub selector: Option<String>,

    /// URL path prefix filter
    #[arg(short, long)]
    pub prefix: Option<String>,

    /// Delay between pages in ms
    #[arg(short, long)]
    pub delay: Option<u64>,

    /// HTTP-only mode (no JS rendering)
    #[arg(long)]
    pub no_browser: bool,

    /// Disable TUI, plain text output
    #[arg(long)]
    pub headless: bool,

    /// Config file path
    #[arg(long)]
    pub config: Option<String>,

    /// Named profile from config file
    #[arg(long)]
    pub profile: Option<String>,
}

/// A profile section in the config file (all fields optional).
#[derive(Deserialize, Default, Debug)]
pub struct ProfileConfig {
    pub url: Option<String>,
    pub browser: Option<String>,
    pub output: Option<String>,
    pub max_pages: Option<usize>,
    pub delay: Option<u64>,
    pub selector: Option<String>,
    pub prefix: Option<String>,
}

#[derive(Deserialize, Default, Debug)]
pub struct TomlConfig {
    #[serde(default)]
    pub default: ProfileConfig,
    #[serde(default)]
    pub profiles: HashMap<String, ProfileConfig>,
}

/// Fully resolved config after merging file defaults → profile → CLI args.
pub struct Config {
    pub url: Option<String>,
    pub browser: String,
    pub output: String,
    pub max_pages: usize,
    pub delay_ms: u64,
    pub selector: Option<String>,
    pub prefix: Option<String>,
    pub no_browser: bool,
    pub headless: bool,
}

impl Config {
    pub fn resolve(cli: Cli) -> Result<Self> {
        // Load config file
        let toml_cfg = load_toml(cli.config.as_deref())?;

        // Start with [default] section
        let def = &toml_cfg.default;

        // Overlay profile if requested
        let prof = cli.profile.as_deref()
            .map(|name| toml_cfg.profiles.get(name)
                .ok_or_else(|| eyre!("Profile '{}' not found in config file", name)))
            .transpose()?;

        // Helper: CLI > profile > default > hardcoded fallback
        macro_rules! resolve {
            ($cli:expr, $field:ident, $fallback:expr) => {
                $cli.or_else(|| prof.and_then(|p| p.$field.clone()))
                    .or_else(|| def.$field.clone())
                    .unwrap_or($fallback)
            };
        }

        let url = cli.url
            .or_else(|| prof.and_then(|p| p.url.clone()))
            .or_else(|| def.url.clone());
        // None is valid — means interactive mode

        let browser = resolve!(cli.browser, browser, "auto".to_string());
        match browser.to_lowercase().as_str() {
            "auto" | "firefox" | "chrome" => {}
            other => return Err(eyre!("Invalid --browser '{}' (use auto, firefox, or chrome)", other)),
        }

        Ok(Config {
            url,
            browser,
            output: resolve!(cli.output, output, "./output".to_string()),
            max_pages: cli.max_pages
                .or_else(|| prof.and_then(|p| p.max_pages))
                .or(def.max_pages)
                .unwrap_or(500),
            delay_ms: cli.delay
                .or_else(|| prof.and_then(|p| p.delay))
                .or(def.delay)
                .unwrap_or(500),
            selector: cli.selector
                .or_else(|| prof.and_then(|p| p.selector.clone()))
                .or_else(|| def.selector.clone()),
            prefix: cli.prefix
                .or_else(|| prof.and_then(|p| p.prefix.clone()))
                .or_else(|| def.prefix.clone()),
            no_browser: cli.no_browser,
            headless: cli.headless,
        })
    }
}

fn load_toml(path: Option<&str>) -> Result<TomlConfig> {
    let file = match path {
        Some(p) => std::path::PathBuf::from(p),
        None => std::path::PathBuf::from("sitescrape.toml"),
    };
    if !file.exists() {
        return Ok(TomlConfig::default());
    }
    let text = std::fs::read_to_string(&file)
        .map_err(|e| eyre!("Failed to read config file {}: {}", file.display(), e))?;
    toml::from_str(&text).map_err(|e| eyre!("Failed to parse config file: {}", e))
}
