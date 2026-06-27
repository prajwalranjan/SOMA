use rusqlite::{Connection, Result};
use std::path::Path;

pub fn init_db(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;

        CREATE TABLE IF NOT EXISTS notes (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            thought_at TEXT NOT NULL,
            logged_at TEXT NOT NULL,
            sentiment TEXT,
            embedding_ref TEXT,
            content_type TEXT DEFAULT 'thought'
        );

        CREATE TABLE IF NOT EXISTS note_chunks (
            id TEXT PRIMARY KEY,
            note_id TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            content TEXT NOT NULL,
            embedding TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS insights (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            created_at TEXT NOT NULL,
            note_ids TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS chat_sessions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS chat_history (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
        );
    ",
    )?;

    // Migrations for columns added after initial schema.
    // SQLite has no ADD COLUMN IF NOT EXISTS; attempt each and swallow only
    // "duplicate column name" errors (SQLITE_ERROR with that message text).
    for sql in [
        "ALTER TABLE note_chunks ADD COLUMN clustering_embedding TEXT",
        "ALTER TABLE note_chunks ADD COLUMN embedding_model TEXT",
        "ALTER TABLE note_chunks ADD COLUMN clustering_embedding_model TEXT",
    ] {
        if let Err(e) = conn.execute(sql, []) {
            let msg = e.to_string();
            if !msg.contains("duplicate column name") {
                return Err(e);
            }
        }
    }

    Ok(conn)
}
