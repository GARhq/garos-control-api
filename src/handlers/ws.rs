//! WebSocket handler at `/api/ws`.

use crate::auth::jwt::JwtService;
use crate::error::AppError;
use crate::realtime::events::Channel;
use crate::realtime::hub::RealtimeHub;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
    pub channels: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WsClientMessage {
    Subscribe { channels: Vec<String> },
    Unsubscribe { channels: Vec<String> },
    Ping,
    Pong,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsServerMessage {
    Hello { client_id: String, channels: Vec<String> },
    Error { message: String },
    Heartbeat,
}

/// `GET /api/ws` — WebSocket upgrade.
pub async fn ws_handler(
    State((jwt, hub)): State<(Arc<JwtService>, RealtimeHub)>,
    Query(q): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    let token = q.token.ok_or(AppError::Unauthorized)?;
    let claims = jwt.verify(&token, "access")?;
    let channels = q
        .channels
        .as_deref()
        .map(|s| {
            s.split(',')
                .filter_map(Channel::from_str)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![Channel::All]);
    let _ = claims;
    let (id, rx) = hub.subscribe(channels.clone());
    Ok(ws.on_upgrade(move |socket| client_loop(socket, id, rx, hub, channels)))
}

async fn client_loop(
    socket: WebSocket,
    id: uuid::Uuid,
    mut rx: tokio::sync::broadcast::Receiver<Arc<crate::realtime::events::Event>>,
    hub: RealtimeHub,
    initial_channels: Vec<Channel>,
) {
    let (mut sender, mut receiver) = socket.split();
    let hello = WsServerMessage::Hello {
        client_id: id.to_string(),
        channels: initial_channels
            .iter()
            .map(|c| serde_json::to_value(c).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default())
            .collect(),
    };
    if let Ok(text) = serde_json::to_string(&hello) {
        if sender.send(Message::Text(text)).await.is_err() {
            return;
        }
    }
    let mut heartbeat = tokio::time::interval(Duration::from_secs(30));
    heartbeat.tick().await; // skip immediate

    loop {
        tokio::select! {
            evt = rx.recv() => {
                match evt {
                    Ok(ev) => {
                        let json = match serde_json::to_string(&*ev) {
                            Ok(s) => s,
                            Err(_) => continue,
                        };
                        if sender.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            _ = heartbeat.tick() => {
                if sender.send(Message::Ping(vec![1,2,3,4])).await.is_err() {
                    break;
                }
            }
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(t))) => {
                        if let Ok(client) = serde_json::from_str::<WsClientMessage>(&t) {
                            match client {
                                WsClientMessage::Subscribe { .. } | WsClientMessage::Unsubscribe { .. } => {
                                    // Channels are filtered client-side after send; re-subscribe ignored.
                                }
                                WsClientMessage::Ping => {
                                    let _ = sender.send(Message::Pong(vec![1,2,3,4])).await;
                                }
                                WsClientMessage::Pong => {}
                            }
                        }
                    }
                    Some(Ok(Message::Ping(p))) => {
                        let _ = sender.send(Message::Pong(p)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }
    hub.unsubscribe(id);
}
