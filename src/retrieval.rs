use serde::Serialize;
use std::collections::{HashMap, HashSet};

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

    let records = load_scope_records(config, scope)?;
    let query_tokens = normalize_tokens(query);
    let query_embedding = query_embedding(config, query);
    let mut scores = records
        .iter()
        .map(|record| {
            (
                record.doc_id.clone(),
                score_record(record, &query_tokens, query_embedding.as_deref()),
            )
        })
        .collect::<HashMap<_, _>>();
    let stable_by_claim = records
        .iter()
        .filter(|record| record.source_type == "stable")
        .filter_map(|record| {
            record
                .claim_fingerprint
                .as_ref()
                .map(|fingerprint| (fingerprint.clone(), record.doc_id.clone()))
        })
        .collect::<HashMap<_, _>>();

    for record in &records {
        let score = scores.get(&record.doc_id).copied().unwrap_or(0);
        if score == 0 {
            continue;
        }
        let Some(claim_fingerprint) = record.claim_fingerprint.as_ref() else {
            continue;
        };
        let Some(stable_doc_id) = stable_by_claim.get(claim_fingerprint) else {
            continue;
        };
        if stable_doc_id == &record.doc_id {
            continue;
        }
        let stable_score = scores.entry(stable_doc_id.clone()).or_insert(0);
        *stable_score = (*stable_score).max(score);
    }

    let mut records = records
        .into_iter()
        .filter(|record| scores.get(&record.doc_id).copied().unwrap_or(0) > 0)
        .collect::<Vec<_>>();
    records.sort_by(|left, right| {
        let left_key = (
            source_priority(&left.source_type, intent),
            std::cmp::Reverse(scores.get(&left.doc_id).copied().unwrap_or(0)),
            left.pointer.clone(),
        );
        let right_key = (
            source_priority(&right.source_type, intent),
            std::cmp::Reverse(scores.get(&right.doc_id).copied().unwrap_or(0)),
            right.pointer.clone(),
        );
        left_key.cmp(&right_key)
    });

    let mut results = Vec::new();
    let mut seen_summaries = HashSet::new();
    let mut used_budget = 0usize;
    for record in records {
        let summary_key = canonical_summary_key(&record.summary);
        if !seen_summaries.insert(summary_key) {
            continue;
        }
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
    let search_text = record
        .raw_text
        .as_deref()
        .map(|raw| format!("{} {}", record.summary, raw))
        .unwrap_or_else(|| record.summary.clone());
    let haystack = normalize_tokens(&search_text);
    let summary_lower = search_text.to_lowercase();
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

fn canonical_summary_key(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_lowercase()
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
