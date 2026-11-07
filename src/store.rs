use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const EASY_SETUP: &[u8] = include_bytes!("../builtin/recordings/ctd/easy/setup_camera.txt");
const EASY_RESTART: &[u8] = include_bytes!("../builtin/recordings/ctd/easy/restart.txt");
const EASY_CLEAR_SETUP: &[u8] = include_bytes!("../builtin/recordings/ctd/easy_clear/setup_camera.txt");

const BUILT_IN: [(&str, &[u8]); 7] = [
    ("templates/ctd/start_game.png", include_bytes!("../builtin/templates/ctd/start_game.png")),
    ("templates/ctd/results_lost.png", include_bytes!("../builtin/templates/ctd/results_lost.png")),
    ("templates/ctd/results_won.png", include_bytes!("../builtin/templates/ctd/results_won.png")),
    ("recordings/ctd/easy/setup_camera.txt", EASY_SETUP),
    ("recordings/ctd/easy/restart.txt", EASY_RESTART),
    ("recordings/ctd/easy_clear/setup_camera.txt", EASY_CLEAR_SETUP),
    ("recordings/ctd/easy_clear/restart.txt", EASY_RESTART),
];

pub const DEFAULT_LENIENCY: i32 = 50;

pub fn data_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("WMACROS_DATA") {
        return PathBuf::from(dir);
    }
    PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join("welffi macros")
}

pub fn recording_path(set: &str, name: &str) -> PathBuf {
    data_root().join("recordings").join(set).join(format!("{name}.txt"))
}

pub fn template_path(set: &str, name: &str) -> PathBuf {
    data_root().join("templates").join(set).join(format!("{name}.png"))
}

pub fn recordings_folder() -> PathBuf {
    data_root().join("recordings")
}

fn write_if_missing(path: &Path, bytes: &[u8]) {
    if path.exists() {
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, bytes);
}

pub fn seed() {
    let root = data_root();
    for (path, bytes) in BUILT_IN {
        write_if_missing(&root.join(path), bytes);
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub leniency: BTreeMap<String, i32>,
    pub debug: bool,
    pub recorder_size: usize,
    pub recorder_rate: usize,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings { leniency: BTreeMap::new(), debug: false, recorder_size: 0, recorder_rate: 1 }
    }
}

fn settings_path() -> PathBuf {
    data_root().join("settings.json")
}

impl Settings {
    pub fn load() -> Settings {
        std::fs::read_to_string(settings_path()).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(data_root());
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(settings_path(), text);
        }
    }

    pub fn leniency(&self, mode: &str) -> i32 {
        self.leniency.get(mode).copied().unwrap_or(DEFAULT_LENIENCY)
    }
}
