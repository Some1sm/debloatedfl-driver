use std::path::PathBuf;
use crate::gui::macro_view::MacroItem;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AppSettings {
    pub ui_scale: f32,
    pub font_size_delta: f32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            ui_scale: 1.15,
            font_size_delta: 0.0,
        }
    }
}

pub fn get_storage_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let p = PathBuf::from(appdata).join("fl-driver-rs");
        let _ = std::fs::create_dir_all(&p);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from(".")
}

pub fn load_settings() -> AppSettings {
    let path = get_storage_dir().join("settings.json");
    if let Ok(data) = std::fs::read_to_string(&path) {
        if let Ok(s) = serde_json::from_str::<AppSettings>(&data) {
            return s;
        }
    }
    AppSettings::default()
}

pub fn save_settings(settings: &AppSettings) {
    let path = get_storage_dir().join("settings.json");
    if let Ok(data) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(&path, data);
    }
}

pub fn load_saved_macros() -> Option<Vec<MacroItem>> {
    let path = get_storage_dir().join("macros.json");
    if let Ok(data) = std::fs::read_to_string(&path) {
        if let Ok(m) = serde_json::from_str::<Vec<MacroItem>>(&data) {
            return Some(m);
        }
    }
    None
}

pub fn save_macros(macros: &[MacroItem]) {
    let path = get_storage_dir().join("macros.json");
    if let Ok(data) = serde_json::to_string_pretty(macros) {
        let _ = std::fs::write(&path, data);
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct LightLayerStorage {
    pub layer_1: Vec<[u8; 3]>,
    pub layer_2: Vec<[u8; 3]>,
    pub layer_3: Vec<[u8; 3]>,
}

impl Default for LightLayerStorage {
    fn default() -> Self {
        Self {
            layer_1: vec![[0, 0, 0]; 87],
            layer_2: vec![[0, 0, 0]; 87],
            layer_3: vec![[0, 0, 0]; 87],
        }
    }
}

pub fn load_light_layers() -> LightLayerStorage {
    let path = get_storage_dir().join("light_layers.json");
    if let Ok(data) = std::fs::read_to_string(&path) {
        if let Ok(l) = serde_json::from_str::<LightLayerStorage>(&data) {
            return l;
        }
    }
    LightLayerStorage::default()
}

pub fn save_light_layers(layers: &LightLayerStorage) {
    let path = get_storage_dir().join("light_layers.json");
    if let Ok(data) = serde_json::to_string_pretty(layers) {
        let _ = std::fs::write(&path, data);
    }
}

pub fn export_macros_to_file(macros: &[MacroItem]) -> Option<String> {
    let file = rfd::FileDialog::new()
        .add_filter("JSON Files (*.json)", &["json"])
        .set_file_name("keyboard_macros.json")
        .save_file();
    if let Some(path) = file {
        match serde_json::to_string_pretty(macros) {
            Ok(json) => match std::fs::write(&path, json) {
                Ok(_) => Some(format!("Exported {} macros to {}", macros.len(), path.display())),
                Err(e) => Some(format!("Failed to write file: {}", e)),
            },
            Err(e) => Some(format!("Serialization error: {}", e)),
        }
    } else {
        None
    }
}

pub fn export_single_macro_to_file(macro_item: &MacroItem) -> Option<String> {
    let clean_name = macro_item.name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
    let file = rfd::FileDialog::new()
        .add_filter("JSON Files (*.json)", &["json"])
        .set_file_name(&format!("{}.json", clean_name))
        .save_file();
    if let Some(path) = file {
        match serde_json::to_string_pretty(macro_item) {
            Ok(json) => match std::fs::write(&path, json) {
                Ok(_) => Some(format!("Exported '{}' to {}", macro_item.name, path.display())),
                Err(e) => Some(format!("Failed to write file: {}", e)),
            },
            Err(e) => Some(format!("Serialization error: {}", e)),
        }
    } else {
        None
    }
}

pub fn import_macros_from_file() -> Result<(Vec<MacroItem>, String), String> {
    let file = rfd::FileDialog::new()
        .add_filter("JSON Files (*.json)", &["json"])
        .pick_file();
    if let Some(path) = file {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        if let Ok(list) = serde_json::from_str::<Vec<MacroItem>>(&content) {
            let count = list.len();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            return Ok((list, format!("Successfully imported {} macros from '{}'", count, name)));
        }
        if let Ok(single) = serde_json::from_str::<MacroItem>(&content) {
            let name = single.name.clone();
            return Ok((vec![single], format!("Successfully imported macro '{}'", name)));
        }
        Err("Invalid macro format. Expected a JSON file with a macro or array of macros.".to_string())
    } else {
        Err("Cancelled".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::macro_view::MacroAction;

    #[test]
    fn test_settings_serialization() {
        let s = AppSettings {
            ui_scale: 1.40,
            font_size_delta: 2.0,
        };
        let json = serde_json::to_string(&s).unwrap();
        let loaded: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.ui_scale, 1.40);
        assert_eq!(loaded.font_size_delta, 2.0);
    }

    #[test]
    fn test_macro_serialization() {
        let mut m = MacroItem::new("Combo Attack");
        m.actions = vec![
            MacroAction::KeyDown("Q".to_string()),
            MacroAction::Delay(40),
            MacroAction::KeyUp("Q".to_string()),
        ];
        let list = vec![m];
        let json = serde_json::to_string_pretty(&list).unwrap();
        let loaded: Vec<MacroItem> = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Combo Attack");
        assert_eq!(loaded[0].actions.len(), 3);
    }
}
