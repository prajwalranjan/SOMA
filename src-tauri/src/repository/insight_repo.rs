use crate::models::Insight;
use anyhow::Result;
use rusqlite::Connection;

pub trait InsightRepository {
    fn save(&self, insight: &Insight, note_ids_json: &str) -> Result<()>;
    fn get_all(&self) -> Result<Vec<Insight>>;
    fn exists(&self, note_ids_json: &str) -> Result<bool>;
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

    fn exists(&self, note_ids_json: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM insights WHERE note_ids = ?1",
            rusqlite::params![note_ids_json],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}
