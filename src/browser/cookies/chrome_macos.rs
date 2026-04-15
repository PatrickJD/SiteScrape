use super::{Cookie, SameSite};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use rusqlite::Connection;
use security_framework::passwords::get_generic_password;
use std::fs;
use tempfile::TempDir;

const CHROME_EPOCH_OFFSET: i64 = 11644473600;
const KEY_LEN: usize = 16;

fn validate_profile(name: &str) -> Result<()> {
    if name.contains('/') || name.contains('\\') || name.contains("..") || name.contains('\0') {
        return Err(eyre!("Invalid profile name"));
    }
    Ok(())
}

fn derive_key(password: &[u8]) -> [u8; KEY_LEN] {
    use hmac::Hmac;
    use sha1::Sha1;
    let mut key = [0u8; KEY_LEN];
    pbkdf2::pbkdf2::<Hmac<Sha1>>(password, b"saltysalt", 1003, &mut key)
        .expect("HMAC can be initialized with any key length");
    key
}

fn decrypt_value(encrypted: &[u8], key: &[u8; KEY_LEN]) -> Result<String> {
    if encrypted.len() <= 3 {
        return Ok(String::new());
    }
    let prefix = &encrypted[..3];
    if prefix != b"v10" && prefix != b"v11" {
        return Ok(String::from_utf8_lossy(encrypted).to_string());
    }
    let iv = [0x20u8; KEY_LEN];
    let mut buf = encrypted[3..].to_vec();

    use cbc::cipher::{block_padding::Pkcs7, BlockModeDecrypt, KeyIvInit};
    type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

    let dec = Aes128CbcDec::new_from_slices(key, &iv)
        .map_err(|e| color_eyre::eyre::eyre!("AES init failed: {}", e))?;
    let pt = dec
        .decrypt_padded::<Pkcs7>(&mut buf)
        .map_err(|e| color_eyre::eyre::eyre!("Decryption failed: {}", e))?;
    Ok(String::from_utf8_lossy(pt).to_string())
}

pub fn extract(profile: Option<&str>) -> Result<Vec<Cookie>> {
    let base = dirs::home_dir()
        .ok_or_else(|| eyre!("Cannot find home directory"))?
        .join("Library/Application Support/Google/Chrome");

    let profile_name = profile.unwrap_or("Default");
    validate_profile(profile_name)?;

    let cookies_path = base.join(profile_name).join("Cookies");
    if !cookies_path.exists() {
        return Err(eyre!("Chrome cookies not found at {:?}", cookies_path));
    }

    // Copy DB to temp
    let tmp = TempDir::new()?;
    let tmp_db = tmp.path().join("Cookies");
    fs::copy(&cookies_path, &tmp_db)?;
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_db, std::fs::Permissions::from_mode(0o600))?;
    }

    // Get decryption key from Keychain
    let mut password = get_generic_password("Chrome Safe Storage", "Chrome")
        .map_err(|e| eyre!("Keychain access failed: {}", e))?;
    let mut key = derive_key(&password);
    // Zero the password bytes immediately after key derivation
    password.fill(0);

    let conn = Connection::open(&tmp_db)?;
    let mut stmt = conn.prepare(
        "SELECT name, encrypted_value, host_key, path, expires_utc, is_secure, is_httponly, samesite FROM cookies"
    )?;

    let cookies: Vec<Cookie> = stmt
        .query_map([], |row| {
            let name: String = row.get(0)?;
            let encrypted: Vec<u8> = row.get(1)?;
            let domain: String = row.get(2)?;
            let path: String = row.get(3)?;
            let expires_utc: i64 = row.get(4)?;
            let is_secure: i32 = row.get(5)?;
            let is_httponly: i32 = row.get(6)?;
            let samesite: i32 = row.get(7)?;
            Ok((
                name,
                encrypted,
                domain,
                path,
                expires_utc,
                is_secure,
                is_httponly,
                samesite,
            ))
        })?
        .filter_map(|r| r.ok())
        .filter_map(
            |(name, encrypted, domain, path, expires_utc, is_secure, is_httponly, samesite)| {
                let value = match decrypt_value(&encrypted, &key) {
                    Ok(v) => v,
                    Err(_) => {
                        // Note: cookie values are not zeroized on drop; acceptable for a local CLI tool.
                        eprintln!("Warning: failed to decrypt a cookie, skipping");
                        return None;
                    }
                };
                Some(Cookie {
                    name,
                    value,
                    domain,
                    path,
                    expires: if expires_utc <= 0 {
                        -1.0
                    } else {
                        (expires_utc / 1_000_000 - CHROME_EPOCH_OFFSET) as f64
                    },
                    secure: is_secure != 0,
                    http_only: is_httponly != 0,
                    same_site: match samesite {
                        1 => SameSite::Lax,
                        2 => SameSite::Strict,
                        _ => SameSite::None,
                    },
                })
            },
        )
        .collect();

    // Zero the key
    key.fill(0);

    Ok(cookies)
}
