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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        // Unique suffix prevents inter-test interference when tests run in parallel.
        path.push(format!("soma_settings_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn load_returns_defaults_when_file_does_not_exist() {
        let dir = temp_dir();
        let settings = load(&dir);
        assert_eq!(settings.active_model, "phi3:mini");
        assert_eq!(settings.embedding_model, "nomic-embed-text");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_returns_defaults_when_json_is_invalid() {
        let dir = temp_dir();
        std::fs::write(dir.join("settings.json"), "not valid json {{").unwrap();
        let settings = load(&dir);
        assert_eq!(settings.active_model, "phi3:mini");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = temp_dir();
        let original = AppSettings {
            active_model: "llama3:latest".to_string(),
            embedding_model: "mxbai-embed-large".to_string(),
        };
        save(&dir, &original).unwrap();
        let loaded = load(&dir);
        assert_eq!(loaded.active_model, "llama3:latest");
        assert_eq!(loaded.embedding_model, "mxbai-embed-large");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_produces_valid_json_file() {
        let dir = temp_dir();
        let settings = AppSettings::default();
        save(&dir, &settings).unwrap();
        let raw = std::fs::read_to_string(dir.join("settings.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed["active_model"].as_str(), Some("phi3:mini"));
        assert_eq!(parsed["embedding_model"].as_str(), Some("nomic-embed-text"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
