use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use warp::{ws::Message as WebsocketMessage, Filter, Rejection};
use serde::{Serialize, ser::{SerializeStruct, Serializer}};

mod handler;
mod ws;

type WarpResult<T> = std::result::Result<T, Rejection>;
type Clients = Arc<Mutex<HashMap<String, Client>>>;
type Messages = Arc<Mutex<HashMap<String, Message>>>;

#[derive(Debug, Clone)]
pub struct Client {
    pub username: String,
    pub user_id: usize,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<WebsocketMessage, warp::Error>>>,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub receiver_id: usize,
    pub sender_id: usize,
    pub user_id: usize,
    pub message: String,
}

impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer
    {
        let mut state = serializer.serialize_struct("Message", 3)?;
        state.serialize_field("sender_id", &self.sender_id)?;
        state.serialize_field("receiver_id", &self.receiver_id)?;
        state.serialize_field("message", &self.message)?;
        state.end()
    }
}

#[tokio::main]
async fn main() {
    let clients: Clients = Arc::new(Mutex::new(HashMap::new()));
    let messages: Messages = Arc::new(Mutex::new(HashMap::new()));

    let register_route = warp::path("register")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_clients(clients.clone()))
        .and_then(handler::register_handler);

    let messages_route = warp::path("messages")
        .and(warp::get())
        .and(with_messages(messages.clone()))
        .and_then(handler::get_messages_handler);

    let send_message_route = warp::path("send")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_clients(clients.clone()))
        .and(with_messages(messages.clone()))
        .and_then(handler::send_message_handler);

    let websocket_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(with_clients(clients.clone()))
        .and_then(handler::ws_handler);

    let routes = register_route
        .or(messages_route)
        .or(send_message_route)
        .or(websocket_route)
        .with(warp::cors().allow_any_origin());

    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}

fn with_clients(clients: Clients) -> impl Filter<Extract = (Clients,), Error = Infallible> + Clone {
    warp::any().map(move || clients.clone())
}

fn with_messages(messages: Messages) -> impl Filter<Extract = (Messages,), Error = Infallible> + Clone {
    warp::any().map(move || messages.clone())
}
