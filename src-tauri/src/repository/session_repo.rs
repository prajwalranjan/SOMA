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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
            CREATE TABLE chat_sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE chat_history (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn create_session_and_fetch() {
        let conn = test_conn();
        let repo = SqliteSessionRepository { conn: &conn };

        let session = repo.create_session("Test chat").unwrap();
        assert_eq!(session.name, "Test chat");

        let all = repo.get_all_sessions().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, session.id);
    }

    #[test]
    fn ensure_default_session_creates_one_if_none_exist() {
        let conn = test_conn();
        let repo = SqliteSessionRepository { conn: &conn };

        let all_before = repo.get_all_sessions().unwrap();
        assert_eq!(all_before.len(), 0);

        let default = repo.ensure_default_session().unwrap();
        assert_eq!(default.name, "New chat");

        let all_after = repo.get_all_sessions().unwrap();
        assert_eq!(all_after.len(), 1);
    }

    #[test]
    fn ensure_default_session_returns_existing_if_present() {
        let conn = test_conn();
        let repo = SqliteSessionRepository { conn: &conn };

        let first = repo.create_session("Existing").unwrap();
        let ensured = repo.ensure_default_session().unwrap();

        assert_eq!(
            ensured.id, first.id,
            "should return existing session, not create a new one"
        );
    }

    #[test]
    fn messages_are_isolated_between_sessions() {
        let conn = test_conn();
        let repo = SqliteSessionRepository { conn: &conn };

        let session_a = repo.create_session("Session A").unwrap();
        let session_b = repo.create_session("Session B").unwrap();

        repo.save_message(&ChatMessage {
            session_id: session_a.id.clone(),
            role: "user".to_string(),
            content: "hello from A".to_string(),
            timestamp: "t1".to_string(),
        })
        .unwrap();

        repo.save_message(&ChatMessage {
            session_id: session_b.id.clone(),
            role: "user".to_string(),
            content: "hello from B".to_string(),
            timestamp: "t1".to_string(),
        })
        .unwrap();

        let messages_a = repo.get_messages(&session_a.id).unwrap();
        let messages_b = repo.get_messages(&session_b.id).unwrap();

        assert_eq!(messages_a.len(), 1);
        assert_eq!(messages_b.len(), 1);
        assert_eq!(messages_a[0].content, "hello from A");
        assert_eq!(messages_b[0].content, "hello from B");
    }

    #[test]
    fn delete_session_cascades_to_messages() {
        let conn = test_conn();
        let repo = SqliteSessionRepository { conn: &conn };

        let session = repo.create_session("To delete").unwrap();
        repo.save_message(&ChatMessage {
            session_id: session.id.clone(),
            role: "user".to_string(),
            content: "doomed message".to_string(),
            timestamp: "t1".to_string(),
        })
        .unwrap();

        repo.delete_session(&session.id).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM chat_history WHERE session_id = ?1",
                rusqlite::params![session.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "messages should cascade delete with session");
    }
}
