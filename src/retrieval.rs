use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::config::MemfoldConfig;
use crate::domain::Intent;
use crate::domain::ScopeRef;
use crate::error::{Error, Result};
use crate::qmd_adapter::{embed_query, load_scope_records, sync_scope, QmdRecord};
use crate::state::schema;
use crate::token_estimate;

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

    let tombstoned_claims = load_tombstoned_claims(config, scope)?;
    let tombstoned_summaries = load_tombstoned_summaries(config, scope)?;
    let records = load_scope_records(config, scope)?
        .into_iter()
        .filter(|record| {
            let fingerprint_ok = record
                .claim_fingerprint
                .as_ref()
                .map(|fingerprint| !tombstoned_claims.contains(fingerprint))
                .unwrap_or(true);
            let history_ok = if record.source_type == "history" {
                !record.history_has_user_signal.unwrap_or(false)
                    || !tombstoned_summaries.contains(&canonical_summary_key(&record.summary))
            } else {
                true
            };
            fingerprint_ok && history_ok
        })
        .collect::<Vec<_>>();
    let query_tokens = normalize_tokens(query);
    let query_embedding = query_embedding(config, query);
    let mut exact_matches = records
        .iter()
        .map(|record| {
            (
                record.doc_id.clone(),
                exact_match_bonus(&record.summary, record.raw_text.as_deref(), query) > 0,
            )
        })
        .collect::<HashMap<_, _>>();
    let mut scores = records
        .iter()
        .map(|record| {
            (
                record.doc_id.clone(),
                score_record(record, query, &query_tokens, query_embedding.as_deref()),
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
        if exact_matches.get(&record.doc_id).copied().unwrap_or(false) {
            exact_matches.insert(stable_doc_id.clone(), true);
        }
    }

    let mut records = records
        .into_iter()
        .filter(|record| scores.get(&record.doc_id).copied().unwrap_or(0) > 0)
        .collect::<Vec<_>>();
    let has_exact_match = exact_matches.values().any(|matched| *matched);
    if has_exact_match && is_token_like_query(query) {
        records.retain(|record| exact_matches.get(&record.doc_id).copied().unwrap_or(false));
    }
    records.sort_by(|left, right| {
        let left_key = (
            std::cmp::Reverse(exact_matches.get(&left.doc_id).copied().unwrap_or(false)),
            source_priority(&left.source_type, intent),
            std::cmp::Reverse(scores.get(&left.doc_id).copied().unwrap_or(0)),
            left.pointer.clone(),
        );
        let right_key = (
            std::cmp::Reverse(exact_matches.get(&right.doc_id).copied().unwrap_or(false)),
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

fn score_record(
    record: &QmdRecord,
    query: &str,
    query_tokens: &[String],
    query_embedding: Option<&[f32]>,
) -> usize {
    let search_text = record
        .raw_text
        .as_deref()
        .map(|raw| format!("{} {}", record.summary, raw))
        .unwrap_or_else(|| record.summary.clone());
    let exact_bonus = exact_match_bonus(&record.summary, record.raw_text.as_deref(), query);
    let haystack = normalize_tokens(&search_text);
    let normalized_summary = normalized_search_text(&record.summary);
    let normalized_raw = record.raw_text.as_deref().map(normalized_search_text);
    let normalized_query = normalized_search_text(query);
    let lexical = query_tokens
        .iter()
        .filter(|token| {
            haystack.iter().any(|candidate| candidate == *token)
                || normalized_summary.contains(token.as_str())
                || normalized_raw
                    .as_deref()
                    .map(|raw| raw.contains(token.as_str()))
                    .unwrap_or(false)
        })
        .count();
    let lexical = if lexical == 0
        && !normalized_query.is_empty()
        && (normalized_summary.contains(&normalized_query)
            || normalized_raw
                .as_deref()
                .map(|raw| raw.contains(&normalized_query))
                .unwrap_or(false))
    {
        1
    } else {
        lexical
    };

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

    lexical + semantic + exact_bonus
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
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            current.extend(ch.to_lowercase());
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn estimate_tokens(text: &str) -> usize {
    token_estimate::estimate_tokens(text)
}

fn canonical_summary_key(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_lowercase()
}

fn normalized_search_text(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn exact_match_bonus(summary: &str, raw_text: Option<&str>, query: &str) -> usize {
    let raw_query = query.trim().to_lowercase();
    let normalized_query = normalized_search_text(query);
    let raw_match = !raw_query.is_empty()
        && (summary.to_lowercase().contains(&raw_query)
            || raw_text
                .map(|raw| raw.to_lowercase().contains(&raw_query))
                .unwrap_or(false));
    let normalized_match = normalized_query.len() >= 6
        && (normalized_search_text(summary).contains(&normalized_query)
            || raw_text
                .map(|raw| normalized_search_text(raw).contains(&normalized_query))
                .unwrap_or(false));
    if raw_match || normalized_match {
        100
    } else {
        0
    }
}

fn is_token_like_query(query: &str) -> bool {
    let trimmed = query.trim();
    !trimmed.is_empty()
        && (trimmed.chars().any(|ch| ch.is_ascii_digit())
            || trimmed.contains('-')
            || trimmed.contains('_')
            || trimmed.contains('/')
            || trimmed.contains(':'))
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

fn load_tombstoned_claims(config: &MemfoldConfig, scope: &ScopeRef) -> Result<HashSet<String>> {
    let db_path = config.state_db_path();
    MemfoldConfig::ensure_parent_dir(&db_path)?;
    let conn = Connection::open(db_path)?;
    schema::apply_schema(&conn)?;
    let mut stmt = conn.prepare(
        "SELECT claim_fingerprint
         FROM tombstones
         WHERE scope_type = ?1 AND scope_id = ?2",
    )?;
    let rows = stmt.query_map(
        params![scope.scope_type.as_str(), &scope.scope_id],
        |row| row.get::<_, String>(0),
    )?;
    let claims = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(claims.into_iter().collect())
}

fn load_tombstoned_summaries(config: &MemfoldConfig, scope: &ScopeRef) -> Result<HashSet<String>> {
    let db_path = config.state_db_path();
    MemfoldConfig::ensure_parent_dir(&db_path)?;
    let conn = Connection::open(db_path)?;
    schema::apply_schema(&conn)?;
    let mut stmt = conn.prepare(
        "SELECT summary, claim_fingerprint
         FROM session_log_entries
         WHERE scope_type = ?1
           AND scope_id = ?2
           AND claim_fingerprint IS NOT NULL",
    )?;
    let rows = stmt.query_map(
        params![scope.scope_type.as_str(), &scope.scope_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )?;
    let summary_fingerprints = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    let tombstoned_claims = load_tombstoned_claims(config, scope)?;

    let mut grouped = HashMap::<String, HashSet<String>>::new();
    for (summary, fingerprint) in summary_fingerprints {
        grouped
            .entry(canonical_summary_key(&summary))
            .or_default()
            .insert(fingerprint);
    }

    let fully_tombstoned = grouped
        .into_iter()
        .filter_map(|(summary, fingerprints)| {
            (!fingerprints.is_empty()
                && fingerprints
                    .iter()
                    .all(|fingerprint| tombstoned_claims.contains(fingerprint)))
            .then_some(summary)
        })
        .collect::<HashSet<_>>();
    Ok(fully_tombstoned)
}
