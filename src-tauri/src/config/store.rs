use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use super::model::{AppConfig, Profile};

const MAX_CONFIG_BYTES: u64 = 4 * 1024 * 1024;
const MAX_IMPORT_BYTES: u64 = 2 * 1024 * 1024;

pub struct Loaded {
    pub config: AppConfig,
    pub warning: Option<String>,
}

pub struct ConfigStore {
    dir: PathBuf,
}

impl ConfigStore {
    pub fn new(dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&dir)
            .map_err(|error| format!("Could not create the config folder: {error}"))?;
        Ok(Self { dir })
    }

    pub fn path(&self) -> PathBuf {
        self.dir.join("config.json")
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn load(&self) -> Loaded {
        let path = self.path();
        if !path.exists() {
            return Loaded {
                config: AppConfig::default(),
                warning: None,
            };
        }

        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                return Loaded {
                    config: AppConfig::default(),
                    warning: Some(format!("Could not read the configuration: {error}")),
                }
            }
        };
        if metadata.len() > MAX_CONFIG_BYTES {
            self.back_up_corrupt(&path);
            return Loaded {
                config: AppConfig::default(),
                warning: Some("The configuration file was too large and was reset.".into()),
            };
        }

        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) => {
                return Loaded {
                    config: AppConfig::default(),
                    warning: Some(format!("Could not read the configuration: {error}")),
                }
            }
        };

        match serde_json::from_str::<AppConfig>(&raw) {
            Ok(mut config) => {
                config.settings.launch_at_startup = false; // reconciled with the registry at startup
                let notes = config.sanitize();
                let warning = if notes.is_empty() {
                    None
                } else {
                    Some(notes.join(" "))
                };
                Loaded { config, warning }
            }
            Err(error) => {
                self.back_up_corrupt(&path);
                Loaded {
                    config: AppConfig::default(),
                    warning: Some(format!(
                        "The configuration was invalid ({error}). A backup was kept and defaults were restored."
                    )),
                }
            }
        }
    }

    pub fn save(&self, config: &AppConfig) -> Result<(), String> {
        let path = self.path();
        let temp = self.dir.join("config.json.tmp");
        let text = serde_json::to_string_pretty(config)
            .map_err(|error| format!("Could not serialize the configuration: {error}"))?;

        if path.exists() {
            let _ = fs::copy(&path, self.dir.join("config.backup.json"));
        }

        {
            let mut file = fs::File::create(&temp)
                .map_err(|error| format!("Could not create the configuration file: {error}"))?;
            file.write_all(text.as_bytes())
                .map_err(|error| format!("Could not write the configuration: {error}"))?;
            file.sync_all()
                .map_err(|error| format!("Could not flush the configuration: {error}"))?;
        }

        fs::rename(&temp, &path)
            .map_err(|error| format!("Could not replace the configuration file: {error}"))?;
        Ok(())
    }

    fn back_up_corrupt(&self, path: &Path) {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let backup = self.dir.join(format!("config.corrupt-{stamp}.json"));
        let _ = fs::copy(path, backup);
    }

    pub fn read_profile(&self, path: &Path) -> Result<Profile, String> {
        let metadata = fs::metadata(path)
            .map_err(|error| format!("Could not open the selected file: {error}"))?;
        if metadata.len() > MAX_IMPORT_BYTES {
            return Err("The selected profile is larger than 2 MB.".into());
        }
        let raw = fs::read_to_string(path)
            .map_err(|error| format!("Could not read the selected file: {error}"))?;
        let value: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|error| format!("That file is not valid JSON: {error}"))?;

        // Accept both a bare profile and `{ "profile": { ... } }`.
        let profile_value = value
            .get("profile")
            .cloned()
            .unwrap_or_else(|| value.clone());
        let mut profile: Profile = serde_json::from_value(profile_value)
            .map_err(|error| format!("That file is not a MouseMacro profile: {error}"))?;
        if profile.mappings.len() > 500 {
            return Err("The profile contains more than 500 mappings.".into());
        }
        if profile.name.trim().is_empty() {
            profile.name = "Imported profile".into();
        }
        Ok(profile)
    }

    pub fn write_profile(&self, profile: &Profile, path: &Path) -> Result<(), String> {
        #[derive(Serialize)]
        struct Export<'a> {
            #[serde(flatten)]
            profile: &'a Profile,
            r#format: &'static str,
            version: u32,
        }
        let payload = Export {
            profile,
            r#format: "mousemacro.profile",
            version: super::model::SCHEMA_VERSION,
        };
        let text = serde_json::to_string_pretty(&payload)
            .map_err(|error| format!("Could not serialize the profile: {error}"))?;
        fs::write(path, text).map_err(|error| format!("Could not save the profile: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::MouseButton;

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("mousemacro-{label}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn saves_and_loads_round_trip() {
        let dir = temp_dir("roundtrip");
        let store = ConfigStore::new(dir.clone()).unwrap();
        let mut config = AppConfig::default();
        config.active_profile_mut().unwrap().mappings[0].button = MouseButton::X1;
        store.save(&config).unwrap();

        let loaded = store.load();
        assert!(loaded.warning.is_none());
        assert_eq!(
            loaded.config.active_profile().unwrap().mappings[0].button,
            MouseButton::X1
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn corrupt_config_is_backed_up() {
        let dir = temp_dir("corrupt");
        let store = ConfigStore::new(dir.clone()).unwrap();
        fs::write(store.path(), "{not json").unwrap();
        let loaded = store.load();
        assert!(loaded.warning.is_some());
        assert_eq!(loaded.config.profiles.len(), 4);
        assert!(fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().starts_with("config.corrupt-")));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn profile_export_can_be_imported() {
        let dir = temp_dir("profile");
        let store = ConfigStore::new(dir.clone()).unwrap();
        let path = dir.join("profile.json");
        let profile = AppConfig::default().profiles[0].clone();
        store.write_profile(&profile, &path).unwrap();
        let imported = store.read_profile(&path).unwrap();
        assert_eq!(imported.name, profile.name);
        assert_eq!(imported.mappings.len(), profile.mappings.len());

        let _ = fs::remove_dir_all(dir);
    }
}
