use std::{fs::File, thread};

use afire::{
    extensions::{RouteShorthands, ServeStatic},
    headers::ContentType,
    Content, Middleware, Server,
};
use serde_json::json;
use uuid::Uuid;

use crate::database::Database;

pub fn start(database: Database) {
    let mut server = Server::<Database>::new("0.0.0.0", 8081)
        .workers(4)
        .state(database);

    ServeStatic::new("web").attach(&mut server);

    server.get("/messages", |ctx| {
        let messages = ctx.app().lock().get_messages()?;
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

    thread::spawn(|| server.run().unwrap());
}
