use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::time::Duration;
use atomic_refcell::AtomicRefCell;
use futures::{SinkExt, StreamExt};
use tauri::{command, GlobalWindowEvent, Menu, WindowEvent};
use tokio::fs::{OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::channel;
use tokio::time::sleep;
use tracing::{error, info};
use websocket::{ClientBuilder, Message, OwnedMessage, WebSocketResult};
use starship_plugin_api::api::StarShipPluginAPI;
use crate::blazzy_client::BlazzyClient;
use crate::blazzy_runner::BlazzyRunner;
use crate::config_manager::{AppState, ConfigManager};
use crate::meilisearch_runner::runner::{MeilisearchHost, MeilisearchMasterKey, MeilisearchRunner};
use crate::tasker::{Tasker, TaskerError,};
use crate::ws_connector::WsConnector;

pub struct App {
    config: ConfigManager,
    ws_connector: WsConnector,
    plugins: Vec<Box<dyn StarShipPluginAPI>>,

}

impl App {
    pub async fn init_conf(conf_path: Option<PathBuf>) -> Self {
        let conf_path = conf_path.unwrap_or(PathBuf::from(std::env::current_exe().unwrap().parent().unwrap().join(".conf.toml")));
        {
            OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .open(conf_path.clone())
                .await.unwrap();
        }
        let mut config = ConfigManager::new(conf_path).await;
        config.setup().await;
        Self{
            config,
            plugins: vec![],
            ws_connector: WsConnector::init(),
        }
    }

    pub async fn get_state(&mut self) -> AppState {
        self.config.get_state().await
    }

    pub async fn conf_first_setup(&mut self) {
        self.config.setup().await;
    }

    pub async fn default_run(&mut self) {

        let tasker_app = tokio::task::spawn(async {
            let mut tasker = Tasker::init();
            if let Err(e) = tasker.safe_run().await {
                error!(name: "Tasker run error", "Error: {}", e);
            }
        });

        let blazzy_client = BlazzyClient::init();

        self.ws_connector.connect("tasker","ws://127.0.0.1:5000/", None).await;



        //self.ws_connector.send("tasker", "observe { \"main_app\": \"SpaceTraveler\", \"sub_apps\": [\"blazzy\"], \"safe\": \"WoSafe\" }").await;
        //self.ws_connector.send("tasker", "run_app { \"app\": \"F:/devProjects/explorer/src-tauri/assets/blazzy.exe\", \"args\": [\"-p\", \"C:/\"], \"window\": false }").await;



        let menu = Menu::new();

        tauri::Builder::default()
            .menu(menu)
            .invoke_handler(tauri::generate_handler![call])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");

    }

    pub async fn call() {
        info!("Calling")
    }

}

#[command]
pub async fn call() {
    App::call().await;
}