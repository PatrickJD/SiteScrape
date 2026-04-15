# Installation

## Pre-built Binaries

Download the latest release from the [Releases](https://github.com/PatrickJD/SiteScrape/releases) page.

| Platform | Archive |
|----------|---------|
| macOS Intel (x86_64) | `sitescrape-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon (aarch64) | `sitescrape-aarch64-apple-darwin.tar.gz` |
| Linux (x86_64) | `sitescrape-x86_64-unknown-linux-gnu.tar.gz` |
| Windows (x86_64) | `sitescrape-x86_64-pc-windows-msvc.zip` |

### macOS

```bash
tar xzf sitescrape-<target>.tar.gz
chmod +x sitescrape
sudo mv sitescrape /usr/local/bin/
```

#### Gatekeeper (unsigned binary)

macOS blocks unsigned binaries on first run. You have two options:

**Option A — System Settings UI:**
1. Run `sitescrape` — macOS shows "cannot be opened because the developer cannot be verified." Click **Cancel**.
2. Open **System Settings → Privacy & Security**.
3. In the **Security** section, click **Allow Anyway** next to the sitescrape message.
4. Run `sitescrape` again and click **Open**.

**Option B — Terminal (faster):**
```bash
xattr -d com.apple.quarantine /usr/local/bin/sitescrape
```

You only need to do this once per binary.

### Linux

```bash
tar xzf sitescrape-x86_64-unknown-linux-gnu.tar.gz
chmod +x sitescrape
mv sitescrape ~/.local/bin/   # or /usr/local/bin/ with sudo
```

### Windows

Extract the zip and move `sitescrape.exe` to a directory in your PATH.

## Build from Source

Requires the [Rust toolchain](https://rustup.rs/).

```bash
git clone https://github.com/PatrickJD/SiteScrape.git
cd sitescrape
cargo build --release
# Binary at target/release/sitescrape
```

## Prerequisites

- **Chrome or Chromium** — required for browser mode (JS-rendered sites). Not needed with `--no-browser`.
- Chrome cookie extraction is supported on macOS and Windows. Firefox cookies work on all platforms.
