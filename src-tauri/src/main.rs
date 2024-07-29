// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::app::App;
use crate::config_manager::AppState;

mod blazzy_runner;
mod meilisearch_runner;
mod app;
mod tasker;
mod config_manager;
mod ws_connector;
mod blazzy_client;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let mut app = App::init_conf(None).await;
    if let AppState::FirstRun =  app.get_state().await {
        app.conf_first_setup().await;
    }
    app.default_run().await;
}
