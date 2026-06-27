use crate::services::embedding_service::cosine_similarity;
use crate::services::ollama_client::{Message, OllamaApi, OllamaClient};
use crate::services::prompt_builder::PromptBuilder;
use anyhow::Result;

// Max chars before a title is considered implausibly long (prompt asks for 5 words).
const TITLE_HARD_CAP: usize = 80;
const TITLE_TRUNCATE_TO: usize = 60;

pub fn parse_insight_response(response: &str) -> (String, String) {
    if response.trim().is_empty() {
        return ("Pattern detected".to_string(), String::new());
    }

    let lower = response.to_lowercase();
    let title_pos = lower.find("title:");
    let insight_pos = lower.find("insight:");

    match (title_pos, insight_pos) {
        (Some(tp), Some(ip)) => {
            let (title_raw, body_raw) = if tp < ip {
                let title_text = response[tp + "title:".len()..ip].trim();
                let body_text = response[ip + "insight:".len()..].trim();
                (title_text, body_text)
            } else {
                // Unusual ordering: insight before title
                let body_text = response[ip + "insight:".len()..tp].trim();
                let title_text = response[tp + "title:".len()..].trim();
                (title_text, body_text)
            };
            (truncate_title(title_raw), body_raw.to_string())
        }
        (Some(tp), None) => {
            let title_text = response[tp + "title:".len()..].trim();
            (truncate_title(title_text), response.to_string())
        }
        (None, Some(ip)) => {
            let body = response[ip + "insight:".len()..].trim().to_string();
            ("Pattern detected".to_string(), body)
        }
        (None, None) => ("Pattern detected".to_string(), response.to_string()),
    }
}

fn truncate_title(raw: &str) -> String {
    if raw.chars().count() <= TITLE_HARD_CAP {
        return raw.to_string();
    }
    let truncated: String = raw.chars().take(TITLE_TRUNCATE_TO).collect();
    // Prefer breaking at a sentence boundary within the truncated portion
    if let Some(pos) = truncated.rfind(['.', '!', '?']) {
        let candidate = truncated[..=pos].trim();
        if !candidate.is_empty() {
            return candidate.to_string();
        }
    }
    truncated.trim_end().to_string()
}

const MIN_POINTS: usize = 2;

fn compute_adaptive_epsilon(embeddings: &[crate::models::Embedding]) -> f32 {
    if embeddings.len() < 6 {
        // Too few pairs for a meaningful mean/median calculation.
        // Use the permissive end of the adaptive range (similarity >= 0.4)
        // so loosely related notes can still cluster.
        return 0.6;
    }

    let mut similarities: Vec<f32> = vec![];

    for i in 0..embeddings.len() {
        for j in (i + 1)..embeddings.len() {
            let sim =
                cosine_similarity(&embeddings[i].vector, &embeddings[j].vector);
            similarities.push(sim);
        }
    }

    similarities.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mean = similarities.iter().sum::<f32>() / similarities.len() as f32;
    let median = similarities[similarities.len() / 2];
    let similarity_threshold = (mean + median) / 2.0;
    let epsilon = 1.0 - similarity_threshold;

    println!(
        "Adaptive EPSILON: {:.3} (mean sim: {:.3}, median sim: {:.3}, threshold: {:.3})",
        epsilon, mean, median, similarity_threshold
    );

    epsilon.clamp(0.3, 0.6)
}

pub struct InsightService<C: OllamaApi = OllamaClient> {
    pub model: String,
    client: C,
}

impl InsightService {
    pub fn new() -> Self {
        Self { model: "phi3:mini".to_string(), client: OllamaClient::new() }
    }

    pub fn with_model(model: impl Into<String>) -> Self {
        Self { model: model.into(), client: OllamaClient::new() }
    }
}

impl<C: OllamaApi> InsightService<C> {
    pub fn with_client(model: impl Into<String>, client: C) -> Self {
        Self { model: model.into(), client }
    }

    pub fn cluster_embeddings(&self, embeddings: &[crate::models::Embedding]) -> Vec<Vec<String>> {
        let epsilon = compute_adaptive_epsilon(embeddings);
        let n = embeddings.len();
        let mut labels: Vec<i32> = vec![-1; n];
        let mut cluster_id = 0i32;

        for i in 0..n {
            if labels[i] != -1 {
                continue;
            }
            let neighbours = self.region_query(embeddings, i, epsilon);
            if neighbours.len() < MIN_POINTS {
                labels[i] = -2;
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
                let q_neighbours = self.region_query(embeddings, q, epsilon);
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

        let mut clusters: Vec<Vec<String>> = vec![vec![]; cluster_id as usize];
        for (i, &label) in labels.iter().enumerate() {
            if label >= 0 {
                clusters[label as usize].push(embeddings[i].note_id.clone());
            }
        }

        clusters.into_iter().filter(|c| c.len() >= MIN_POINTS).collect()
    }

    fn region_query(
        &self,
        embeddings: &[crate::models::Embedding],
        idx: usize,
        epsilon: f32,
    ) -> Vec<usize> {
        embeddings
            .iter()
            .enumerate()
            .filter(|(j, emb)| {
                if *j == idx {
                    return false;
                }
                let sim = cosine_similarity(&embeddings[idx].vector, &emb.vector);
                (1.0 - sim) <= epsilon
            })
            .map(|(j, _)| j)
            .collect()
    }

    pub async fn generate_insight_text(
        &self,
        notes: &[crate::models::Note],
    ) -> Result<(String, String)> {
        let prompt = PromptBuilder::insight_prompt(notes);
        let response = self
            .client
            .chat(
                &self.model,
                vec![Message { role: "user".to_string(), content: prompt }],
            )
            .await?;

        Ok(parse_insight_response(&response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Embedding;

    // --- parse_insight_response tests ---

    #[test]
    fn parse_insight_response_perfect_format() {
        let (title, body) = parse_insight_response("TITLE: Foo\nINSIGHT: Bar");
        assert_eq!(title, "Foo");
        assert_eq!(body, "Bar");
    }

    #[test]
    fn parse_insight_response_lowercase_labels() {
        let (title, body) = parse_insight_response("Title: Foo\nInsight: Bar");
        assert_eq!(title, "Foo");
        assert_eq!(body, "Bar");
    }

    #[test]
    fn parse_insight_response_both_labels_on_one_line() {
        // Exact real-world malformed output from phi3:mini (the production bug).
        let input = "Title: Reflections on Habits and Pleasures Insight: The user finds \
            motivation in exercise, enjoys spicy food like biryani but also \
            acknowledges a dependency on caffeine.";
        let (title, body) = parse_insight_response(input);
        assert_eq!(title, "Reflections on Habits and Pleasures", "should extract title before Insight:");
        assert!(
            body.starts_with("The user finds"),
            "body should start with insight text, got: {body}"
        );
        assert!(
            !body.contains("Title:") && !body.contains("title:"),
            "body must not contain the raw label text"
        );
    }

    #[test]
    fn parse_insight_response_only_title_label() {
        // No insight label — title extracted, full response used as body fallback.
        let input = "TITLE: Morning Routines";
        let (title, body) = parse_insight_response(input);
        assert_eq!(title, "Morning Routines");
        assert_eq!(body, input, "body should be the full response when no INSIGHT: label");
    }

    #[test]
    fn parse_insight_response_no_labels_falls_back() {
        let input = "These notes are about morning routines and coffee habits.";
        let (title, body) = parse_insight_response(input);
        assert_eq!(title, "Pattern detected");
        assert_eq!(body, input);
    }

    #[test]
    fn parse_insight_response_truncates_implausibly_long_title() {
        let long_title = "The user seems to explore multiple themes in their journaling \
            including exercise food preferences and caffeine consumption patterns \
            across different days and moods which is very notable";
        let input = format!("Title: {long_title} Insight: Key themes.");
        let (title, body) = parse_insight_response(&input);
        assert!(
            title.chars().count() <= TITLE_HARD_CAP,
            "title longer than {TITLE_HARD_CAP} chars should be truncated, got: {title}"
        );
        assert_ne!(title, "Pattern detected", "a title label was present, should not fall back");
        assert!(body.contains("Key themes"), "body should contain the insight text");
    }

    #[test]
    fn parse_insight_response_empty_input() {
        let (title, body) = parse_insight_response("");
        assert_eq!(title, "Pattern detected");
        assert_eq!(body, "");
    }

    #[test]
    fn parse_insight_response_whitespace_only_input() {
        let (title, body) = parse_insight_response("   \n\t  ");
        assert_eq!(title, "Pattern detected");
        assert_eq!(body, "");
    }

    fn make_embedding(note_id: &str, vector: Vec<f32>) -> Embedding {
        Embedding {
            note_id: note_id.to_string(),
            vector,
            model: "test".to_string(),
            created_at: "now".to_string(),
        }
    }

    #[test]
    fn cluster_groups_similar_embeddings() {
        let svc = InsightService::new();

        // 8 embeddings in 16 dims: 4 near dim-0 axis, 4 near dim-8 axis.
        // Gives adaptive epsilon enough pairwise signal (28 pairs with clear
        // intra ~0.999 vs inter ~0.0 split) so it clamps to 0.6 and correctly
        // groups a,b,c,d together while keeping them separate from e,f,g,h.
        let mut va = vec![0.0f32; 16]; va[0] = 1.0;
        let mut vb = vec![0.0f32; 16]; vb[0] = 0.98; vb[1] = 0.02;
        let mut vc = vec![0.0f32; 16]; vc[0] = 0.99; vc[1] = 0.01;
        let mut vd = vec![0.0f32; 16]; vd[0] = 0.97; vd[1] = 0.03;
        let mut ve = vec![0.0f32; 16]; ve[8] = 1.0;
        let mut vf = vec![0.0f32; 16]; vf[8] = 0.98; vf[9] = 0.02;
        let mut vg = vec![0.0f32; 16]; vg[8] = 0.99; vg[9] = 0.01;
        let mut vh = vec![0.0f32; 16]; vh[8] = 0.97; vh[9] = 0.03;

        let embeddings = vec![
            make_embedding("a", va),
            make_embedding("b", vb),
            make_embedding("c", vc),
            make_embedding("d", vd),
            make_embedding("e", ve),
            make_embedding("f", vf),
            make_embedding("g", vg),
            make_embedding("h", vh),
        ];

        let clusters = svc.cluster_embeddings(&embeddings);

        let found_ab_together = clusters
            .iter()
            .any(|c| c.contains(&"a".to_string()) && c.contains(&"b".to_string()));
        assert!(
            found_ab_together,
            "near-identical embeddings should cluster together"
        );
    }

    #[test]
    fn cluster_returns_empty_for_too_few_embeddings() {
        let svc = InsightService::new();
        let embeddings = vec![make_embedding("a", vec![1.0, 0.0])];

        let clusters = svc.cluster_embeddings(&embeddings);
        assert_eq!(
            clusters.len(),
            0,
            "single embedding cannot form a cluster (MIN_POINTS=2)"
        );
    }

    #[test]
    fn cluster_separates_dissimilar_groups() {
        let svc = InsightService::new();

        // 6 embeddings in 16 dims: 3 near dim-0 axis, 3 near dim-8 axis.
        // The two groups are orthogonal (inter-cluster cosine sim = 0),
        // so adaptive epsilon clamps to 0.6 and the groups land in separate
        // clusters. Each group has exactly MIN_POINTS=2 neighbours, so all
        // members become core points.
        let mut va1 = vec![0.0f32; 16]; va1[0] = 1.0;
        let mut va2 = vec![0.0f32; 16]; va2[0] = 0.99; va2[1] = 0.01;
        let mut va3 = vec![0.0f32; 16]; va3[0] = 0.98; va3[1] = 0.02;
        let mut vb1 = vec![0.0f32; 16]; vb1[8] = 1.0;
        let mut vb2 = vec![0.0f32; 16]; vb2[8] = 0.99; vb2[9] = 0.01;
        let mut vb3 = vec![0.0f32; 16]; vb3[8] = 0.98; vb3[9] = 0.02;

        let embeddings = vec![
            make_embedding("a1", va1),
            make_embedding("a2", va2),
            make_embedding("a3", va3),
            make_embedding("b1", vb1),
            make_embedding("b2", vb2),
            make_embedding("b3", vb3),
        ];

        let clusters = svc.cluster_embeddings(&embeddings);

        let a_cluster = clusters.iter().find(|c| c.contains(&"a1".to_string()));
        let b_cluster = clusters.iter().find(|c| c.contains(&"b1".to_string()));

        assert!(a_cluster.is_some());
        assert!(b_cluster.is_some());
        assert_ne!(
            a_cluster.unwrap(),
            b_cluster.unwrap(),
            "dissimilar groups should not merge into one cluster"
        );
    }
}
