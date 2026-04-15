#[cfg(target_os = "macos")]
mod chrome_macos;
#[cfg(target_os = "windows")]
mod chrome_win;
#[cfg(target_os = "macos")]
mod detect_macos;
#[cfg(target_os = "windows")]
mod detect_win;
pub mod firefox;

#[derive(Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: f64,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: SameSite,
}

#[derive(Debug, Clone)]
pub enum SameSite {
    None,
    Lax,
    Strict,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Browser {
    Firefox,
    Chrome,
}

pub fn extract_cookies(browser: &Browser, profile: Option<&str>) -> color_eyre::eyre::Result<Vec<Cookie>> {
    match browser {
        Browser::Firefox => firefox::extract(profile),
        #[cfg(target_os = "macos")]
        Browser::Chrome => chrome_macos::extract(profile),
        #[cfg(target_os = "windows")]
        Browser::Chrome => chrome_win::extract(profile),
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        Browser::Chrome => Err(color_eyre::eyre::eyre!("Chrome cookie extraction is not supported on this platform. Use Firefox or --no-browser mode.")),
    }
}

pub fn detect() -> color_eyre::eyre::Result<Browser> {
    #[cfg(target_os = "macos")]
    { detect_macos::detect() }
    #[cfg(target_os = "windows")]
    { detect_win::detect() }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    { Ok(Browser::Firefox) }
}
