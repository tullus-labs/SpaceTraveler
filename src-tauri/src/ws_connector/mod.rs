use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use futures::SinkExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::mpsc::error::SendError;
use tokio_stream::StreamExt;
use tracing::{info, error};
use websocket::{ClientBuilder, Message, OwnedMessage};

pub struct WsConnector {
    senders: HashMap<String, Sender<OwnedMessage>>
}

impl WsConnector {
    pub fn init() -> Self {
        Self {
            senders: HashMap::new()
        }
    }

    pub async fn connect(&mut self, connection_key: &str, url: &str, client_sender: Option<Sender<OwnedMessage>>) {
        let client = ClientBuilder::new(url)
            .unwrap()
            .add_protocol("rust-websocket")
            .connect_insecure()
            .unwrap();

        let (mut rec, mut send) = client.split().unwrap();
        let (tx, mut rx) = channel(32);
        let tx_1 = tx.clone();
        let client_sender = client_sender.clone();

        tokio::task::spawn(async move {
            loop {
                let message = rx.recv().await.unwrap();

                match message {
                    OwnedMessage::Close(_) => {
                        let _ = send.send_message(&message);
                        return;
                    }
                    _ => (),
                }

                send.send_message(&message).unwrap_or_else( |e | {
                    error!("{}", e);
                    let _ = send.send_message(&Message::close());
                    return;
                })
            }
        });

        tokio::task::spawn(async move {
            for message in rec.incoming_messages() {
                let message = message.unwrap_or_else( | e | {
                    error!("{}", e);
                    return OwnedMessage::Close(None);
                });
                match message {
                    OwnedMessage::Text(data) => {
                        if let Some(sender) = client_sender.clone() {
                            if let Err(e) = sender.send(OwnedMessage::Text(data.clone())).await {
                                error!("{e}")
                            }
                        }
                        info!("{}", data);
                    }
                    OwnedMessage::Close(_) => {
                        let _ = tx_1.send(OwnedMessage::Close(None)).await;
                        return;
                    }
                    OwnedMessage::Ping(data) => {
                        if let Err(e) = tx_1.send(OwnedMessage::Pong(data)).await {
                            error!("{}", e);
                            return;
                        }
                    }
                    _ => {}
                }
            }
        });

        self.senders.insert(connection_key.to_string(), tx);
    }

    pub fn get_connection(&self, connection_key: &str) -> Option<Sender<OwnedMessage>> {
        if let Some(tx) = self.senders.get(connection_key) {
            return Some(tx.clone())
        }
        None
    }

    pub async fn send(&self, connection_key: &str, message: &str) {
        if let Some(tx) = self.get_connection(connection_key) {
            if let Err(e) =  tx.send(OwnedMessage::Text(message.to_string())).await {
                error!("{}", e)
            }
        }
    }
}
