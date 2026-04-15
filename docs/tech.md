# Tech Reference

Verified API patterns for the chromiumoxide migration.
Verified: 2026-04-13 against crate versions pinned below.

---

## chromiumoxide v0.8.0

**Cargo.toml**: `chromiumoxide = "=0.8.0"`

### Imports

```rust
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::network::CookieParam;
use futures::StreamExt; // required for Handler polling
```

### Browser launch

`Browser::launch` is async and returns `(Browser, Handler)`. The `Handler` **must** be driven on a separate task or the browser will deadlock.

```rust
let (browser, mut handler) = Browser::launch(config).await?;

tokio::spawn(async move {
    while let Some(h) = handler.next().await {
        if h.is_err() {
            break;
        }
    }
});
```

Source: <https://docs.rs/chromiumoxide/0.8.0/chromiumoxide/browser/struct.Browser.html#method.launch>

### BrowserConfig builder

`.build()` returns `Result<BrowserConfig, String>`.

```rust
let config = BrowserConfig::builder()
    .with_head()          // optional: show browser window
    .build()
    .map_err(|e| anyhow::anyhow!(e))?;
```

Source: <https://docs.rs/chromiumoxide/0.8.0/chromiumoxide/browser/struct.BrowserConfig.html>

### new_page

Creates a new tab and navigates to the URL. Returns `Page`.

```rust
let page = browser.new_page("https://example.com").await?;
```

Source: <https://docs.rs/chromiumoxide/0.8.0/chromiumoxide/browser/struct.Browser.html#method.new_page>

### page.content

Returns the full HTML of the current page as `String`.

```rust
let html: String = page.content().await?;
```

Source: <https://docs.rs/chromiumoxide/0.8.0/chromiumoxide/page/struct.Page.html#method.content>

### page.evaluate

Evaluates a JS expression. Returns `EvaluationResult`. Extract a typed value with `.into_value::<T>()`.

```rust
let result = page.evaluate("document.title").await?;
let title: String = result.into_value::<String>()?;
```

Source: <https://docs.rs/chromiumoxide/0.8.0/chromiumoxide/page/struct.Page.html#method.evaluate>

### CookieParam builder

```rust
use chromiumoxide::cdp::browser_protocol::network::CookieParam;

let cookie = CookieParam::builder()
    .name("session")
    .value("abc123")
    .domain("example.com")
    .path("/")
    .secure(true)
    .http_only(true)
    // .expires(unix_timestamp_f64)  // optional
    .build()?;

browser.set_cookies(vec![cookie]).await?;
```

Source: <https://docs.rs/chromiumoxide/0.8.0/chromiumoxide/cdp/browser_protocol/network/struct.CookieParam.html>

---

## async-trait v0.1.88

**Cargo.toml**: `async-trait = "=0.1.88"`

### Import

```rust
use async_trait::async_trait;
```

### Usage

Apply `#[async_trait]` to **both** the trait definition and every `impl` block. Add `Send + Sync` bounds when the trait is used across tokio tasks.

```rust
#[async_trait]
pub trait Navigate: Send + Sync {
    async fn navigate(&self, url: &str) -> Result<PageResult>;
}

#[async_trait]
impl Navigate for Engine {
    async fn navigate(&self, url: &str) -> Result<PageResult> {
        // ...
    }
}
```

Source: <https://docs.rs/async-trait/0.1.88/async_trait/>

---

## reqwest v0.12.18 (async)

**Cargo.toml**: `reqwest = "=0.12.18"` (use async client, not `reqwest::blocking`)

### Import

```rust
use reqwest::Client;
use reqwest::cookie::Jar;
use std::sync::Arc;
```

### Client construction with cookie jar

```rust
let jar = Arc::new(Jar::default());
let client = Client::builder()
    .cookie_provider(jar.clone())
    .user_agent("Mozilla/5.0 ...")
    .build()?;
```

### Async request / response

```rust
let resp = client.get(url).send().await?;
let body: String = resp.text().await?;
```

Source: <https://docs.rs/reqwest/0.12.18/reqwest/struct.Client.html>

---

## Windows Platform APIs

Verified: 2026-04-13 against crate versions pinned below.

---

## windows-sys v0.59.0 — DPAPI (`Win32_Security_Cryptography`, `Win32_Foundation`)

**Cargo.toml** (Windows-conditional):
```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows-sys = { version = "=0.59.0", features = [
    "Win32_Security_Cryptography",
    "Win32_Foundation",
    "Win32_System_Registry",
] }
```

### `CRYPT_INTEGER_BLOB` struct

Also aliased as `DATA_BLOB` in Win32 headers. In `windows-sys` it is always `CRYPT_INTEGER_BLOB`.

```rust
// windows_sys::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB
#[repr(C)]
pub struct CRYPT_INTEGER_BLOB {
    pub cbData: u32,       // byte length of pbData
    pub pbData: *mut u8,   // pointer to data buffer
}
```

Source: <https://docs.rs/windows-sys/0.59.0/windows_sys/Win32/Security/Cryptography/struct.CRYPT_INTEGER_BLOB.html>

### `CryptUnprotectData` function signature

```rust
pub unsafe extern "system" fn CryptUnprotectData(
    pdatain: *const CRYPT_INTEGER_BLOB,       // encrypted input blob
    ppszdatadescr: *mut PWSTR,                // optional: description string out (pass null_mut())
    poptionalentropy: *const CRYPT_INTEGER_BLOB, // optional entropy (pass null())
    pvreserved: *const c_void,                // reserved, must be null()
    ppromptstruct: *const CRYPTPROTECT_PROMPTSTRUCT, // UI prompt (pass null())
    dwflags: u32,                             // 0 for default
    pdataout: *mut CRYPT_INTEGER_BLOB,        // decrypted output blob (caller must LocalFree pbData)
) -> BOOL  // TRUE on success
```

Source: <https://docs.rs/windows-sys/0.59.0/windows_sys/Win32/Security/Cryptography/fn.CryptUnprotectData.html>

### Imports

```rust
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};
use windows_sys::Win32::Foundation::LocalFree;
```

### Usage pattern — decrypt DPAPI blob

```rust
unsafe fn dpapi_decrypt(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    let mut input = CRYPT_INTEGER_BLOB {
        cbData: ciphertext.len() as u32,
        pbData: ciphertext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: null_mut() };

    let ok = CryptUnprotectData(
        &input,
        null_mut(),
        null(),
        null(),
        null(),
        0,
        &mut output,
    );
    if ok == 0 {
        return Err("CryptUnprotectData failed".into());
    }

    let result = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
    LocalFree(output.pbData as *mut _);  // must free the output buffer
    Ok(result)
}
```

---

## Chrome Local State — Encrypted Key Format

**File path**: `%LOCALAPPDATA%\Google\Chrome\User Data\Local State`

**JSON field**: `os_crypt.encrypted_key`

**Format**:
1. Base64-decode the value
2. First 5 bytes are ASCII `DPAPI` (magic prefix — strip them)
3. Remaining bytes are the DPAPI-encrypted AES key blob
4. Pass bytes `[5..]` to `CryptUnprotectData`
5. Output is a 32-byte AES-256-GCM key

```rust
use base64::{engine::general_purpose::STANDARD, Engine};

let raw = STANDARD.decode(&encrypted_key_b64)?;
assert_eq!(&raw[..5], b"DPAPI");
let dpapi_blob = &raw[5..];
let aes_key: Vec<u8> = dpapi_decrypt(dpapi_blob)?;  // 32 bytes
assert_eq!(aes_key.len(), 32);
```

Source: Chromium source — `components/os_crypt/sync/os_crypt_win.cc`

---

## AES-256-GCM Cookie Decryption (`aes-gcm` v0.10.x)

**Cargo.toml** (Windows-conditional):
```toml
[target.'cfg(target_os = "windows")'.dependencies]
aes-gcm = { version = "=0.10.3", features = ["aes"] }
```

### Cookie value byte layout

| Bytes | Content |
|-------|---------|
| `[0..3]` | Prefix: `v10` or `v20` (3 bytes, ASCII) |
| `[3..15]` | 12-byte GCM nonce |
| `[15..end]` | Ciphertext + 16-byte GCM auth tag (tag is the last 16 bytes, appended by Chrome) |

### Imports

```rust
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
```

### Decryption pattern

```rust
fn decrypt_cookie(key: &[u8; 32], encrypted_value: &[u8]) -> Result<Vec<u8>, aes_gcm::Error> {
    // Strip v10/v20 prefix (3 bytes)
    let data = &encrypted_value[3..];
    let nonce = Nonce::from_slice(&data[..12]);       // bytes 3..15 of original
    let ciphertext_with_tag = &data[12..];            // bytes 15..end of original

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    cipher.decrypt(nonce, ciphertext_with_tag)
}
```

Note: `aes-gcm` v0.10.x `decrypt()` expects ciphertext with the 16-byte tag appended (Chrome's format). It returns the plaintext on success or `aes_gcm::Error` on auth failure.

Source: <https://docs.rs/aes-gcm/0.10.3/aes_gcm/>

---

## Windows Registry API — Browser Detection (`Win32_System_Registry`)

### Function signatures

```rust
pub unsafe extern "system" fn RegOpenKeyExW(
    hkey: HKEY,              // root key, e.g. HKEY_CURRENT_USER
    lpsubkey: PCWSTR,        // subkey path as wide string
    uloptions: u32,          // 0
    samdesired: REG_SAM_FLAGS, // KEY_READ (= 0x20019)
    phkresult: *mut HKEY,    // out: opened key handle
) -> WIN32_ERROR             // ERROR_SUCCESS (0) on success

pub unsafe extern "system" fn RegQueryValueExW(
    hkey: HKEY,              // opened key handle
    lpvaluename: PCWSTR,     // value name as wide string
    lpreserved: *const u32,  // null
    lptype: *mut REG_VALUE_TYPE, // out: value type (REG_SZ = 1)
    lpdata: *mut u8,         // out: data buffer
    lpcbdata: *mut u32,      // in/out: buffer size in bytes
) -> WIN32_ERROR

pub unsafe extern "system" fn RegCloseKey(hkey: HKEY) -> WIN32_ERROR
```

Sources:
- <https://docs.rs/windows-sys/0.59.0/windows_sys/Win32/System/Registry/fn.RegOpenKeyExW.html>
- <https://docs.rs/windows-sys/0.59.0/windows_sys/Win32/System/Registry/fn.RegQueryValueExW.html>

### Registry path for default browser

```
HKCU\Software\Microsoft\Windows\Shell\Associations\UrlAssociations\https\UserChoice
Value: ProgId  (REG_SZ)
```

ProgId values:
- `ChromeHTML` → Chrome
- `FirefoxURL-*` (prefix match) → Firefox
- anything else → fallback

### Imports

```rust
use windows_sys::Win32::System::Registry::{
    RegOpenKeyExW, RegQueryValueExW, RegCloseKey,
    HKEY_CURRENT_USER, KEY_READ,
};
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
```

### Usage pattern — read ProgId

```rust
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn read_default_browser_progid() -> Option<String> {
    let subkey = wide(r"Software\Microsoft\Windows\Shell\Associations\UrlAssociations\https\UserChoice");
    let value_name = wide("ProgId");

    let mut hkey = 0isize;
    if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != ERROR_SUCCESS {
        return None;
    }

    let mut buf = vec![0u8; 512];
    let mut size = buf.len() as u32;
    let ret = RegQueryValueExW(hkey, value_name.as_ptr(), null(), null_mut(), buf.as_mut_ptr(), &mut size);
    RegCloseKey(hkey);

    if ret != ERROR_SUCCESS { return None; }

    // buf contains UTF-16LE, size is byte count including null terminator
    let words: Vec<u16> = buf[..size as usize]
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    Some(String::from_utf16_lossy(&words).trim_end_matches('\0').to_string())
}
```

---

## Windows Platform Paths

| Resource | Path |
|----------|------|
| Chrome Local State | `%LOCALAPPDATA%\Google\Chrome\User Data\Local State` |
| Chrome Cookies (default profile) | `%LOCALAPPDATA%\Google\Chrome\User Data\Default\Network\Cookies` |
| Chrome Cookies (named profile) | `%LOCALAPPDATA%\Google\Chrome\User Data\{profile}\Network\Cookies` |
| Firefox profiles directory | `%APPDATA%\Mozilla\Firefox\Profiles` |

In Rust, resolve via `dirs::data_local_dir()` (`%LOCALAPPDATA%`) and `dirs::data_dir()` (`%APPDATA%`).

---

## GitHub Actions — Verified Action Patterns

Verified: 2026-04-15 against action repos on github.com.

### actions/checkout

- **Version**: `@v4`
- **Usage**: `uses: actions/checkout@v4`
- **Inputs**: none required for basic usage
- **Source**: https://github.com/actions/checkout

### dtolnay/rust-toolchain

- **Version**: `@stable` (rev-based, not semver-tagged)
- **Usage**: `uses: dtolnay/rust-toolchain@stable`
- **Inputs** (all optional):
  - `toolchain`: Rustup toolchain specifier (default: derived from @rev, e.g. `@stable`, `@nightly`, `@1.89.0`)
  - `targets`: Comma-separated additional targets (e.g. `x86_64-apple-darwin`)
  - `components`: Comma-separated additional components (e.g. `clippy, rustfmt`)
- **Outputs**: `cachekey`, `name`
- **Note**: Toolchain is selected by the @rev suffix, NOT by the `toolchain` input. Use `@stable` for stable, `@nightly` for nightly.
- **Source**: https://github.com/dtolnay/rust-toolchain

### Swatinem/rust-cache

- **Version**: `@v2`
- **Usage**: `uses: Swatinem/rust-cache@v2`
- **Inputs** (all optional):
  - `prefix-key`: Cache key prefix (default: `v0-rust`)
  - `shared-key`: Stable key across jobs
  - `key`: Additional differentiator key
- **Caches**: `~/.cargo` (registry, cache, git deps) + `./target` (dependency build artifacts)
- **Auto-keyed by**: job_id, rustc version, Cargo.lock/Cargo.toml hashes
- **Note**: Sets `CARGO_INCREMENTAL=0` automatically. Works around macOS cache corruption.
- **Source**: https://github.com/Swatinem/rust-cache

### softprops/action-gh-release

- **Version**: `@v2` (v3 requires Node 24; use v2 for broader compatibility)
- **Usage**: `uses: softprops/action-gh-release@v2`
- **Key inputs** (all optional):
  - `files`: Newline-delimited globs of asset paths to upload
  - `generate_release_notes`: Boolean — auto-generate release notes from commits
  - `name`: Release title (default: tag name)
  - `tag_name`: Tag name (default: `github.ref_name`)
  - `draft`: Boolean — keep as draft
  - `prerelease`: Boolean
  - `fail_on_unmatched_files`: Boolean
  - `token`: GitHub token (default: `github.token`)
- **Outputs**: `url`, `id`, `upload_url`, `assets`
- **Permissions required**: `contents: write`
- **Source**: https://github.com/softprops/action-gh-release

### Andrew-Chen-Wang/github-wiki-action

- **Version**: `@v4`
- **Usage**: `uses: Andrew-Chen-Wang/github-wiki-action@v4`
- **Key inputs** (all optional):
  - `path`: Directory containing wiki files (default: `wiki/`)
  - `token`: GitHub token (default: `github.token`)
  - `strategy`: `clone` (default) or `init` (force-push)
  - `commit-message`: Commit message for wiki update
  - `repository`: Target repo (default: current repo)
  - `ignore`: Multiline list of files to ignore
- **Outputs**: `wiki_url`
- **Permissions required**: `contents: write`
- **Prerequisite**: Wiki must be manually bootstrapped — create one page via GitHub UI first.
- **Source**: https://github.com/Andrew-Chen-Wang/github-wiki-action
