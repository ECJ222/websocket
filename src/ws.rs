use crate::{Client, Clients};
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;
use warp::{reply::json};
use warp::ws::{Message as WebSocketMessage, WebSocket};

pub async fn client_connection(ws: WebSocket, id: String, clients: Clients, mut client: Client) {
    let (client_websocket_sender, mut client_websocket_receiver) = ws.split();
    let (client_sender, client_receiver) = mpsc::unbounded_channel();

    tokio::task::spawn(client_receiver.forward(client_websocket_sender).map(|result| {
        if let Err(e) = result {
            eprintln!("error sending websocket message: {}", e)
        }
    }));

    client.sender = Some(client_sender);
    clients.lock().await.insert(id.clone(), client);

    json(&format!("{} connected", id));

    while let Some(result) = client_websocket_receiver.next().await { 
        let msg = match result {
            Ok(msg) => msg,
            Err(e) =>  {
                eprintln!("error receiving websocket message for id: {}): {}", id.clone(), e);
                break;
            }
        };

        client_message(&id, msg);
    }

    json(&format!("{} disconnected", id));

    clients.lock().await.remove(&id);
}

fn client_message(id: &str, msg: WebSocketMessage) {
    println!("received message from {}: {:?}", id, msg);

    let message = match msg.to_str() {
        Ok(v) => v,
        Err(_) => return,
    };

    if message == "ping" || message == "ping\n" {
        return;
    }
}
