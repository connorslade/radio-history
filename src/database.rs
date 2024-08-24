use std::{fs, sync::Arc};

use anyhow::Result;
use chrono::NaiveDateTime;
use parking_lot::{Mutex, MutexGuard};
use rusqlite::{params, Connection};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone)]
pub struct Database {
    connection: Arc<Mutex<Connection>>,
}

pub struct LockedDatabase<'a> {
    connection: MutexGuard<'a, Connection>,
}

#[derive(Serialize)]
pub struct Message {
    pub date: NaiveDateTime,
    pub audio: Uuid,
    pub text: String,
}

impl Database {
    pub fn new() -> Result<Self> {
        let _ = fs::create_dir_all("data/audio");

        let connection = Connection::open("data/data.db")?;
        connection.execute(include_str!("sql/init_messages.sql"), params![])?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub fn lock(&self) -> LockedDatabase {
        LockedDatabase {
            connection: self.connection.lock(),
        }
    }
}

impl<'a> LockedDatabase<'a> {
    pub fn insert_message(&self, text: Option<&str>, audio: Uuid) -> Result<()> {
        self.connection
            .execute(include_str!("sql/insert_message.sql"), params![text, audio])?;
        Ok(())
    }

    pub fn get_messages(&self) -> Result<Vec<Message>> {
        let mut statement = self
            .connection
            .prepare(include_str!("sql/get_messages.sql"))?;
        let messages = statement
            .query_map(params![], |row| {
                Ok(Message {
                    date: row.get(0)?,
                    audio: row.get(1)?,
                    text: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(messages)
    }
}
