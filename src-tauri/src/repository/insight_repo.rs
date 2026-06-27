use crate::models::Insight;
use anyhow::Result;
use rusqlite::Connection;

pub trait InsightRepository {
    fn save(&self, insight: &Insight, note_ids_json: &str) -> Result<()>;
    fn get_all(&self) -> Result<Vec<Insight>>;
}

pub struct SqliteInsightRepository<'a> {
    pub conn: &'a Connection,
}

impl<'a> InsightRepository for SqliteInsightRepository<'a> {
    fn save(&self, insight: &Insight, note_ids_json: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO insights (id, title, body, created_at, note_ids)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                insight.id,
                insight.title,
                insight.body,
                insight.created_at,
                note_ids_json,
            ],
        )?;
        Ok(())
    }

    fn get_all(&self) -> Result<Vec<Insight>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, body, created_at, note_ids
             FROM insights ORDER BY created_at DESC",
        )?;

        let insights = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(id, title, body, created_at, note_ids_json)| {
                let note_ids: Vec<String> = serde_json::from_str(&note_ids_json).ok()?;
                Some(Insight {
                    id,
                    title,
                    body,
                    created_at,
                    note_ids,
                })
            })
            .collect();

        Ok(insights)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE insights (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                body TEXT NOT NULL,
                created_at TEXT NOT NULL,
                note_ids TEXT NOT NULL
            );",
        )
        .unwrap();
        conn
    }

    fn save_insight(repo: &SqliteInsightRepository<'_>, id: &str, created_at: &str, note_ids: &[&str]) {
        let insight = Insight {
            id: id.to_string(),
            title: "Test title".to_string(),
            body: "Test body".to_string(),
            created_at: created_at.to_string(),
            note_ids: note_ids.iter().map(|s| s.to_string()).collect(),
        };
        let json = serde_json::to_string(&insight.note_ids).unwrap();
        repo.save(&insight, &json).unwrap();
    }

    #[test]
    fn save_and_get_all_roundtrip() {
        let conn = test_conn();
        let repo = SqliteInsightRepository { conn: &conn };
        save_insight(&repo, "i1", "2024-01-01T00:00:00Z", &["n1", "n2"]);

        let all = repo.get_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "i1");
        assert_eq!(all[0].title, "Test title");
        assert_eq!(all[0].note_ids, vec!["n1".to_string(), "n2".to_string()]);
    }

    #[test]
    fn get_all_returns_newest_first() {
        let conn = test_conn();
        let repo = SqliteInsightRepository { conn: &conn };
        save_insight(&repo, "old", "2024-01-01T00:00:00Z", &["n1"]);
        save_insight(&repo, "new", "2024-06-01T00:00:00Z", &["n2"]);

        let all = repo.get_all().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, "new", "newer insight must come first (ORDER BY created_at DESC)");
        assert_eq!(all[1].id, "old");
    }

    #[test]
    fn row_with_invalid_note_ids_json_is_silently_skipped() {
        let conn = test_conn();
        let repo = SqliteInsightRepository { conn: &conn };
        // Insert a row with malformed JSON in note_ids — get_all must skip it
        // rather than returning an Err or panicking.
        conn.execute(
            "INSERT INTO insights (id, title, body, created_at, note_ids) \
             VALUES ('bad', 'T', 'B', '2024-01-01T00:00:00Z', 'not-valid-json')",
            [],
        )
        .unwrap();

        let all = repo.get_all().unwrap();
        assert_eq!(all.len(), 0, "row with invalid note_ids JSON must be silently dropped");
    }

    #[test]
    fn note_ids_vec_survives_json_roundtrip() {
        let conn = test_conn();
        let repo = SqliteInsightRepository { conn: &conn };
        let ids = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];
        let insight = Insight {
            id: "i2".to_string(),
            title: "T".to_string(),
            body: "B".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            note_ids: ids.clone(),
        };
        repo.save(&insight, &serde_json::to_string(&ids).unwrap()).unwrap();

        let loaded = repo.get_all().unwrap();
        assert_eq!(loaded[0].note_ids, ids);
    }
}
