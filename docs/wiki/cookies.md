# Cookie Extraction

SiteScrape reads cookies from your browser's cookie database to authenticate against internal sites. No credentials are stored or transmitted — it reads the same cookies your browser has already saved.

## How It Works

1. SiteScrape detects your default browser (or uses the one you specify with `-b`)
2. It copies the cookie database file to a temp location (to avoid locking the live DB)
3. It reads cookies for the target domain
4. Cookies are injected into the scraping session (headless Chrome or HTTP client)

## Supported Browsers

| Browser | macOS | Linux | Windows |
|---------|-------|-------|---------|
| Firefox | ✓ | ✓ | ✓ |
| Chrome | ✓ | — | ✓ |

### Firefox

Reads from the default profile's `cookies.sqlite` file.

- macOS: `~/Library/Application Support/Firefox/Profiles/`
- Linux: `~/.mozilla/firefox/`
- Windows: `%APPDATA%\Mozilla\Firefox\Profiles\`

### Chrome

Reads and decrypts cookies from Chrome's cookie database.

- macOS: Decrypts using the Chrome Safe Storage key from Keychain (AES-128-CBC)
- Windows: Decrypts using DPAPI + AES-256-GCM

Chrome cookie extraction on Linux is not currently supported.

## Auto-Detection

With `-b auto` (the default), SiteScrape detects your default browser:

- macOS: Queries LaunchServices for the default HTTP handler
- Windows: Reads the `UrlAssociations` registry key
- Falls back to Firefox if detection fails

## Tips

- Make sure you've logged into the target site in your browser before running SiteScrape
- Close the browser or ensure cookies have been flushed to disk — some browsers delay writing cookies
- If you get authentication errors, try `-b firefox` or `-b chrome` explicitly
- Use `/cookies` in interactive mode to reload cookies without restarting
