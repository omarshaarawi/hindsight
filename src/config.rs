use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub default_mode: Option<String>,
    pub default_limit: Option<u32>,
    pub height: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        let mut config = Config::default();
        
        if let Some(proj_dirs) = ProjectDirs::from("com", "shaarawi", "hindsight") {
            let config_path = proj_dirs.config_dir().join("config.toml");
            if let Ok(contents) = fs::read_to_string(config_path) {
                if let Ok(parsed) = toml::from_str::<Config>(&contents) {
                    config = parsed;
                }
            }
        }
        
        config
    }
}