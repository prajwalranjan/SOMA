use crate::models::{ChatMessage, Insight, Note};
use crate::repository::insight_repo::SqliteInsightRepository;
use crate::repository::note_repo::NoteRepository;
use crate::repository::note_repo::SqliteNoteRepository;
use crate::services::{ChatService, EmbeddingService, InsightService, RetrievalService};
use chrono::Utc;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn add_note(
    content: String,
    thought_at: Option<String>,
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<Note, String> {
    let note = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let repo = SqliteNoteRepository { conn: &conn };
        repo.create(&content, thought_at)
            .map_err(|e| e.to_string())?
    };

    // Chunk and save chunks
    let chunking_svc = crate::services::ChunkingService::new();
    let chunks = chunking_svc.chunk(&note.id, &content);

    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let chunk_repo = crate::repository::chunk_repo::SqliteChunkRepository { conn: &conn };
        crate::repository::chunk_repo::ChunkRepository::save_chunks(&chunk_repo, &chunks)
            .map_err(|e| e.to_string())?;
    }

    // Generate embeddings for each chunk in background
    let db_clone = db.inner().clone();
    tokio::spawn(async move {
        let svc = EmbeddingService::new();
        for chunk in &chunks {
            match svc.generate(&chunk.id, &chunk.content).await {
                Ok(embedding) => {
                    let conn = db_clone.lock().unwrap();
                    let chunk_repo =
                        crate::repository::chunk_repo::SqliteChunkRepository { conn: &conn };
                    let _ = crate::repository::chunk_repo::ChunkRepository::update_embedding(
                        &chunk_repo,
                        &chunk.id,
                        &embedding.vector,
                    );
                }
                Err(e) => eprintln!("Chunk embedding failed: {}", e),
            }
        }
    });

    Ok(note)
}

#[tauri::command]
pub fn get_notes(db: State<'_, Arc<Mutex<Connection>>>) -> Result<Vec<Note>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let repo = SqliteNoteRepository { conn: &conn };
    repo.get_all().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn chat(query: String, db: State<'_, Arc<Mutex<Connection>>>) -> Result<String, String> {
    println!("chat command called with query: {}", query);
    let embedding_svc = EmbeddingService::new();
    let retrieval_svc = RetrievalService::new();
    let chat_svc = ChatService::new();

    println!("about to generate query embedding");
    let query_embedding = embedding_svc
        .generate("query", &query)
        .await
        .map_err(|e| e.to_string())?;

    println!(
        "query embedding generated, len: {}",
        query_embedding.vector.len()
    );

    let relevant_notes = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        println!("got db lock");
        let note_repo = SqliteNoteRepository { conn: &conn };
        let chunk_repo = crate::repository::chunk_repo::SqliteChunkRepository { conn: &conn };
        let count = crate::repository::chunk_repo::ChunkRepository::count_chunks_with_embeddings(
            &chunk_repo,
        )
        .map_err(|e| e.to_string())?;
        println!("chunk count: {}", count);

        if count >= 3 {
            let all_chunks =
                crate::repository::chunk_repo::ChunkRepository::get_all_chunks_with_embeddings(
                    &chunk_repo,
                )
                .map_err(|e| e.to_string())?;
            println!("fetched {} chunks", all_chunks.len());

            let mut scored: Vec<(f32, String)> = all_chunks
                .iter()
                .filter_map(|c| {
                    c.embedding.as_ref().map(|v| {
                        let sim = EmbeddingService::cosine_similarity(&query_embedding.vector, v);
                        (sim, c.note_id.clone())
                    })
                })
                .collect();
            println!("scored {} chunks", scored.len());

            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            scored.dedup_by_key(|s| s.1.clone());
            scored.truncate(5);
            println!("after dedup/truncate: {} candidates", scored.len());

            let top_note_ids: Vec<String> = scored
                .into_iter()
                .filter(|(score, _)| *score > 0.3)
                .map(|(_, id)| id)
                .collect();
            println!("top note ids: {:?}", top_note_ids);

            note_repo
                .get_notes_by_ids(&top_note_ids)
                .map_err(|e| e.to_string())?
        } else {
            println!("using fulltext search");
            note_repo
                .search_fulltext(&query)
                .map_err(|e| e.to_string())?
        }
    };
    println!("relevant notes fetched: {}", relevant_notes.len());

    println!("calling chat_svc.respond");
    let result = chat_svc
        .respond(&query, &relevant_notes)
        .await
        .map_err(|e| {
            println!("chat_svc.respond error: {}", e);
            e.to_string()
        });
    println!("chat_svc.respond returned ok: {:?}", result.is_ok());
    result
}

#[tauri::command]
pub fn save_message(
    role: String,
    content: String,
    timestamp: String,
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO chat_history (id, role, content, timestamp) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id, role, content, timestamp],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_chat_history(db: State<'_, Arc<Mutex<Connection>>>) -> Result<Vec<ChatMessage>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT role, content, timestamp FROM chat_history ORDER BY timestamp ASC")
        .map_err(|e| e.to_string())?;

    let messages = stmt
        .query_map([], |row| {
            Ok(ChatMessage {
                role: row.get(0)?,
                content: row.get(1)?,
                timestamp: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(messages)
}

#[tauri::command]
pub fn get_insights(db: State<'_, Arc<Mutex<Connection>>>) -> Result<Vec<Insight>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let repo = SqliteInsightRepository { conn: &conn };
    crate::repository::insight_repo::InsightRepository::get_all(&repo).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_insights(
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<Vec<Insight>, String> {
    let insight_svc = InsightService::new();

    let (embeddings, existing_insights) = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let note_repo = SqliteNoteRepository { conn: &conn };
        let insight_repo = SqliteInsightRepository { conn: &conn };
        let chunk_repo = crate::repository::chunk_repo::SqliteChunkRepository { conn: &conn };

        // Use chunk embeddings instead of note embeddings
        let chunks =
            crate::repository::chunk_repo::ChunkRepository::get_all_chunks_with_embeddings(
                &chunk_repo,
            )
            .map_err(|e| e.to_string())?;

        let embeddings: Vec<crate::models::Embedding> = chunks
            .iter()
            .filter_map(|c| {
                c.embedding.as_ref().map(|v| crate::models::Embedding {
                    note_id: c.note_id.clone(),
                    vector: v.clone(),
                    model: "nomic-embed-text".to_string(),
                    created_at: c.created_at.clone(),
                })
            })
            .collect();

        let existing = crate::repository::insight_repo::InsightRepository::get_all(&insight_repo)
            .map_err(|e| e.to_string())?;

        (embeddings, existing)
    };

    let clusters = insight_svc.cluster_embeddings(&embeddings);
    let mut new_insights = vec![];

    for cluster_ids in clusters {
        let note_ids_json = serde_json::to_string(&cluster_ids).map_err(|e| e.to_string())?;

        let already_exists = existing_insights
            .iter()
            .any(|i| serde_json::to_string(&i.note_ids).ok().as_deref() == Some(&note_ids_json));

        if already_exists {
            continue;
        }

        let notes = {
            let conn = db.lock().map_err(|e| e.to_string())?;
            let repo = SqliteNoteRepository { conn: &conn };
            repo.get_notes_by_ids(&cluster_ids)
                .map_err(|e| e.to_string())?
        };

        if notes.is_empty() {
            continue;
        }

        match insight_svc.generate_insight_text(&notes).await {
            Ok((title, body)) => {
                let insight = Insight {
                    id: Uuid::new_v4().to_string(),
                    title,
                    body,
                    created_at: Utc::now().to_rfc3339(),
                    note_ids: cluster_ids,
                };
                let conn = db.lock().map_err(|e| e.to_string())?;
                let insight_repo = SqliteInsightRepository { conn: &conn };
                crate::repository::insight_repo::InsightRepository::save(
                    &insight_repo,
                    &insight,
                    &note_ids_json,
                )
                .map_err(|e| e.to_string())?;
                new_insights.push(insight);
            }
            Err(e) => eprintln!("Insight generation failed: {}", e),
        }
    }

    Ok(new_insights)
}

#[tauri::command]
pub async fn check_ollama() -> Result<bool, String> {
    let client = reqwest::Client::new();
    match client.get("http://localhost:11434/api/tags").send().await {
        Ok(res) => {
            let text = res.text().await.unwrap_or_default();
            Ok(text.contains("nomic-embed-text") && text.contains("phi3"))
        }
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub async fn update_note(
    id: String,
    content: String,
    thought_at: Option<String>,
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<Note, String> {
    let note = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let repo = SqliteNoteRepository { conn: &conn };
        repo.update(&id, &content, thought_at)
            .map_err(|e| e.to_string())?
    };

    // Delete old chunks, create new ones, re-embed in background
    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let chunk_repo = crate::repository::chunk_repo::SqliteChunkRepository { conn: &conn };
        crate::repository::chunk_repo::ChunkRepository::delete_chunks_for_note(&chunk_repo, &id)
            .map_err(|e| e.to_string())?;
    }

    let chunking_svc = crate::services::ChunkingService::new();
    let chunks = chunking_svc.chunk(&note.id, &content);

    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let chunk_repo = crate::repository::chunk_repo::SqliteChunkRepository { conn: &conn };
        crate::repository::chunk_repo::ChunkRepository::save_chunks(&chunk_repo, &chunks)
            .map_err(|e| e.to_string())?;
    }

    let db_clone = db.inner().clone();
    tokio::spawn(async move {
        let svc = EmbeddingService::new();
        for chunk in &chunks {
            match svc.generate(&chunk.id, &chunk.content).await {
                Ok(embedding) => {
                    let conn = db_clone.lock().unwrap();
                    let chunk_repo =
                        crate::repository::chunk_repo::SqliteChunkRepository { conn: &conn };
                    let _ = crate::repository::chunk_repo::ChunkRepository::update_embedding(
                        &chunk_repo,
                        &chunk.id,
                        &embedding.vector,
                    );
                }
                Err(e) => eprintln!("Chunk embedding failed: {}", e),
            }
        }
    });

    Ok(note)
}

#[tauri::command]
pub fn delete_note(id: String, db: State<'_, Arc<Mutex<Connection>>>) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let repo = SqliteNoteRepository { conn: &conn };
    repo.delete(&id).map_err(|e| e.to_string())
}
