use crate::{ws, Client, Clients, Message, Messages, WarpResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use warp::{http::StatusCode, reply::json, ws::Message as WebsocketMessage, Reply};

#[derive(Deserialize, Debug)]
pub struct RegisterRequest {
    username: String,
    user_id: usize,
}

#[derive(Serialize, Debug)]
pub struct RegisterResponse {
    url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MessageEvent {
    receiver_id: usize,
    sender_id: usize,
    user_id: Option<usize>,
    message: String,
}

pub async fn get_messages_handler(messages: Messages) -> WarpResult<impl Reply> {
    let mut all_messages = HashMap::new();
    let messages_array = messages.lock().await;
    messages_array.iter().for_each(|(key, value)| {
        all_messages.insert(key, value);
    });

    Ok(json(&all_messages))
}

pub async fn send_message_handler(
    body: MessageEvent,
    clients: Clients,
    messages: Messages,
) -> WarpResult<impl Reply> {
    clients
        .lock()
        .await
        .iter()
        .filter(|(_, client)| match body.user_id {
            Some(v) => client.user_id == v,
            None => true,
        })
        .for_each(|(_, client)| {
            if let Some(sender) = &client.sender {
                let _ = sender.send(Ok(WebsocketMessage::text(body.message.clone())));
                let _ = add_message(body.clone(), messages.clone());
            }
        });

    Ok(StatusCode::OK)
}

async fn add_message(body: MessageEvent, messages: Messages) {
    let id = messages.lock().await.len() + 1;
    let user_id = match body.user_id {
        Some(v) => v,
        None => 0,
    };

    messages.lock().await.insert(
        id.to_string(),
        Message {
            receiver_id: body.receiver_id,
            sender_id: body.sender_id,
            user_id: user_id,
            message: body.message
        },
    );
}

pub async fn register_handler(body: RegisterRequest, clients: Clients) -> WarpResult<impl Reply> {
    let username = body.username;
    let user_id = body.user_id;
    let uuid = Uuid::new_v4().simple().to_string();

    register_client(uuid.clone(), username, user_id, clients.clone()).await;
    Ok(json(&RegisterResponse {
        url: format!("ws://127.0.0.1:8000/ws/{}", uuid),
    }))
}

pub async fn register_client(id: String, username: String, user_id: usize, clients: Clients) {
    clients.lock().await.insert(
        id,
        Client {
            username,
            user_id,
            sender: None,
        },
    );
}

pub async fn ws_handler(ws: warp::ws::Ws, id: String, clients: Clients) -> WarpResult<impl Reply> {
    let client = clients.lock().await.get(&id).cloned();
    match client {
        Some(c) => Ok(ws.on_upgrade(move |socket| ws::client_connection(socket, id, clients, c))),
        None => Err(warp::reject::not_found()),
    }
}
