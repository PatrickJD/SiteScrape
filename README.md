[![CI](https://github.com/PatrickJD/SiteScrape/actions/workflows/ci.yml/badge.svg)](https://github.com/PatrickJD/SiteScrape/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/PatrickJD/SiteScrape)](https://github.com/PatrickJD/SiteScrape/releases/latest)
[![License](https://img.shields.io/badge/license-MIT--0-blue)](LICENSE)
[![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-brightgreen)]()

# sitescrape

A CLI tool that scrapes authenticated internal websites and converts pages to markdown files. Uses your existing browser cookies for authentication. Ships as a single Rust binary.

## Requirements

- Rust toolchain (to build from source)
- macOS, Linux, or Windows
- Chrome or Chromium installed (for JS rendering mode)

## Installation

### Pre-built binaries

Download the latest release from [GitHub Releases](https://github.com/PatrickJD/SiteScrape/releases). Releases are built and published automatically via GitHub Actions when a version tag is pushed.

| Platform | Archive |
|----------|---------|
| macOS Intel (x86_64) | `sitescrape-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon (aarch64) | `sitescrape-aarch64-apple-darwin.tar.gz` |
| Linux (x86_64) | `sitescrape-x86_64-unknown-linux-gnu.tar.gz` |
| Windows (x86_64) | `sitescrape-x86_64-pc-windows-msvc.zip` |

**macOS:**
```bash
tar xzf sitescrape-<target>.tar.gz
chmod +x sitescrape
sudo mv sitescrape /usr/local/bin/
```

> **macOS Gatekeeper notice:** The pre-built binaries are not signed with an Apple Developer certificate. macOS will block the binary the first time you run it. To allow it:
>
> 1. Run `sitescrape` in your terminal — macOS will show a dialog saying the app "cannot be opened because the developer cannot be verified." Click **Cancel**.
> 2. Open **System Settings → Privacy & Security**.
> 3. Scroll down to the **Security** section. You'll see a message about `sitescrape` being blocked. Click **Allow Anyway**.
> 4. Run `sitescrape` again. Click **Open** in the confirmation dialog.
>
> Alternatively, remove the quarantine attribute before first run:
> ```bash
> xattr -d com.apple.quarantine /usr/local/bin/sitescrape
> ```
> You only need to do this once per binary.

**Linux:**
```bash
tar xzf sitescrape-x86_64-unknown-linux-gnu.tar.gz
chmod +x sitescrape
mv sitescrape ~/.local/bin/   # or /usr/local/bin/ with sudo
```

**Windows:** Extract the zip and move `sitescrape.exe` to a directory in your PATH, or add its directory to PATH.

**Prerequisites:** Chrome or Chromium must be installed for browser mode (JS-rendered sites). Use `--no-browser` for HTTP-only mode.

**Platform notes:** Chrome cookie extraction is supported on macOS and Windows. Firefox cookies are supported on all platforms.

### Build from source

```bash
cd sitescrape
cargo build --release
# Binary at target/release/sitescrape
```

## Usage

```
Usage: sitescrape [OPTIONS] [URL]

Arguments:
  [URL]  URL to scrape

Options:
  -b, --browser <BROWSER>      Cookie source browser (auto, firefox, chrome)
  -o, --output <OUTPUT>        Output directory
  -m, --max-pages <MAX_PAGES>  Max pages to crawl
  -s, --selector <SELECTOR>    CSS selector for main content
  -p, --prefix <PREFIX>        URL path prefix filter
  -d, --delay <DELAY>          Delay between pages in ms
      --no-browser             HTTP-only mode (no JS rendering)
      --headless               Disable TUI, plain text output
      --config <CONFIG>        Config file path
      --profile <PROFILE>      Named profile from config file
  -h, --help                   Print help
  -V, --version                Print version
```

### Examples

```bash
# Basic usage — scrape with auto-detected browser cookies
sitescrape https://docs.internal.example.com/guide/

# Use Firefox cookies, limit to 100 pages
sitescrape https://docs.internal.example.com/ -b firefox -m 100

# Static site, no JS rendering needed
sitescrape https://docs.internal.example.com/ --no-browser

# Headless mode (no TUI, plain text progress)
sitescrape https://docs.internal.example.com/ --headless

# Custom content selector and output directory
sitescrape https://docs.internal.example.com/ -s ".main-content" -o ./docs

# Use a named profile from config file
sitescrape --profile mysite
```

## Interactive Mode

Launch without a URL to enter interactive mode:

```bash
sitescrape
```

This opens a persistent TUI where you can run multiple scrapes without restarting.

### Commands

| Command | Description |
|---------|-------------|
| `/scrape <url> [-m N] [-s sel] [-p pfx] [-d ms] [--no-browser]` | Start a scrape |
| `/stop` or `Esc` | Stop the current scrape |
| `/status` | Show current scrape progress |
| `/config` | Show current configuration |
| `/set <key> <value>` | Change a setting (output, delay, max_pages, selector) |
| `/cookies` | Reload cookies from browser |
| `/clear` | Clear the log |
| `/help` | Show available commands |
| `/quit` or `/q` | Exit |

```
# In the interactive TUI:
> /scrape https://docs.internal.example.com/guide/
> /set output ./docs
> /scrape https://other-site.example.com/ -m 50 -s ".content"
> /cookies
> /quit
```

## TUI Interface

The TUI uses a modern styled interface with color-coded log entries, Unicode status icons, a braille spinner during active scrapes, and a progress bar. Log entries are color-coded by outcome: green for saved pages, red for errors, yellow for auth failures, cyan for info messages.

| Icon | Meaning |
|------|---------|
| `✓` | Page saved |
| `✗` | Error |
| `⚠` | Auth failure |
| `◌` | Visiting (in progress) |
| `ℹ` | Info |

## TUI Keybindings

| Key | Context | Action |
|-----|---------|--------|
| `Enter` | Interactive | Run command |
| `Esc` | Interactive (idle) | Clear input |
| `Esc` | Interactive (scraping) | Stop scrape |
| `p` | One-shot TUI | Pause/resume |
| `↑` / `↓` | Both | Scroll log |
| `Ctrl+C` | Both | Quit |

## Config File

Place `sitescrape.toml` in the current directory, or specify a path with `--config`:

```toml
[default]
browser = "auto"
output = "./output"
max_pages = 500
delay = 500
selector = "main"

[profiles.mysite]
url = "https://docs.internal.example.com/guide/"
max_pages = 200
selector = ".content"
```

Precedence: CLI args > profile values > default values.

## Browser Support

- **Firefox**: Reads cookies from the default profile. macOS: `~/Library/Application Support/Firefox/Profiles/`, Windows: `%APPDATA%\Mozilla\Firefox\Profiles`.
- **Chrome**: Reads and decrypts cookies. macOS: Keychain (Chrome Safe Storage key) + AES-128-CBC. Windows: DPAPI + AES-256-GCM.
- **Auto-detection**: Detects the default browser. macOS: LaunchServices. Windows: registry (`UrlAssociations`). Falls back to Firefox if detection fails.

## Modes

- **Interactive mode** (default, no URL): Persistent TUI with command input — run multiple scrapes without restarting.
- **TUI mode** (with URL): One-shot scrape with progress display. Auto-activates when stdout is a TTY.
- **Headless mode** (`--headless` or piped output): Plain text progress with an indicatif progress bar. Use for scripting or CI.
- **Browser mode** (default): Uses headless Chrome via chromiumoxide for JS rendering. Requires Chrome/Chromium installed.
- **No-browser mode** (`--no-browser`): HTTP-only, no Chrome — faster for static sites that don't require JS rendering.

## Output Format

Each page is saved as a markdown file with YAML frontmatter:

```markdown
---
title: "Page Title"
url: "https://docs.internal.example.com/guide/getting-started"
---

# Getting Started
...
```

URL path segments are converted to Title-Case filenames (e.g., `/guide/getting-started` → `Guide/Getting-Started.md`).

## Known Limitations

- Chrome/Chromium must be installed for JS rendering mode
- Sequential crawling only — one page at a time with a configurable delay
- Browser must have written cookies to disk before running (cookie DB files are copied to a temp location for reading)

## Reporting Issues

If you run into a problem, please [open an issue](../../issues/new) and include:

- What you were trying to do and what happened instead
- The command you ran (redact any internal URLs)
- Your OS and architecture (e.g., macOS ARM64, Ubuntu 22.04 x86_64)
- The sitescrape version (`sitescrape --version`)
- Any error output from the terminal

Before opening a new issue, check [existing issues](../../issues) to see if it's already been reported.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on reporting bugs, submitting pull requests, and the code of conduct.
