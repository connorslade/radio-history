use std::{fs::File, sync::Arc, thread};

use afire::{
    extensions::{RouteShorthands, ServeStatic},
    headers::ContentType,
    prelude::WebSocketExt,
    trace::{self, Level},
    Content, Middleware, Server,
};
use clone_macro::clone;
use flume::Sender;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::database::{Database, Message};

pub struct App {
    database: Database,
    clients: Arc<Mutex<Vec<Sender<UiMessage>>>>,
}

#[derive(Clone, Serialize)]
#[serde(tag = "type")]
pub enum UiMessage {
    Receiving,
    Processing,
    Complete(Message),
}

pub fn start(database: Database) -> Sender<UiMessage> {
    trace::set_log_level(Level::Trace);

    let (tx, rx) = flume::unbounded::<UiMessage>();
    let clients = Arc::new(Mutex::new(Vec::<Sender<_>>::new()));

    thread::spawn(clone!([clients], move || {
        for message in rx.iter() {
            for client in clients.lock().iter() {
                client.send(message.clone()).unwrap();
            }
        }
    }));

    let mut server = Server::<App>::new("0.0.0.0", 8081)
        .workers(16)
        .state(App { database, clients });

    ServeStatic::new("web").attach(&mut server);

    server.get("/messages", |ctx| {
        let messages = ctx.app().database.lock().get_messages()?;
        ctx.text(json!(messages)).content(Content::JSON).send()?;
        Ok(())
    });

    server.get("/audio/{uuid}", |ctx| {
        let uuid = Uuid::parse_str(ctx.param("uuid"))?;
        let path = format!("data/audio/{uuid}.wav");
        ctx.stream(File::open(path)?)
            .header(ContentType::new("audio/wav"))
            .send()?;
        Ok(())
    });

    server.get("/events", |ctx| {
        let socket = ctx.ws()?;

        let (tx, rx) = flume::unbounded();
        ctx.app().clients.lock().push(tx);

        for message in rx.iter() {
            socket.send(json!(message));
        }

        Ok(())
    });

    thread::spawn(|| server.run().unwrap());

    tx
}
