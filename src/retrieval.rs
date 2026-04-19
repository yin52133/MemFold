use serde::Serialize;

use crate::config::MemfoldConfig;
use crate::domain::Intent;
use crate::domain::ScopeRef;
use crate::error::{Error, Result};
use crate::qmd_adapter::{embed_query, load_scope_records, sync_scope, QmdRecord};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchResult {
    pub source_type: String,
    pub doc_id: String,
    pub pointer: String,
    pub summary: String,
    pub status: String,
    pub scope_type: String,
    pub scope_id: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
}

pub fn search_memories(
    config: &MemfoldConfig,
    scope: &ScopeRef,
    intent: Intent,
    query: &str,
    budget: usize,
) -> Result<SearchResponse> {
    if query.trim().is_empty() {
        return Err(Error::EmptyQuery);
    }

    if load_scope_records(config, scope)?.is_empty() {
        sync_scope(config, scope)?;
    }

    let mut records = load_scope_records(config, scope)?;
    let query_tokens = normalize_tokens(query);
    let query_embedding = query_embedding(config, query);
    records.retain(|record| score_record(record, &query_tokens, query_embedding.as_deref()) > 0);

    records.sort_by(|left, right| {
        let left_key = (
            source_priority(&left.source_type, intent),
            std::cmp::Reverse(score_record(left, &query_tokens, query_embedding.as_deref())),
            left.pointer.clone(),
        );
        let right_key = (
            source_priority(&right.source_type, intent),
            std::cmp::Reverse(score_record(right, &query_tokens, query_embedding.as_deref())),
            right.pointer.clone(),
        );
        left_key.cmp(&right_key)
    });

    let mut results = Vec::new();
    let mut used_budget = 0usize;
    for record in records {
        let token_estimate = estimate_tokens(&record.summary);
        if used_budget + token_estimate > budget {
            break;
        }
        used_budget += token_estimate;
        results.push(SearchResult {
            source_type: record.source_type,
            doc_id: record.doc_id,
            pointer: record.pointer,
            summary: record.summary,
            status: record.status,
            scope_type: record.scope_type,
            scope_id: record.scope_id,
            updated_at: record.updated_at,
        });
    }

    Ok(SearchResponse { results })
}

fn score_record(record: &QmdRecord, query_tokens: &[String], query_embedding: Option<&[f32]>) -> usize {
    let haystack = normalize_tokens(&record.summary);
    let summary_lower = record.summary.to_lowercase();
    let lexical = query_tokens
        .iter()
        .filter(|token| {
            haystack.iter().any(|candidate| candidate == *token)
                || summary_lower.contains(token.as_str())
        })
        .count();

    let semantic = match (&record.embedding, query_embedding) {
        (Some(doc), Some(query)) => {
            let cosine = cosine_similarity(doc, query);
            if lexical == 0 {
                if cosine > 0.95 {
                    2
                } else {
                    0
                }
            } else if cosine > 0.70 {
                3
            } else if cosine > 0.50 {
                2
            } else if cosine > 0.30 {
                1
            } else {
                0
            }
        }
        _ => 0,
    };

    lexical + semantic
}

fn source_priority(source_type: &str, intent: Intent) -> u8 {
    match intent {
        Intent::Continue => match source_type {
            "stable" => 0,
            "history" => 1,
            "session_log" => 2,
            _ => 3,
        },
        Intent::KnowledgeLookup => match source_type {
            "stable" => 0,
            "history" => 1,
            "session_log" => 2,
            _ => 3,
        },
        _ => 0,
    }
}

fn normalize_tokens(text: &str) -> Vec<String> {
    text.split(|ch: char| ch.is_whitespace() || ch.is_ascii_punctuation())
        .filter(|part| !part.trim().is_empty())
        .map(|part| part.trim().to_lowercase())
        .collect()
}

fn estimate_tokens(text: &str) -> usize {
    let count = normalize_tokens(text).len();
    count.max(1)
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut left_norm = 0.0f32;
    let mut right_norm = 0.0f32;
    for (l, r) in left.iter().zip(right.iter()) {
        dot += l * r;
        left_norm += l * l;
        right_norm += r * r;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        dot / (left_norm.sqrt() * right_norm.sqrt())
    }
}

fn query_embedding(config: &MemfoldConfig, query: &str) -> Option<Vec<f32>> {
    embed_query(config, query).ok().flatten()
}
