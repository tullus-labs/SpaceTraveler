use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::time::Duration;
use atomic_refcell::AtomicRefCell;
use serde::{Deserialize, Serialize};
use tauri::{GlobalWindowEvent, Menu, WindowEvent};
use tauri::utils::config::PluginConfig;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::sleep;
use toml::macros::insert_toml;
use starship_plugin_api::api::StarShipPluginAPI;
use crate::blazzy_runner::BlazzyRunner;
use crate::meilisearch_runner::runner::{MeilisearchHost, MeilisearchMasterKey, MeilisearchRunner};
use crate::tasker::{Tasker, TaskerError, TaskerResult};


#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    state: AppState,
    plugins_conf: Vec<PluginConfig>
}

impl AppConfig {
    pub async fn conf_first_setup(conf_file: &mut File) {
        let app_conf = AppConfig {
            state: AppState::FirstRun,
            plugins_conf: vec![]
        };
        conf_file.write_all(toml::to_string(&app_conf).unwrap().as_bytes()).await.unwrap();
    }

    pub async fn get_state(conf_file: &mut File) -> AppState {
        let mut con = "".to_string();
        conf_file.read_to_string(&mut con).await.unwrap();
        return if let Ok(app_conf) = toml::de::from_str::<AppConfig>(&con) {
            app_conf.state
        } else {
            AppState::FirstRun
        }

    }

}

#[derive(Serialize, Deserialize)]
pub enum AppState {
    FirstRun,
    Stable
}

pub struct App {
    config: File,
    tasker: Tasker,
    plugins: Vec<Box<dyn StarShipPluginAPI>>
}

impl App {
    pub async fn init_conf(conf_path: Option<PathBuf>) -> Self {
        let conf_path = conf_path.unwrap_or(PathBuf::from(std::env::current_exe().unwrap().parent().unwrap().join(".conf.toml")));
        Self{
            config: OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .open(conf_path)
                .await.unwrap(),
            tasker: Tasker::init(),
            plugins: vec![]
        }
    }

    pub async fn get_state(&mut self) -> AppState {
        AppConfig::get_state(&mut self.config).await
    }

    pub async fn conf_first_setup(&mut self) {
        AppConfig::conf_first_setup(&mut self.config).await;
    }

    pub async fn default_run(&mut self) {
        let (tx, rx) = mpsc::channel::<GlobalWindowEvent>();

        self.tasker.add_arc("blazzy", Arc::new(AtomicRefCell::new(BlazzyRunner::init().await))).await;
        self.tasker.add_arc(
            "meilisearch",
            Arc::new(AtomicRefCell::new(
                MeilisearchRunner::new(
                    MeilisearchHost::default(),
                    MeilisearchMasterKey::gen().await,
                ).await,
            ))
        ).await;

        //meilisearch runner task
        let meilisearch_runner = self.tasker.get_arc("meilisearch");
        self.tasker.add_task( tokio::task::spawn(async move {
            if let Some(runner) = meilisearch_runner.borrow_mut().downcast_mut::<MeilisearchRunner>() {
                if let Err(e) = runner.safe_run().await {
                    return TaskerResult::Err(TaskerError::ERROR(e.to_string()))
                };
                runner.run_client().await;
            }
            TaskerResult::Ok(())
        })).await;

        //blazzy runner task
        let blazzy_runner = self.tasker.get_arc("blazzy");
        self.tasker.add_task(tokio::task::spawn(async move {
            if let Some(runner) = blazzy_runner.borrow_mut().downcast_mut::<BlazzyRunner>() {
                if let Err(e) = runner.safe_run().await {
                    return TaskerResult::Err(TaskerError::ERROR(e.to_string()))
                }
            }
            TaskerResult::Ok(())
        })).await;

        //App stopper task
        let meilisearch_stopper = self.tasker.get_arc("meilisearch");
        self.tasker.add_task(tokio::task::spawn(async move {
            while let Ok(event) = rx.recv() {
                match event.event() {
                    WindowEvent::CloseRequested { .. } => {
                        let window = event.window();
                        if let Some(stopper) = meilisearch_stopper.borrow_mut().downcast_mut::<MeilisearchRunner>() {
                            stopper.stop().await;
                        }
                        sleep(Duration::from_secs(2)).await;
                        window.close().unwrap();
                    }
                    _ => {}
                }
            }
            TaskerResult::Ok(())
        })).await;

        let menu = Menu::new();

        tauri::Builder::default()
            .menu(menu)
            .on_window_event(move |event| {
                tx.send(event).unwrap();
            })
            .run(tauri::generate_context!())
            .expect("error while running tauri application");

        self.tasker.run_tasks().await;

    }

}