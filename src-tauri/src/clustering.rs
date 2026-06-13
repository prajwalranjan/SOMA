use crate::embeddings::{cosine_similarity, get_all_embeddings};
use anyhow::Result;
use chrono::{DateTime, Timelike, Utc};
use rusqlite::Connection;

pub struct Cluster {
    pub note_ids: Vec<String>,
    pub temporal_pattern: Option<TemporalPattern>,
}

pub struct TemporalPattern {
    pub description: String,
    pub strength: f32, // 0.0 to 1.0
}

// DBSCAN parameters
const EPSILON: f32 = 0.4; // max cosine distance for neighbours (1 - similarity)
const MIN_POINTS: usize = 2; // minimum notes to form a cluster

pub fn run_clustering(conn: &Connection) -> Result<Vec<Cluster>> {
    let embeddings = get_all_embeddings(conn)?;

    if embeddings.len() < MIN_POINTS {
        return Ok(vec![]);
    }

    let n = embeddings.len();
    let mut labels: Vec<i32> = vec![-1; n]; // -1 = unvisited
    let mut cluster_id = 0i32;

    for i in 0..n {
        if labels[i] != -1 {
            continue;
        }

        let neighbours = region_query(&embeddings, i);

        if neighbours.len() < MIN_POINTS {
            labels[i] = -2; // noise
            continue;
        }

        labels[i] = cluster_id;
        let mut seed_set = neighbours.clone();
        let mut j = 0;

        while j < seed_set.len() {
            let q = seed_set[j];
            if labels[q] == -2 {
                labels[q] = cluster_id;
            }
            if labels[q] != -1 {
                j += 1;
                continue;
            }
            labels[q] = cluster_id;
            let q_neighbours = region_query(&embeddings, q);
            if q_neighbours.len() >= MIN_POINTS {
                for &nb in &q_neighbours {
                    if !seed_set.contains(&nb) {
                        seed_set.push(nb);
                    }
                }
            }
            j += 1;
        }

        cluster_id += 1;
    }

    // Group note_ids by cluster
    let mut clusters: Vec<Vec<String>> = vec![vec![]; cluster_id as usize];
    for (i, &label) in labels.iter().enumerate() {
        if label >= 0 {
            clusters[label as usize].push(embeddings[i].0.clone());
        }
    }

    // Analyse temporal patterns per cluster
    let results = clusters
        .into_iter()
        .filter(|c| c.len() >= MIN_POINTS)
        .map(|note_ids| {
            let pattern = detect_temporal_pattern(conn, &note_ids);
            Cluster {
                note_ids,
                temporal_pattern: pattern,
            }
        })
        .collect();

    Ok(results)
}

fn region_query(embeddings: &[(String, Vec<f32>, String)], idx: usize) -> Vec<usize> {
    embeddings
        .iter()
        .enumerate()
        .filter(|(j, (_, emb, _))| {
            if *j == idx {
                return false;
            }
            let sim = cosine_similarity(&embeddings[idx].1, emb);
            let distance = 1.0 - sim;
            distance <= EPSILON
        })
        .map(|(j, _)| j)
        .collect()
}

fn detect_temporal_pattern(conn: &Connection, note_ids: &[String]) -> Option<TemporalPattern> {
    let placeholders: String = note_ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 1))
        .collect::<Vec<_>>()
        .join(", ");

    let query = format!(
        "SELECT thought_at FROM notes WHERE id IN ({})",
        placeholders
    );

    let mut stmt = conn.prepare(&query).ok()?;
    let params: Vec<&dyn rusqlite::ToSql> =
        note_ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

    let timestamps: Vec<DateTime<Utc>> = stmt
        .query_map(params.as_slice(), |row| row.get::<_, String>(0))
        .ok()?
        .filter_map(|r| r.ok())
        .filter_map(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .collect();

    if timestamps.is_empty() {
        return None;
    }

    // Check for time-of-day pattern
    let hours: Vec<u32> = timestamps.iter().map(|t| t.hour()).collect();
    let avg_hour = hours.iter().sum::<u32>() as f32 / hours.len() as f32;
    let variance = hours
        .iter()
        .map(|&h| {
            let diff = h as f32 - avg_hour;
            diff * diff
        })
        .sum::<f32>()
        / hours.len() as f32;

    // Low variance = strong temporal clustering
    let strength = 1.0 - (variance / 144.0).min(1.0); // 144 = 12^2, max variance for hours

    if strength > 0.6 {
        let time_label = match avg_hour as u32 {
            0..=5 => "late at night",
            6..=9 => "in the morning",
            10..=12 => "around midday",
            13..=17 => "in the afternoon",
            18..=21 => "in the evening",
            _ => "at night",
        };

        return Some(TemporalPattern {
            description: format!("You tend to write about this {}", time_label),
            strength,
        });
    }

    None
}
