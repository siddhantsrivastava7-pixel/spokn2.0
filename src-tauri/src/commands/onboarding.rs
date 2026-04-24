//! Onboarding-specific Tauri commands: hardware detection + model
//! recommendation. The heavy lifting lives in [`crate::hardware`] and
//! [`crate::model_recommend`] — these thin wrappers just expose them to the
//! frontend via tauri-specta.

use crate::hardware::{self, HardwareInfo};
use crate::model_recommend;

#[tauri::command]
#[specta::specta]
pub fn detect_hardware() -> HardwareInfo {
    hardware::detect()
}

#[tauri::command]
#[specta::specta]
pub fn recommend_model_for_languages(languages: Vec<String>) -> String {
    let hw = hardware::detect();
    model_recommend::recommend(&languages, &hw)
}
