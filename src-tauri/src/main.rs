// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::ops::Deref;
use std::sync::{Arc, mpsc};
use atomic_refcell::AtomicRefCell;
use tauri::{GlobalWindowEvent, Menu, WindowEvent};
use tokio::join;
use crate::blazzy_runner::{BlazzyRunner};
use crate::meilisearch_runner::runner::{MeilisearchHost, MeilisearchMasterKey, MeilisearchRunner};

mod meilisearch_runner;
mod blazzy_runner;

#[tokio::main]
async fn main() {

    let (tx, mut rx) = mpsc::channel::<GlobalWindowEvent>();

    let blazzy = Arc::new(AtomicRefCell::new(
        BlazzyRunner::init().await
    ));

    let meilisearch = Arc::new(AtomicRefCell::new(
        MeilisearchRunner::new(
            MeilisearchHost::default(), MeilisearchMasterKey::gen().await
        ).await
    ));

    let meilisearch_runner = meilisearch.clone();

    let meilisearch_server = tokio::task::spawn( async move {
        if let Err(e) = meilisearch_runner.borrow_mut().safe_run().await {
            panic!("{}", e)
        };
    });

    let blazzy_runner = blazzy.clone();

    let blazzy_server = tokio::task::spawn( async move {
        if let Err(e) = blazzy_runner.borrow_mut().safe_run().await {
            panic!("{}", e)
        }
    });

    let meilisearch_stopper = meilisearch.clone();

    let stopper = tokio::task::spawn( async move {
        while let Ok(event) = rx.recv() {
            match event.event() {
                WindowEvent::CloseRequested { .. } => {
                    let window = event.window();
                    meilisearch_stopper.borrow_mut().stop().await;
                    window.close().unwrap();
                }
                _ => {}
            }
        }
    });

    let menu = Menu::new();

    tauri::Builder::default()
        .menu(menu)
        .on_window_event(move |event| {
            tx.send(event).unwrap();
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    join!(meilisearch_server, blazzy_server, stopper);
}
