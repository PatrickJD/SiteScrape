# Configuration

## Config File

SiteScrape reads configuration from a TOML file. It looks for `sitescrape.toml` in the current directory, or you can specify a path with `--config`.

```toml
[default]
browser = "auto"       # auto, firefox, chrome
output = "./output"    # output directory
max_pages = 500        # max pages to crawl
delay = 500            # delay between pages in ms
selector = "main"      # CSS selector for main content

[profiles.mysite]
url = "https://docs.example.com/guide/"
max_pages = 200
selector = ".content"
```

## Profiles

Profiles let you save settings for sites you scrape frequently. Use them with `--profile`:

```bash
sitescrape --profile mysite
```

Profile values override `[default]` values. CLI arguments override everything.

## Precedence

CLI args > profile values > `[default]` values.

For example, if your profile sets `max_pages = 200` but you pass `-m 50`, the limit is 50.

## CLI Options Reference

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--browser` | `-b` | Cookie source browser (`auto`, `firefox`, `chrome`) | `auto` |
| `--output` | `-o` | Output directory | `./output` |
| `--max-pages` | `-m` | Maximum pages to crawl | `500` |
| `--selector` | `-s` | CSS selector for main content | none (full page) |
| `--prefix` | `-p` | URL path prefix filter | none |
| `--delay` | `-d` | Delay between pages in ms | `500` |
| `--no-browser` | | HTTP-only mode, no JS rendering | off |
| `--headless` | | Plain text output, no TUI | off |
| `--config` | | Path to config file | `./sitescrape.toml` |
| `--profile` | | Named profile from config file | none |
