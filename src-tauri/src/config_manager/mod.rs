use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use starship_plugin_api::plugin_config::PluginConfig;
pub struct ConfigManager {
    app_conf: AppConfig,
    conf_file: File
}

impl ConfigManager {
    pub async fn new(conf_file: PathBuf) -> Self {
        let mut conf_file = File::open(conf_file).await.unwrap();
        Self {
            app_conf: AppConfig::init(),
            conf_file
        }
    }

    pub async fn get_state(&mut self) -> AppState {
        self.app_conf.state.clone()
    }

    pub async fn set_state(&mut self, state: AppState) {
        self.app_conf.set_state(state).await;
        let con = toml::to_string(&self.app_conf).unwrap();
        self.conf_file.write_all(con.as_bytes()).await.unwrap();
    }

    pub async fn setup(&mut self) {
        self.app_conf.conf_first_setup(&mut self.conf_file).await;
    }
}

#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    state: AppState,
    plugins_conf: Vec<PluginConfig>
}

impl AppConfig {
    pub fn init() -> AppConfig {
        Self {
            state: AppState::None,
            plugins_conf: vec![]
        }
    }
    pub async fn conf_first_setup(&self, conf_file: &mut File) {
        let app_conf = AppConfig {
            state: AppState::FirstRun,
            plugins_conf: vec![]
        };
        conf_file.write_all(toml::to_string(&app_conf).unwrap().as_bytes()).await.unwrap();
    }

    pub async fn get_state(&self, conf_file: &mut File) -> AppState {
        let mut con = "".to_string();
        conf_file.read_to_string(&mut con).await.unwrap();
        return if let Ok(app_conf) = toml::de::from_str::<AppConfig>(&con) {
            app_conf.state
        } else {
            AppState::FirstRun
        }

    }

    pub async fn set_state(&mut self, state: AppState) {
        self.state = state;
    }

}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum AppState {
    FirstRun,
    Stable,
    None,
}