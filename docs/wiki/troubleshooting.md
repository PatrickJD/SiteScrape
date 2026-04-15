# Troubleshooting

## "cannot be opened because the developer cannot be verified" (macOS)

The binary is not signed with an Apple Developer certificate. See [Installation — Gatekeeper](installation#gatekeeper-unsigned-binary) for the fix.

## Authentication errors / pages return login page

- Make sure you're logged into the target site in your browser
- Cookies may not have been flushed to disk — close the browser tab or restart the browser, then retry
- Try specifying the browser explicitly: `-b firefox` or `-b chrome`
- In interactive mode, run `/cookies` to reload cookies

## "Chrome/Chromium not found"

Browser mode requires Chrome or Chromium installed. Either:
- Install Chrome, or
- Use `--no-browser` for HTTP-only mode (no JS rendering)

## Pages are missing content

- The default scrape captures the full page. Use `-s` to target a specific CSS selector (e.g., `-s ".main-content"`, `-s "article"`)
- Some sites load content dynamically — make sure you're using browser mode (the default), not `--no-browser`

## Scrape stops early / misses pages

- Check `-m` (max pages) — the default is 500
- Use `-p` to set a URL prefix filter if the crawler is wandering to unrelated sections
- Increase `--delay` if the site is rate-limiting you

## Chrome cookie decryption fails (macOS)

SiteScrape reads the Chrome Safe Storage key from Keychain. If macOS prompts you to allow access, click **Allow** (or **Always Allow**). If you previously denied access, open Keychain Access, find "Chrome Safe Storage", and delete it — Chrome will recreate it on next launch.

## Output files have garbled names

URL path segments are converted to Title-Case filenames. If the site uses encoded characters or very long paths, filenames may look odd. This is cosmetic — the content is correct.

## Still stuck?

[Open an issue](../../issues/new) with:
- The command you ran (redact internal URLs)
- Your OS and architecture
- SiteScrape version (`sitescrape --version`)
- The error output
