use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use atomic_refcell::AtomicRefCell;
use tokio::sync::mpsc::{Sender, Receiver};
use tracing::{error, info};
use websocket::OwnedMessage;

pub struct BlazzyClient {
    sender: Option<Sender<OwnedMessage>>,
    recv: Option<Arc<AtomicRefCell<Receiver<OwnedMessage>>>>,
    exe_path: PathBuf
}

impl BlazzyClient {
    pub fn init() -> Self {
        let exe_path = std::env::current_exe().unwrap()
            .parent().unwrap()
            .join("services/blazzy").join("blazzy.exe");
        {
            if !exe_path.exists() {
                let blazzy_path = exe_path.parent().unwrap();
                if !blazzy_path.exists() {
                    let services_path = blazzy_path.parent().unwrap();
                    if !services_path.exists() {
                        std::fs::create_dir(services_path).unwrap();
                    }
                    std::fs::create_dir(blazzy_path).unwrap();
                }
                let mut file = File::create(&exe_path).unwrap();
                file.write_all(include_bytes!("../../assets/blazzy.exe")).unwrap();
            }
        }

        Self {
            sender: None,
            recv: None,
            exe_path
        }

    }

    pub fn connect_channel(&mut self, channel: (Sender<OwnedMessage>, Receiver<OwnedMessage>)) {
        self.sender = Some(channel.0);
        self.recv = Some(Arc::new(AtomicRefCell::new(channel.1)));
    }

    pub async fn send(&self, message: &str) {
        if let Some(sender) = self.sender.clone() {
            if let Err(e) = sender.send(OwnedMessage::Text(message.to_string())).await {
                error!("{}", e)
            }
        }
    }

    pub async fn listen(&mut self) {
        if let Some(mut rx) = self.recv.clone() {
            tokio::task::spawn(async move {
                if let Some(message) = rx.borrow_mut().recv().await {
                    match message {
                        OwnedMessage::Text(data) => {
                            info!("{data}")
                        }
                        _ => {}
                    }
                }
            });
        }
    }
}