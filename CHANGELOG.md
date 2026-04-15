# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Changed
- Updated README for public release — added macOS Gatekeeper instructions, issue reporting guidelines
- Added CONTRIBUTING.md, LICENSE (MIT-0), and CHANGELOG.md
- Migrated CI/CD from GitLab CI to GitHub Actions
- Added CI workflow (fmt, clippy, test, build on Linux)
- Added Release workflow (4-target matrix: macOS ARM/Intel, Linux, Windows)
- Added Wiki sync workflow (auto-publishes docs/wiki/ to GitHub Wiki)
- Added README badges (CI status, release version, license, platforms)
- Changed Windows build target from x86_64-pc-windows-gnu to x86_64-pc-windows-msvc
- Simplified release.sh to tag-and-push (builds now handled by GitHub Actions)
- Replaced scripts/sync-wiki.sh with GitHub Actions wiki-sync workflow

## [0.1.0] - 2026-04-14

Initial release.

### Added
- CLI scraper with BFS crawling and HTML-to-Markdown conversion
- Browser cookie extraction (Chrome on macOS/Windows, Firefox on all platforms)
- Headless Chrome rendering via chromiumoxide for JS-heavy sites
- `--no-browser` HTTP-only mode for static sites
- Interactive TUI mode with command input, log scrolling, and progress display
- One-shot TUI mode with pause/resume and braille spinner
- Headless mode with indicatif progress bar for CI/scripting
- TOML config file with named profiles
- CSS selector filtering for main content extraction
- URL prefix filtering to scope crawls
- YAML frontmatter in output markdown files
- Pre-built binaries for macOS (Intel + Apple Silicon), Linux, and Windows
