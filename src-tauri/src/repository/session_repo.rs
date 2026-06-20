use crate::models::{ChatMessage, ChatSession};
use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use uuid::Uuid;

pub trait SessionRepository {
    fn create_session(&self, name: &str) -> Result<ChatSession>;
    fn get_all_sessions(&self) -> Result<Vec<ChatSession>>;
    fn rename_session(&self, id: &str, name: &str) -> Result<()>;
    fn delete_session(&self, id: &str) -> Result<()>;
    fn touch_session(&self, id: &str) -> Result<()>;
    fn save_message(&self, msg: &ChatMessage) -> Result<()>;
    fn get_messages(&self, session_id: &str) -> Result<Vec<ChatMessage>>;
    fn ensure_default_session(&self) -> Result<ChatSession>;
}

pub struct SqliteSessionRepository<'a> {
    pub conn: &'a Connection,
}

impl<'a> SessionRepository for SqliteSessionRepository<'a> {
    fn create_session(&self, name: &str) -> Result<ChatSession> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO chat_sessions (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, name, now, now],
        )?;
        Ok(ChatSession {
            id,
            name: name.to_string(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    fn get_all_sessions(&self) -> Result<Vec<ChatSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, created_at, updated_at FROM chat_sessions ORDER BY updated_at DESC",
        )?;
        let sessions = stmt
            .query_map([], |row| {
                Ok(ChatSession {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(sessions)
    }

    fn rename_session(&self, id: &str, name: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE chat_sessions SET name = ?1 WHERE id = ?2",
            rusqlite::params![name, id],
        )?;
        Ok(())
    }

    fn delete_session(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM chat_sessions WHERE id = ?1",
            rusqlite::params![id],
        )?;
        Ok(())
    }

    fn touch_session(&self, id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE chat_sessions SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )?;
        Ok(())
    }

    fn save_message(&self, msg: &ChatMessage) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO chat_history (id, session_id, role, content, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, msg.session_id, msg.role, msg.content, msg.timestamp],
        )?;
        self.touch_session(&msg.session_id)?;
        Ok(())
    }

    fn get_messages(&self, session_id: &str) -> Result<Vec<ChatMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, role, content, timestamp FROM chat_history WHERE session_id = ?1 ORDER BY timestamp ASC",
        )?;
        let messages = stmt
            .query_map(rusqlite::params![session_id], |row| {
                Ok(ChatMessage {
                    session_id: row.get(0)?,
                    role: row.get(1)?,
                    content: row.get(2)?,
                    timestamp: row.get(3)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(messages)
    }

    fn ensure_default_session(&self) -> Result<ChatSession> {
        let sessions = self.get_all_sessions()?;
        if let Some(first) = sessions.into_iter().next() {
            return Ok(first);
        }
        self.create_session("New chat")
    }
}
