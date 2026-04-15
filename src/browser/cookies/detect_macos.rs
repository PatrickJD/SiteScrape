use super::Browser;
use color_eyre::Result;
use std::process::Command;

pub fn detect() -> Result<Browser> {
    let output = Command::new("defaults")
        .args([
            "read",
            "com.apple.LaunchServices/com.apple.launchservices.secure",
            "LSHandlers",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse plist dict blocks delimited by { ... }
    // Each block may contain LSHandlerRoleAll and LSHandlerURLScheme in any order
    let mut role_all = String::new();
    let mut url_scheme = String::new();
    let mut depth = 0usize;

    for line in stdout.lines() {
        let trimmed = line.trim();

        if trimmed == "{" {
            depth += 1;
            if depth == 1 {
                role_all.clear();
                url_scheme.clear();
            }
            continue;
        }

        if trimmed.starts_with("},") || trimmed == "}" {
            if depth == 1 && url_scheme == "https" {
                if let Some(b) = match_browser(&role_all) {
                    return Ok(b);
                }
            }
            depth = depth.saturating_sub(1);
            continue;
        }

        // Only parse top-level keys (depth == 1), skip nested dicts
        if depth != 1 {
            continue;
        }

        // Only match top-level keys (skip nested dicts like LSHandlerPreferredVersions)
        if let Some(val) = extract_plist_value(trimmed, "LSHandlerURLScheme") {
            url_scheme = val;
        } else if let Some(val) = extract_plist_value(trimmed, "LSHandlerRoleAll") {
            role_all = val;
        }
    }

    eprintln!("Warning: Could not detect default browser, defaulting to Firefox");
    Ok(Browser::Firefox)
}

fn extract_plist_value(line: &str, key: &str) -> Option<String> {
    if !line.starts_with(key) {
        return None;
    }
    let val = line.split('=').nth(1)?;
    Some(val.trim().trim_end_matches(';').trim().trim_matches('"').to_string())
}

fn match_browser(role_all: &str) -> Option<Browser> {
    let lower = role_all.to_lowercase();
    if lower.contains("chrome") || lower.contains("com.google.chrome") {
        Some(Browser::Chrome)
    } else if lower.contains("firefox") || lower.contains("org.mozilla.firefox") {
        Some(Browser::Firefox)
    } else {
        None
    }
}