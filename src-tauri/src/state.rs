use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::config::model::{
    is_media_key, Action, AppConfig, ExceptionMode, ExceptionRule, Layer, MacroStep, Mapping,
    Profile, Settings, Theme, Trigger, WheelOptions,
};
use crate::config::store::ConfigStore;
use crate::engine::keys::{normalize_combo, vk_from_name};
use crate::platform::elevation;
use crate::platform::hotkey;
use crate::platform::service::{CaptureMode, EngineService};
use crate::tray;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub hooks_active: bool,
    pub capture_mode: String,
    pub elevated: bool,
    pub autostart: bool,
    pub config_path: String,
    pub config_dir: String,
    pub version: String,
    pub platform: String,
    pub load_warning: Option<String>,
    pub hook_warning: Option<String>,
    pub last_error: Option<String>,
    /// Lowercase executable name of the app in the foreground.
    pub foreground_app: Option<String>,
    /// Profile the engine is using, including per-app overrides.
    pub effective_profile_id: String,
    pub exception_mode: ExceptionMode,
    pub panic_active: bool,
    pub hotkey_warning: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub config: AppConfig,
    pub status: AppStatus,
}

pub struct AppState {
    config: Mutex<AppConfig>,
    write_lock: Mutex<()>,
    pub store: ConfigStore,
    pub engine: Arc<EngineService>,
    pub app: AppHandle,
    load_warning: Mutex<Option<String>>,
}

impl AppState {
    pub fn new(
        app: AppHandle,
        config: AppConfig,
        store: ConfigStore,
        engine: Arc<EngineService>,
        load_warning: Option<String>,
    ) -> Self {
        Self {
            config: Mutex::new(config),
            write_lock: Mutex::new(()),
            store,
            engine,
            app,
            load_warning: Mutex::new(load_warning),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let config = self.config.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let autostart = self
            .app
            .autolaunch()
            .is_enabled()
            .unwrap_or(config.settings.launch_at_startup);
        let config_path = self.store.path().to_string_lossy().to_string();
        let config_dir = self.store.dir().to_string_lossy().to_string();

        Snapshot {
            status: AppStatus {
                hooks_active: self.engine.hooks_active(),
                capture_mode: self.engine.capture_mode().name().to_string(),
                elevated: elevation::is_elevated(),
                autostart,
                config_path,
                config_dir,
                version: self.app.package_info().version.to_string(),
                platform: format!("Windows · {}", std::env::consts::ARCH),
                load_warning: self
                    .load_warning
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone(),
                hook_warning: self.engine.hook_warning(),
                last_error: self.engine.last_error(),
                foreground_app: self.engine.foreground_process(),
                effective_profile_id: self.engine.effective_profile_id(),
                exception_mode: self.engine.exception_mode(),
                panic_active: self.engine.panic_active(),
                hotkey_warning: hotkey::warning(),
            },
            config,
        }
    }

    pub fn cfg(&self) -> AppConfig {
        self.config.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn commit(&self, mut config: AppConfig, apply_theme: bool) -> Result<Snapshot, String> {
        let _write = self.write_lock.lock().unwrap_or_else(|e| e.into_inner());
        config.sanitize();
        self.store.save(&config)?;

        {
            let mut current = self.config.lock().unwrap_or_else(|e| e.into_inner());
            *current = config.clone();
        }

        self.engine.reconfigure(&config);
        hotkey::update(config.settings.panic_hotkey.clone());
        if apply_theme {
            apply_window_theme(&self.app, config.settings.theme);
        }
        if let Err(error) = tray::rebuild(&self.app, &config) {
            eprintln!("[MouseMacro] Could not refresh the tray menu: {error}");
        }
        let _ = self.app.emit("state:changed", ());
        Ok(self.snapshot())
    }

    pub fn set_enabled(&self, enabled: bool) -> Result<Snapshot, String> {
        if enabled {
            // Enabling is the explicit way out of the emergency stop.
            self.engine.clear_panic();
        }
        let mut config = self.cfg();
        config.settings.enabled = enabled;
        self.commit(config, false)
    }

    /// Re-reads `config.json` from disk. Intended for people who edit the file by
    /// hand: without this the running app keeps its in-memory copy.
    pub fn reload(&self) -> Result<Snapshot, String> {
        let loaded = self.store.load();
        {
            let mut warning = self.load_warning.lock().unwrap_or_else(|e| e.into_inner());
            *warning = loaded.warning;
        }
        self.commit(loaded.config, true)
    }

    pub fn set_capture_mode(&self, mode: CaptureMode) -> Result<Snapshot, String> {
        self.engine.set_capture_mode(mode)?;
        Ok(self.snapshot())
    }

    pub fn set_active_profile(&self, profile_id: &str) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        if !config.profiles.iter().any(|p| p.id == profile_id) {
            return Err("That profile no longer exists.".into());
        }
        config.active_profile_id = profile_id.to_string();
        self.commit(config, false)
    }

    pub fn update_settings(&self, mut settings: Settings) -> Result<Snapshot, String> {
        if !(100..=2_000).contains(&settings.long_press_ms) {
            return Err("Long-press timing must be between 100 and 2000 ms.".into());
        }
        if !(120..=1_000).contains(&settings.double_click_ms) {
            return Err("Double-click timing must be between 120 and 1000 ms.".into());
        }
        settings.sanitize();
        if !settings.panic_hotkey.is_empty() {
            hotkey::parse_combo(&settings.panic_hotkey)?;
        }

        let mut config = self.cfg();
        let was_enabled = config.settings.enabled;
        let want_autostart = settings.launch_at_startup;
        let registry = self.app.autolaunch();
        let current_registry = registry.is_enabled().unwrap_or(false);
        let changed_registry = current_registry != want_autostart;
        if changed_registry {
            let result = if want_autostart {
                registry.enable()
            } else {
                registry.disable()
            };
            if let Err(error) = result {
                return Err(format!(
                    "Windows startup could not be updated: {error}"
                ));
            }
        }

        if settings.enabled && !was_enabled {
            self.engine.clear_panic();
        }
        config.settings = settings;
        match self.commit(config, true) {
            Ok(snapshot) => Ok(snapshot),
            Err(error) => {
                if changed_registry {
                    let _ = if want_autostart {
                        registry.disable()
                    } else {
                        registry.enable()
                    };
                }
                Err(error)
            }
        }
    }

    pub fn set_launch_at_startup(&self, enabled: bool) -> Result<Snapshot, String> {
        let mut settings = self.cfg().settings;
        settings.launch_at_startup = enabled;
        self.update_settings(settings)
    }

    pub fn create_profile(&self, name: &str) -> Result<Snapshot, String> {
        let name = validate_profile_name(name)?;
        let mut config = self.cfg();
        ensure_unique_name(&config, &name, None)?;
        let mut profile = Profile::new(name);
        profile.description = String::new();
        config.profiles.push(profile);
        self.commit(config, false)
    }

    pub fn rename_profile(&self, profile_id: &str, name: &str) -> Result<Snapshot, String> {
        let name = validate_profile_name(name)?;
        let mut config = self.cfg();
        ensure_unique_name(&config, &name, Some(profile_id))?;
        let profile = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        profile.name = name;
        self.commit(config, false)
    }

    pub fn delete_profile(&self, profile_id: &str) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        if config.profiles.len() <= 1 {
            return Err("At least one profile is required.".into());
        }
        let index = config
            .profiles
            .iter()
            .position(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        config.profiles.remove(index);
        if config.active_profile_id == profile_id {
            config.active_profile_id = config.profiles[0].id.clone();
        }
        self.commit(config, false)
    }

    pub fn duplicate_profile(&self, profile_id: &str) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        let source = config
            .profiles
            .iter()
            .find(|p| p.id == profile_id)
            .cloned()
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        let mut name = format!("{} copy", source.name);
        let mut counter = 2;
        while config
            .profiles
            .iter()
            .any(|p| p.name.eq_ignore_ascii_case(&name))
        {
            name = format!("{} copy {}", source.name, counter);
            counter += 1;
        }
        let mut duplicate = Profile::new(name);
        duplicate.description = source.description.clone();
        duplicate.mappings = source
            .mappings
            .iter()
            .cloned()
            .map(|mut mapping| {
                mapping.id = uuid::Uuid::new_v4().to_string();
                mapping
            })
            .collect();
        config.profiles.push(duplicate);
        self.commit(config, false)
    }

    pub fn save_mapping(
        &self,
        profile_id: &str,
        mut mapping: Mapping,
    ) -> Result<Snapshot, String> {
        validate_mapping(&mut mapping)?;
        let mut config = self.cfg();
        let profile = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        if mapping.id.trim().is_empty() {
            mapping.id = uuid::Uuid::new_v4().to_string();
        }
        if let Some(existing) = profile
            .mappings
            .iter_mut()
            .find(|existing| existing.id == mapping.id)
        {
            *existing = mapping;
        } else {
            if profile.mappings.len() >= 500 {
                return Err("This profile already has 500 mappings.".into());
            }
            profile.mappings.push(mapping);
        }
        self.commit(config, false)
    }

    pub fn delete_mapping(
        &self,
        profile_id: &str,
        mapping_id: &str,
    ) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        let profile = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        let before = profile.mappings.len();
        profile.mappings.retain(|m| m.id != mapping_id);
        if profile.mappings.len() == before {
            return Err("That mapping no longer exists.".into());
        }
        self.commit(config, false)
    }

    pub fn set_mapping_enabled(
        &self,
        profile_id: &str,
        mapping_id: &str,
        enabled: bool,
    ) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        let profile = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        let mapping = profile
            .mappings
            .iter_mut()
            .find(|m| m.id == mapping_id)
            .ok_or_else(|| "That mapping no longer exists.".to_string())?;
        mapping.enabled = enabled;
        self.commit(config, false)
    }

    pub fn duplicate_mapping(
        &self,
        profile_id: &str,
        mapping_id: &str,
    ) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        let profile = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        let index = profile
            .mappings
            .iter()
            .position(|m| m.id == mapping_id)
            .ok_or_else(|| "That mapping no longer exists.".to_string())?;
        if profile.mappings.len() >= 500 {
            return Err("This profile already has 500 mappings.".into());
        }
        let mut copy = profile.mappings[index].clone();
        copy.id = uuid::Uuid::new_v4().to_string();
        if let Some(name) = copy.name.as_mut() {
            let base = name.clone();
            name.clear();
            name.push_str(&base.chars().take(52).collect::<String>());
            name.push_str(" copy");
        }
        profile.mappings.insert(index + 1, copy);
        self.commit(config, false)
    }

    pub fn set_mappings_enabled(
        &self,
        profile_id: &str,
        mapping_ids: Vec<String>,
        enabled: bool,
    ) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        let profile = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        let mut changed = 0;
        for mapping in &mut profile.mappings {
            if mapping_ids.iter().any(|id| id == &mapping.id) {
                mapping.enabled = enabled;
                changed += 1;
            }
        }
        if changed == 0 {
            return Err("Those mappings no longer exist.".into());
        }
        self.commit(config, false)
    }

    pub fn save_layer(&self, profile_id: &str, mut layer: Layer) -> Result<Snapshot, String> {
        validate_layer(&mut layer)?;
        let mut config = self.cfg();
        let profile = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        if layer.id.trim().is_empty() {
            layer.id = uuid::Uuid::new_v4().to_string();
        }
        if profile
            .layers
            .iter()
            .any(|existing| existing.id != layer.id && existing.activator == layer.activator)
        {
            return Err(format!(
                "{} is already used as a layer button.",
                layer.activator.label()
            ));
        }
        if let Some(existing) = profile.layers.iter_mut().find(|l| l.id == layer.id) {
            *existing = layer;
        } else {
            if profile.layers.len() >= 20 {
                return Err("This profile already has 20 layers.".into());
            }
            profile.layers.push(layer);
        }
        self.commit(config, false)
    }

    pub fn delete_layer(
        &self,
        profile_id: &str,
        layer_id: &str,
    ) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        let profile = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        let before = profile.layers.len();
        profile.layers.retain(|layer| layer.id != layer_id);
        if profile.layers.len() == before {
            return Err("That layer no longer exists.".into());
        }
        self.commit(config, false)
    }

    pub fn set_layer_enabled(
        &self,
        profile_id: &str,
        layer_id: &str,
        enabled: bool,
    ) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        let profile = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        let layer = profile
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id)
            .ok_or_else(|| "That layer no longer exists.".to_string())?;
        layer.enabled = enabled;
        self.commit(config, false)
    }

    pub fn save_layer_mapping(
        &self,
        profile_id: &str,
        layer_id: &str,
        mut mapping: Mapping,
    ) -> Result<Snapshot, String> {
        validate_mapping(&mut mapping)?;
        let mut config = self.cfg();
        let profile = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        let layer = profile
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id)
            .ok_or_else(|| "That layer no longer exists.".to_string())?;
        if mapping.id.trim().is_empty() {
            mapping.id = uuid::Uuid::new_v4().to_string();
        }
        if let Some(existing) = layer
            .mappings
            .iter_mut()
            .find(|existing| existing.id == mapping.id)
        {
            *existing = mapping;
        } else {
            if layer.mappings.len() >= 200 {
                return Err("This layer already has 200 mappings.".into());
            }
            layer.mappings.push(mapping);
        }
        self.commit(config, false)
    }

    pub fn delete_layer_mapping(
        &self,
        profile_id: &str,
        layer_id: &str,
        mapping_id: &str,
    ) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        let profile = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        let layer = profile
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id)
            .ok_or_else(|| "That layer no longer exists.".to_string())?;
        let before = layer.mappings.len();
        layer.mappings.retain(|mapping| mapping.id != mapping_id);
        if layer.mappings.len() == before {
            return Err("That mapping no longer exists.".into());
        }
        self.commit(config, false)
    }

    pub fn set_layer_mapping_enabled(
        &self,
        profile_id: &str,
        layer_id: &str,
        mapping_id: &str,
        enabled: bool,
    ) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        let profile = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        let layer = profile
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id)
            .ok_or_else(|| "That layer no longer exists.".to_string())?;
        let mapping = layer
            .mappings
            .iter_mut()
            .find(|mapping| mapping.id == mapping_id)
            .ok_or_else(|| "That mapping no longer exists.".to_string())?;
        mapping.enabled = enabled;
        self.commit(config, false)
    }

    pub fn save_exception(
        &self,
        mut rule: ExceptionRule,
    ) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        let Some(process) = crate::config::model::normalize_process_name(&rule.process) else {
            return Err("Enter an application executable, such as chrome.exe.".into());
        };
        rule.process = process;
        if rule.mode == ExceptionMode::Profile {
            let profile_id = rule
                .profile_id
                .clone()
                .ok_or_else(|| "Choose the profile to use for this app.".to_string())?;
            if !config.profiles.iter().any(|profile| profile.id == profile_id) {
                return Err("That profile no longer exists.".into());
            }
        } else {
            rule.profile_id = None;
        }
        if rule.id.trim().is_empty() {
            rule.id = uuid::Uuid::new_v4().to_string();
        }
        if config
            .exceptions
            .iter()
            .any(|existing| existing.id != rule.id && existing.process == rule.process)
        {
            return Err(format!(
                "A rule for “{}” already exists.",
                rule.process
            ));
        }
        if let Some(existing) = config
            .exceptions
            .iter_mut()
            .find(|existing| existing.id == rule.id)
        {
            *existing = rule;
        } else {
            if config.exceptions.len() >= 100 {
                return Err("There are already 100 application rules.".into());
            }
            config.exceptions.push(rule);
        }
        self.commit(config, false)
    }

    pub fn delete_exception(&self, rule_id: &str) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        let before = config.exceptions.len();
        config.exceptions.retain(|rule| rule.id != rule_id);
        if config.exceptions.len() == before {
            return Err("That application rule no longer exists.".into());
        }
        self.commit(config, false)
    }

    pub fn set_exception_enabled(
        &self,
        rule_id: &str,
        enabled: bool,
    ) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        let rule = config
            .exceptions
            .iter_mut()
            .find(|rule| rule.id == rule_id)
            .ok_or_else(|| "That application rule no longer exists.".to_string())?;
        rule.enabled = enabled;
        self.commit(config, false)
    }

    pub fn test_action(&self, mut action: Action) -> Result<(), String> {
        validate_action(&mut action)?;
        self.engine.test_action(&action);
        Ok(())
    }

    pub fn inspect_profile_import(
        &self,
        path: &std::path::Path,
    ) -> Result<Vec<String>, String> {
        let profile = self.store.read_profile(path)?;
        Ok(collect_launch_targets(&profile))
    }

    pub fn import_profile(&self, path: &std::path::Path, allow_launch: bool) -> Result<Snapshot, String> {
        let mut imported = self.store.read_profile(path)?;
        let launches = collect_launch_targets(&imported);
        if !launches.is_empty() && !allow_launch {
            return Err(format!(
                "This profile would run: {}. Confirm the import to allow launch actions.",
                launches.join(", ")
            ));
        }
        imported.id = uuid::Uuid::new_v4().to_string();
        for mapping in &mut imported.mappings {
            mapping.id = uuid::Uuid::new_v4().to_string();
            // Imported files may contain incomplete or unknown mappings. Keep
            // them visible but inert so the user can repair them.
            if validate_mapping(mapping).is_err() {
                mapping.enabled = false;
                mapping.action = Action::Disable;
            }
        }

        let mut config = self.cfg();
        let mut name = imported.name.trim().chars().take(60).collect::<String>();
        if name.is_empty() {
            name = "Imported profile".into();
        }
        let base = name.clone();
        let mut counter = 2;
        while config
            .profiles
            .iter()
            .any(|p| p.name.eq_ignore_ascii_case(&name))
        {
            name = format!("{base} {counter}");
            counter += 1;
        }
        imported.name = name;
        config.active_profile_id = imported.id.clone();
        config.profiles.push(imported);
        self.commit(config, false)
    }

    pub fn export_profile(
        &self,
        profile_id: &str,
        path: &std::path::Path,
    ) -> Result<String, String> {
        let config = self.cfg();
        let profile = config
            .profiles
            .iter()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        self.store.write_profile(profile, path)?;
        Ok(path.to_string_lossy().to_string())
    }

    pub fn export_config(&self, path: &std::path::Path) -> Result<String, String> {
        self.store.write_config(&self.cfg(), path)?;
        Ok(path.to_string_lossy().to_string())
    }

    pub fn import_config(&self, path: &std::path::Path) -> Result<Snapshot, String> {
        let mut config = self.store.read_config(path)?;
        // Never let an imported file change Windows startup integration behind
        // the user's back; the registry remains the source of truth.
        config.settings.launch_at_startup = self.cfg().settings.launch_at_startup;
        config.sanitize();
        self.engine.clear_panic();
        *self
            .load_warning
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        self.commit(config, true)
    }

    pub fn restore_backup(&self, name: &str) -> Result<Snapshot, String> {
        let mut config = self.store.read_backup(name)?;
        config.settings.launch_at_startup = self.cfg().settings.launch_at_startup;
        config.sanitize();
        self.engine.clear_panic();
        *self
            .load_warning
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        self.commit(config, true)
    }

    pub fn list_backups(&self) -> Vec<crate::config::store::BackupInfo> {
        self.store.list_backups()
    }

    pub fn create_backup(&self) -> Result<crate::config::store::BackupInfo, String> {
        self.store.create_backup(&self.cfg())
    }

    pub fn reset(&self) -> Result<Snapshot, String> {
        let registry = self.app.autolaunch();
        let _ = registry.disable();
        let config = AppConfig::default();
        *self
            .load_warning
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        self.engine.clear_errors();
        self.engine.clear_panic();
        self.engine.clear_activity();
        let snapshot = self.commit(config, true)?;
        self.engine.set_capture_mode(CaptureMode::None)?;
        Ok(snapshot)
    }

    pub fn clear_warnings(&self) -> Snapshot {
        *self
            .load_warning
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        self.engine.clear_errors();
        self.snapshot()
    }

    pub fn show_window(&self) {
        if let Some(window) = self.app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }

    pub fn show_window_if_needed(&self) {
        if !self.cfg().settings.start_minimized {
            self.show_window();
        }
    }
}

fn collect_launch_targets(profile: &Profile) -> Vec<String> {
    profile
        .mappings
        .iter()
        .chain(profile.layers.iter().flat_map(|layer| layer.mappings.iter()))
        .filter_map(|mapping| match &mapping.action {
            Action::Launch { target } => Some(target.clone()),
            _ => None,
        })
        .collect()
}

fn validate_profile_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Give the profile a name.".into());
    }
    if name.chars().count() > 60 {
        return Err("Profile names can be at most 60 characters.".into());
    }
    Ok(name.to_string())
}

fn ensure_unique_name(
    config: &AppConfig,
    name: &str,
    except_id: Option<&str>,
) -> Result<(), String> {
    if config.profiles.iter().any(|profile| {
        profile.name.eq_ignore_ascii_case(name) && Some(profile.id.as_str()) != except_id
    }) {
        return Err(format!("A profile named “{name}” already exists."));
    }
    Ok(())
}

pub fn apply_window_theme(app: &AppHandle, theme: Theme) {
    let tauri_theme = match theme {
        Theme::System => None,
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark => Some(tauri::Theme::Dark),
    };
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_theme(tauri_theme);
    }
}

/// Validates a mapping and normalizes its key tokens in place.
pub fn validate_mapping(mapping: &mut Mapping) -> Result<(), String> {
    mapping.name = mapping
        .name
        .as_ref()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty());
    if mapping.name.as_ref().is_some_and(|name| name.chars().count() > 60) {
        return Err("Mapping labels can be at most 60 characters.".into());
    }
    match &mapping.trigger {
        Trigger::Wheel { .. } => {
            if mapping.button.is_some() {
                return Err("A scroll wheel trigger does not use a mouse button.".into());
            }
            mapping
                .wheel
                .get_or_insert_with(WheelOptions::default)
                .sanitize();
        }
        _ => {
            if mapping.button.is_none() {
                return Err("Choose a mouse button for this mapping.".into());
            }
            mapping.wheel = None;
        }
    }
    if let Trigger::WithButton { button } = &mapping.trigger {
        if Some(*button) == mapping.button {
            return Err("A button combination needs a second, different button.".into());
        }
    }

    validate_action(&mut mapping.action)
}

/// Validates and normalizes an action without needing a full mapping. Used by
/// the editor's Test button.
pub fn validate_action(action: &mut Action) -> Result<(), String> {
    match action {
        Action::Shortcut { keys } | Action::HoldKeys { keys } => {
            *keys = normalize_combo(keys);
            if keys.is_empty() {
                return Err("Choose at least one key for this action.".into());
            }
            if keys.len() > 8 {
                return Err("A shortcut can contain at most 8 keys.".into());
            }
            if let Some(unknown) = keys.iter().find(|key| vk_from_name(key).is_none()) {
                return Err(format!("Unknown key “{unknown}”."));
            }
        }
        Action::Key { key } => {
            let normalized = key.trim().to_lowercase();
            if vk_from_name(&normalized).is_none() {
                return Err(format!("Unknown key “{key}”."));
            }
            *key = normalized;
        }
        Action::Scroll { amount, .. } => {
            if !(1..=50).contains(amount) {
                return Err("Scroll amount must be between 1 and 50.".into());
            }
        }
        Action::Macro {
            steps,
            repeat,
            interval_ms,
            toggle: _,
        } => {
            if steps.is_empty() {
                return Err("Add at least one step to the macro.".into());
            }
            if steps.len() > 40 {
                return Err("A macro can contain at most 40 steps.".into());
            }
            if !(1..=100).contains(repeat) {
                return Err("Macro repeat must be between 1 and 100.".into());
            }
            if *interval_ms > 10_000 {
                return Err("The macro repeat interval can be at most 10000 ms.".into());
            }
            let mut step_ms: u64 = 0;
            for step in steps.iter_mut() {
                step_ms = step_ms.saturating_add(validate_step(step)?);
            }
            let cycles = *repeat as u64;
            let total = step_ms
                .saturating_mul(cycles)
                .saturating_add(interval_ms.saturating_mul(cycles.saturating_sub(1)));
            if total > 120_000 {
                return Err(
                    "This macro would run for more than two minutes. Shorten the steps or the repeat count."
                        .into(),
                );
            }
        }
        Action::Launch { target } => {
            *target = target.trim().to_string();
            if target.is_empty() {
                return Err("Enter a program, file, folder or URL to launch.".into());
            }
            if target.chars().count() > 1_000 {
                return Err("The launch target is too long.".into());
            }
        }
        Action::TypeText { text } => {
            if text.chars().count() > 2_000 {
                return Err("A macro can type at most 2000 characters at once.".into());
            }
            if text.is_empty() {
                return Err("Enter the text to type.".into());
            }
        }
        Action::Media { key } => {
            let normalized = key.trim().to_lowercase();
            if !is_media_key(&normalized) {
                return Err(format!("Unknown media key “{key}”."));
            }
            *key = normalized;
        }
        Action::ToggleKeys { keys } => {
            *keys = normalize_combo(keys);
            if keys.is_empty() {
                return Err("Choose at least one key to toggle.".into());
            }
            if keys.len() > 4 {
                return Err("A toggle can contain at most 4 keys.".into());
            }
            if let Some(unknown) = keys.iter().find(|key| vk_from_name(key).is_none()) {
                return Err(format!("Unknown key “{unknown}”."));
            }
        }
        Action::Window { .. }
        | Action::DragLock { .. }
        | Action::MouseClick { .. }
        | Action::MouseHold { .. }
        | Action::Disable
        | Action::None => {}
    }
    Ok(())
}

/// Validates a macro step and returns the pause it contributes to the macro's
/// estimated duration.
fn validate_step(step: &mut MacroStep) -> Result<u64, String> {
    match step {
        MacroStep::TapKeys { keys, delay_ms }
        | MacroStep::HoldKeys { keys, delay_ms }
        | MacroStep::ReleaseKeys { keys, delay_ms } => {
            *keys = normalize_combo(keys);
            if keys.is_empty() {
                return Err("A macro key step needs at least one key.".into());
            }
            if keys.len() > 8 {
                return Err("A macro key step can contain at most 8 keys.".into());
            }
            if let Some(unknown) = keys.iter().find(|key| vk_from_name(key).is_none()) {
                return Err(format!("Unknown key “{unknown}”."));
            }
            *delay_ms = (*delay_ms).min(60_000);
        }
        MacroStep::Wait { delay_ms } => {
            *delay_ms = (*delay_ms).clamp(1, 60_000);
        }
        MacroStep::MouseClick { delay_ms, .. }
        | MacroStep::MouseHold { delay_ms, .. }
        | MacroStep::MouseRelease { delay_ms, .. } => {
            *delay_ms = (*delay_ms).min(60_000);
        }
        MacroStep::Scroll {
            amount, delay_ms, ..
        } => {
            if !(1..=50).contains(amount) {
                return Err("Macro scroll amount must be between 1 and 50.".into());
            }
            *delay_ms = (*delay_ms).min(60_000);
        }
        MacroStep::TypeText { text, delay_ms } => {
            if text.is_empty() {
                return Err("A text step needs some text.".into());
            }
            if text.chars().count() > 2_000 {
                return Err("A text step can type at most 2000 characters.".into());
            }
            *delay_ms = (*delay_ms).min(60_000);
        }
    }
    Ok(step.delay_ms())
}

fn validate_layer(layer: &mut Layer) -> Result<(), String> {
    let name = layer.name.trim();
    if name.is_empty() {
        return Err("Give the layer a name.".into());
    }
    if name.chars().count() > 60 {
        return Err("Layer names can be at most 60 characters.".into());
    }
    layer.name = name.to_string();

    let mut mapping_ids = Vec::new();
    for mapping in &mut layer.mappings {
        if mapping.id.trim().is_empty() || mapping_ids.contains(&mapping.id) {
            mapping.id = uuid::Uuid::new_v4().to_string();
        }
        mapping_ids.push(mapping.id.clone());
        validate_mapping(mapping)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::MacroStep;

    #[test]
    fn macro_validation_rejects_empty_and_overlong_sequences() {
        let mut empty = Action::Macro {
            steps: vec![],
            repeat: 1,
            interval_ms: 0,
            toggle: false,
        };
        assert!(validate_action(&mut empty).is_err());

        let mut too_long = Action::Macro {
            steps: vec![MacroStep::Wait { delay_ms: 60_000 }; 3],
            repeat: 10,
            interval_ms: 0,
            toggle: false,
        };
        assert!(validate_action(&mut too_long).is_err());

        let mut valid = Action::Macro {
            steps: vec![MacroStep::TapKeys {
                keys: vec!["ctrl".into(), "z".into()],
                delay_ms: 10,
            }],
            repeat: 2,
            interval_ms: 100,
            toggle: true,
        };
        assert!(validate_action(&mut valid).is_ok());
    }

    #[test]
    fn launch_media_and_toggle_validation() {
        let mut launch = Action::Launch {
            target: "   ".into(),
        };
        assert!(validate_action(&mut launch).is_err());

        let mut media = Action::Media {
            key: "bogus".into(),
        };
        assert!(validate_action(&mut media).is_err());

        let mut toggle = Action::ToggleKeys {
            keys: vec!["Ctrl".into(), "Shift".into()],
        };
        assert!(validate_action(&mut toggle).is_ok());
        assert_eq!(toggle, Action::ToggleKeys {
            keys: vec!["ctrl".into(), "shift".into()],
        });
    }
}
