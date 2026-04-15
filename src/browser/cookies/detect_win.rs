use super::Browser;
use color_eyre::Result;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, KEY_READ,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn detect() -> Result<Browser> {
    let subkey = wide(r"Software\Microsoft\Windows\Shell\Associations\UrlAssociations\https\UserChoice");
    let value_name = wide("ProgId");

    let prog_id = unsafe {
        let mut hkey = null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey)
            != ERROR_SUCCESS
        {
            return fallback("Could not open UserChoice registry key");
        }

        let mut buf = vec![0u8; 512];
        let mut size = buf.len() as u32;
        let ret = RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            null(),
            null_mut(),
            buf.as_mut_ptr(),
            &mut size,
        );
        RegCloseKey(hkey);

        if ret != ERROR_SUCCESS {
            return fallback("Could not read ProgId registry value");
        }

        let words: Vec<u16> = buf[..size as usize]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&words)
            .trim_end_matches('\0')
            .to_string()
    };

    Ok(if prog_id == "ChromeHTML" {
        Browser::Chrome
    } else if prog_id.starts_with("FirefoxURL") {
        Browser::Firefox
    } else {
        eprintln!("Warning: Unknown ProgId '{prog_id}', defaulting to Firefox");
        Browser::Firefox
    })
}

fn fallback(msg: &str) -> Result<Browser> {
    eprintln!("Warning: {msg}, defaulting to Firefox");
    Ok(Browser::Firefox)
}
