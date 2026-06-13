use rusqlite::{Connection, Result};
use std::path::Path;

pub fn init_db(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;

        CREATE TABLE IF NOT EXISTS notes (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            thought_at TEXT NOT NULL,
            logged_at TEXT NOT NULL,
            sentiment TEXT,
            embedding_ref TEXT
        );

        CREATE TABLE IF NOT EXISTS insights (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            created_at TEXT NOT NULL,
            note_ids TEXT NOT NULL
        );
    ")?;
    Ok(conn)
}