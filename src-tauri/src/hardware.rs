//! Lightweight hardware introspection for the onboarding model picker.
//!
//! Intentionally minimal — we only need enough signal to pick a model tier,
//! not a full system profile.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct HardwareInfo {
    pub platform: String, // "macos" | "windows" | "linux"
    pub arch: String,     // "aarch64" | "x86_64"
    pub ram_gb: u32,      // total system RAM, rounded down
    pub is_apple_silicon: bool,
    /// Coarse tier: "low" | "mid" | "high". High = Apple Silicon or 16GB+ + modern arch.
    pub tier: String,
}

pub fn detect() -> HardwareInfo {
    let platform = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();
    let ram_gb = detect_ram_gb();
    let is_apple_silicon = platform == "macos" && arch == "aarch64";

    let tier = if is_apple_silicon && ram_gb >= 16 {
        "high"
    } else if ram_gb >= 16 || arch == "aarch64" {
        "mid"
    } else {
        "low"
    }
    .to_string();

    HardwareInfo {
        platform,
        arch,
        ram_gb,
        is_apple_silicon,
        tier,
    }
}

#[cfg(target_os = "macos")]
fn detect_ram_gb() -> u32 {
    use std::process::Command;
    Command::new("sysctl")
        .arg("-n")
        .arg("hw.memsize")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|bytes| (bytes / 1_000_000_000) as u32)
        .unwrap_or(8)
}

#[cfg(target_os = "windows")]
fn detect_ram_gb() -> u32 {
    // Windows: read via GetPhysicallyInstalledSystemMemory via PowerShell fallback.
    // Keep it dependency-free: shell out to wmic or powershell.
    use std::process::Command;
    let out = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory",
        ])
        .output()
        .ok();
    out.and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|bytes| (bytes / 1_000_000_000) as u32)
        .unwrap_or(8)
}

#[cfg(target_os = "linux")]
fn detect_ram_gb() -> u32 {
    use std::fs;
    fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|content| {
            content.lines().find_map(|line| {
                line.strip_prefix("MemTotal:").and_then(|rest| {
                    rest.trim()
                        .strip_suffix("kB")
                        .and_then(|kb| kb.trim().parse::<u64>().ok())
                        .map(|kb| (kb / 1_000_000) as u32)
                })
            })
        })
        .unwrap_or(8)
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn detect_ram_gb() -> u32 {
    8
}
