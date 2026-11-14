use serde::{Deserialize, Serialize};
use std::ffi::c_void;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub remember: bool,
    #[serde(default)]
    pub is_bot: bool,
}

fn data_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("DiscordPhoenix")
}

fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

fn token_path() -> PathBuf {
    data_dir().join("token.bin")
}

pub fn load_settings() -> Settings {
    fs::read_to_string(settings_path())
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_settings(settings: &Settings) {
    let _ = fs::create_dir_all(data_dir());
    if let Ok(raw) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(settings_path(), raw);
    }
}

pub fn save_token(token: &str) {
    let Some(encrypted) = protect(token.as_bytes()) else {
        return;
    };
    let _ = fs::create_dir_all(data_dir());
    let _ = fs::write(token_path(), encrypted);
}

pub fn load_token() -> Option<String> {
    let encrypted = fs::read(token_path()).ok()?;
    let plain = unprotect(&encrypted)?;
    String::from_utf8(plain).ok().filter(|s| !s.is_empty())
}

pub fn clear_token() {
    let _ = fs::remove_file(token_path());
}

// ---------------------------------------------------------------------------
// DPAPI (Windows Data Protection API)
// ---------------------------------------------------------------------------

#[repr(C)]
struct DataBlob {
    cb_data: u32,
    pb_data: *mut u8,
}

const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

#[link(name = "crypt32")]
extern "system" {
    fn CryptProtectData(
        data_in: *const DataBlob,
        description: *const u16,
        entropy: *const DataBlob,
        reserved: *mut c_void,
        prompt: *mut c_void,
        flags: u32,
        data_out: *mut DataBlob,
    ) -> i32;

    fn CryptUnprotectData(
        data_in: *const DataBlob,
        description: *mut *mut u16,
        entropy: *const DataBlob,
        reserved: *mut c_void,
        prompt: *mut c_void,
        flags: u32,
        data_out: *mut DataBlob,
    ) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn LocalFree(mem: *mut c_void) -> *mut c_void;
}

fn protect(plain: &[u8]) -> Option<Vec<u8>> {
    let input = DataBlob {
        cb_data: plain.len() as u32,
        pb_data: plain.as_ptr() as *mut u8,
    };
    let mut output = DataBlob {
        cb_data: 0,
        pb_data: std::ptr::null_mut(),
    };

    let ok = unsafe {
        CryptProtectData(
            &input,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 || output.pb_data.is_null() {
        return None;
    }

    let result = unsafe {
        let slice = std::slice::from_raw_parts(output.pb_data, output.cb_data as usize);
        slice.to_vec()
    };
    unsafe {
        LocalFree(output.pb_data as *mut c_void);
    }
    Some(result)
}

fn unprotect(encrypted: &[u8]) -> Option<Vec<u8>> {
    let input = DataBlob {
        cb_data: encrypted.len() as u32,
        pb_data: encrypted.as_ptr() as *mut u8,
    };
    let mut output = DataBlob {
        cb_data: 0,
        pb_data: std::ptr::null_mut(),
    };

    let ok = unsafe {
        CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 || output.pb_data.is_null() {
        return None;
    }

    let result = unsafe {
        let slice = std::slice::from_raw_parts(output.pb_data, output.cb_data as usize);
        slice.to_vec()
    };
    unsafe {
        LocalFree(output.pb_data as *mut c_void);
    }
    Some(result)
}

// ---------------------------------------------------------------------------
// Memória do processo (RAM real, sem depender de crates extras)
// ---------------------------------------------------------------------------

#[repr(C)]
struct ProcessMemoryCounters {
    cb: u32,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentProcess() -> isize;
}

#[link(name = "psapi")]
extern "system" {
    fn GetProcessMemoryInfo(
        process: isize,
        counters: *mut ProcessMemoryCounters,
        cb: u32,
    ) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpapi_roundtrip() {
        let secret = b"token-secreto-de-teste-123";
        let encrypted = protect(secret).expect("CryptProtectData falhou");
        assert_ne!(encrypted, secret);
        let plain = unprotect(&encrypted).expect("CryptUnprotectData falhou");
        assert_eq!(plain, secret);
    }

    #[test]
    fn reads_own_memory() {
        assert!(process_ram_mb() > 1.0);
    }
}

pub fn process_ram_mb() -> f32 {
    let mut counters = ProcessMemoryCounters {
        cb: std::mem::size_of::<ProcessMemoryCounters>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
    };

    let ok = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
    if ok == 0 {
        return 0.0;
    }
    (counters.working_set_size as f32) / (1024.0 * 1024.0)
}
