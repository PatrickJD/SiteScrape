use super::{Cookie, SameSite};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use rusqlite::Connection;
use std::fs;
use tempfile::TempDir;

const CHROME_EPOCH_OFFSET: i64 = 11644473600;

fn validate_profile(name: &str) -> Result<()> {
    if name.contains('/') || name.contains('\\') || name.contains("..") || name.contains('\0') {
        return Err(eyre!("Invalid profile name"));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
/// # Safety
/// Calls CryptUnprotectData FFI. The input buffer is not mutated by the API
/// despite the *mut u8 signature — this is a Windows API convention.
unsafe fn dpapi_decrypt(ciphertext: &[u8]) -> Result<Vec<u8>> {
    use std::ffi::c_void;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

    // SAFETY: CryptUnprotectData does not mutate the input buffer per Microsoft docs.
    // The *mut u8 in CRYPT_INTEGER_BLOB is a C API convention, not a mutation guarantee.
    let mut input = CRYPT_INTEGER_BLOB {
        cbData: ciphertext.len() as u32,
        pbData: ciphertext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };

    let ok = CryptUnprotectData(
        &input as *const _,
        null_mut(),
        null(),
        null() as *const c_void,
        null(),
        0,
        &mut output,
    );
    if ok == 0 {
        return Err(eyre!("CryptUnprotectData failed"));
    }

    let result = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
    // Zero the DPAPI output buffer before freeing to prevent key material leaking in freed heap
    std::ptr::write_bytes(output.pbData, 0, output.cbData as usize);
    LocalFree(output.pbData as *mut _);
    Ok(result)
}

#[cfg(target_os = "windows")]
fn get_aes_key(base: &std::path::Path) -> Result<[u8; 32]> {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let local_state = base.join("Local State");
    let text =
        fs::read_to_string(&local_state).map_err(|e| eyre!("Cannot read Local State: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&text)?;

    let b64 = json["os_crypt"]["encrypted_key"]
        .as_str()
        .ok_or_else(|| eyre!("os_crypt.encrypted_key not found"))?;

    let raw = STANDARD.decode(b64)?;
    if raw.len() < 5 || &raw[..5] != b"DPAPI" {
        return Err(eyre!("Unexpected encrypted_key format"));
    }

    let mut key_bytes = unsafe { dpapi_decrypt(&raw[5..])? };
    if key_bytes.len() != 32 {
        key_bytes.fill(0);
        return Err(eyre!("Expected 32-byte AES key, got {}", key_bytes.len()));
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&key_bytes);
    key_bytes.fill(0);
    Ok(key)
}

#[cfg(target_os = "windows")]
fn decrypt_cookie(key: &[u8; 32], encrypted_value: &[u8]) -> Result<String> {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Key, Nonce};

    if encrypted_value.len() < 15 {
        return Ok(String::new());
    }
    let prefix = &encrypted_value[..3];
    if prefix != b"v10" && prefix != b"v20" {
        return Ok(String::from_utf8_lossy(encrypted_value).to_string());
    }

    let nonce = Nonce::from_slice(&encrypted_value[3..15]);
    let ciphertext_with_tag = &encrypted_value[15..];

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let plaintext = cipher
        .decrypt(nonce, ciphertext_with_tag)
        .map_err(|_| eyre!("AES-GCM decryption failed"))?;

    Ok(String::from_utf8_lossy(&plaintext).to_string())
}

pub fn extract(profile: Option<&str>) -> Result<Vec<Cookie>> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| eyre!("Cannot find LOCALAPPDATA"))?
        .join("Google/Chrome/User Data");

    #[cfg(target_os = "windows")]
    let mut key = get_aes_key(&base)?;

    let profile_name = profile.unwrap_or("Default");
    validate_profile(profile_name)?;

    let cookies_path = base.join(profile_name).join("Network/Cookies");
    if !cookies_path.exists() {
        return Err(eyre!("Chrome cookies not found at {:?}", cookies_path));
    }

    let tmp = TempDir::new()?;
    let tmp_db = tmp.path().join("Cookies");
    fs::copy(&cookies_path, &tmp_db)?;

    let conn = Connection::open(&tmp_db)?;
    let mut stmt = conn.prepare(
        "SELECT name, encrypted_value, host_key, path, expires_utc, is_secure, is_httponly, samesite FROM cookies",
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
                #[cfg(target_os = "windows")]
                let value = match decrypt_cookie(&key, &encrypted) {
                    Ok(v) => v,
                    Err(_) => {
                        eprintln!("Warning: failed to decrypt a cookie, skipping");
                        return None;
                    }
                };
                #[cfg(not(target_os = "windows"))]
                let value = String::new();

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

    #[cfg(target_os = "windows")]
    key.fill(0);

    Ok(cookies)
}
