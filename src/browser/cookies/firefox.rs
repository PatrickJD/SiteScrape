use super::{Cookie, SameSite};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

const MAX_EXPIRY_MS: i64 = 32503680000;

fn get_firefox_profiles_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("Library/Application Support/Firefox/Profiles")
    }
    #[cfg(target_os = "windows")]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("C:\\Users\\Default"))
            .join("Mozilla/Firefox/Profiles")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".mozilla/firefox")
    }
}

fn validate_profile_name(name: &str) -> color_eyre::Result<()> {
    if name.contains('/') || name.contains('\\') || name.contains("..") || name.contains('\0') {
        return Err(eyre!("Invalid profile name"));
    }
    Ok(())
}

fn find_profile(base_path: &PathBuf) -> color_eyre::Result<PathBuf> {
    let entries = fs::read_dir(base_path)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.contains("default-release") {
                    return Ok(path);
                }
            }
        }
    }
    let default = base_path.join("default");
    if default.exists() {
        Ok(default)
    } else {
        Err(eyre!("No Firefox profile found"))
    }
}

fn copy_db_to_temp(profile_path: &PathBuf) -> Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path().join("cookies.sqlite");

    let cookies_path = profile_path.join("cookies.sqlite");
    fs::copy(&cookies_path, &temp_path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o600))?;
    }

    let wal_path = profile_path.join("cookies.sqlite-wal");
    if wal_path.exists() {
        let wal_temp = temp_dir.path().join("cookies.sqlite-wal");
        fs::copy(&wal_path, &wal_temp)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&wal_temp, std::fs::Permissions::from_mode(0o600))?;
        }
    }
    let shm_path = profile_path.join("cookies.sqlite-shm");
    if shm_path.exists() {
        let shm_temp = temp_dir.path().join("cookies.sqlite-shm");
        fs::copy(&shm_path, &shm_temp)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&shm_temp, std::fs::Permissions::from_mode(0o600))?;
        }
    }

    Ok((temp_dir, temp_path))
}

fn normalize_expiry(expiry: i64) -> f64 {
    if expiry > MAX_EXPIRY_MS {
        (expiry / 1000) as f64
    } else if expiry <= 0 {
        -1.0
    } else {
        expiry as f64
    }
}

fn map_samesite(val: i32) -> SameSite {
    match val {
        1 => SameSite::Lax,
        2 => SameSite::Strict,
        _ => SameSite::None,
    }
}

pub fn extract(profile: Option<&str>) -> Result<Vec<Cookie>> {
    let base_path = get_firefox_profiles_path();

    let profile_name = profile.unwrap_or("");
    validate_profile_name(profile_name)?;

    let profile_path = if profile_name.is_empty() {
        find_profile(&base_path)?
    } else {
        base_path.join(profile_name)
    };

    let (_temp_dir, db_path) = copy_db_to_temp(&profile_path)?;

    let conn = Connection::open(&db_path)?;
    let mut stmt = conn.prepare(
        "SELECT name, value, host, path, expiry, isSecure, isHttpOnly, sameSite FROM moz_cookies"
    )?;

    let cookies = stmt.query_map([], |row| {
        Ok(Cookie {
            name: row.get(0)?,
            value: row.get(1)?,
            domain: row.get(2)?,
            path: row.get(3)?,
            expires: normalize_expiry(row.get(4)?),
            http_only: row.get::<_, i32>(6)? != 0,
            secure: row.get::<_, i32>(5)? != 0,
            same_site: map_samesite(row.get(7)?),
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(cookies)
}