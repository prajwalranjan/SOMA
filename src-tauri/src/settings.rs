use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub active_model: String,
    pub embedding_model: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            active_model: "phi3:mini".to_string(),
            embedding_model: "nomic-embed-text".to_string(),
        }
    }
}

pub fn load(app_data_dir: &Path) -> AppSettings {
    let path = app_data_dir.join("settings.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(app_data_dir: &Path, settings: &AppSettings) -> anyhow::Result<()> {
    let path = app_data_dir.join("settings.json");
    std::fs::write(&path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}
