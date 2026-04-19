use std::fs;
use std::path::{Path, PathBuf};

use fastembed::{EmbeddingModel as FastEmbeddingModel, InitOptions, TextEmbedding};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::MemfoldConfig;
use crate::domain::ScopeRef;
use crate::error::Result;
use crate::noise::is_memory_noise;
use crate::state::schema;
use crate::timestamps::now_rfc3339;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QmdRecord {
    pub doc_id: String,
    pub source_type: String,
    pub relative_path: String,
    pub pointer: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_has_user_signal: Option<bool>,
    pub status: String,
    pub scope_type: String,
    pub scope_id: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QmdModelInitResult {
    pub enabled: bool,
    pub model: String,
    pub cache_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QmdConfig {
    enabled: bool,
    model: String,
    cache_dir: String,
}

pub fn sync_scope(config: &MemfoldConfig, scope: &ScopeRef) -> Result<usize> {
    let stable_records = build_stable_records(config, scope)?;
    let session_log_records = build_evidence_records(config, scope)?;
    let history_records = build_archive_records(config, scope)?;

    let mut embedder = load_embedder(config)?;
    let stable_records = embed_records(stable_records, embedder.as_mut());
    let session_log_records = embed_records(session_log_records, embedder.as_mut());
    let history_records = embed_records(history_records, embedder.as_mut());

    write_collection(config, scope, "stable", &stable_records)?;
    write_collection(config, scope, "session_log", &session_log_records)?;
    write_collection(config, scope, "history", &history_records)?;

    Ok(stable_records.len() + session_log_records.len() + history_records.len())
}

pub fn init_model(config: &MemfoldConfig, model: &str) -> Result<QmdModelInitResult> {
    let cache_dir = config.root.join("qmd").join("models");
    fs::create_dir_all(&cache_dir)?;
    if let Some(parent) = qmd_config_path(config).parent() {
        fs::create_dir_all(parent)?;
    }
    let qmd_config = QmdConfig {
        enabled: true,
        model: model.to_string(),
        cache_dir: cache_dir.display().to_string(),
    };

    if model != "mock-test" {
        let _ = create_fastembed(model, &cache_dir)?;
    }

    fs::write(
        qmd_config_path(config),
        serde_json::to_string_pretty(&qmd_config)?,
    )?;

    Ok(QmdModelInitResult {
        enabled: true,
        model: qmd_config.model,
        cache_dir: qmd_config.cache_dir,
    })
}

pub fn embed_query(config: &MemfoldConfig, text: &str) -> Result<Option<Vec<f32>>> {
    let Some(mut embedder) = load_embedder(config)? else {
        return Ok(None);
    };
    let vector = match &mut embedder {
        Embedder::Mock => mock_embed(text),
        Embedder::Fast(model) => {
            let embeddings = model
                .embed(vec![text.to_string()], None)
                .map_err(|err| std::io::Error::other(err.to_string()))?;
            embeddings.into_iter().next().unwrap_or_default()
        }
    };
    Ok(Some(vector))
}

pub fn load_scope_records(config: &MemfoldConfig, scope: &ScopeRef) -> Result<Vec<QmdRecord>> {
    let mut records = Vec::new();
    for source in ["stable", "session_log", "history"] {
        let path = collection_file_path(config, scope, source);
        if !path.exists() {
            continue;
        }

        let contents = fs::read_to_string(path)?;
        for line in contents.lines().filter(|line| !line.trim().is_empty()) {
            records.push(serde_json::from_str::<QmdRecord>(line)?);
        }
    }

    Ok(records)
}

fn build_stable_records(config: &MemfoldConfig, scope: &ScopeRef) -> Result<Vec<QmdRecord>> {
    let conn = open_connection(config)?;
    let stable_dir = config
        .project_root(scope)
        .join("stable");

    let mut files = Vec::new();
    if stable_dir.exists() {
        for entry in fs::read_dir(stable_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                files.push(path);
            }
        }
    }
    files.sort();

    let mut records = Vec::new();
    for file in files {
        let relative_path = make_relative_path(config, &file);
        for item in parse_markdown_items(&file)? {
            if item.status != "stable" {
                continue;
            }
            if let Some(fingerprint) = item.claim_fingerprint.as_deref() {
                if tombstone_exists(&conn, scope, fingerprint)? {
                    continue;
                }
            }

            let (doc_id, updated_at) = conn
                .query_row(
                    "SELECT id, updated_at
                     FROM memory_items
                     WHERE scope_type = ?1
                       AND scope_id = ?2
                       AND item_key = ?3
                       AND deleted_at IS NULL
                     ORDER BY revision DESC, updated_at DESC
                     LIMIT 1",
                    params![scope.scope_type.as_str(), &scope.scope_id, &item.item_key],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?
                .unwrap_or_else(|| {
                    (
                        format!("stable:{}:{}", scope.scope_key(), item.item_key),
                        now_string(),
                    )
                });

            records.push(QmdRecord {
                doc_id,
                source_type: "stable".to_string(),
                relative_path: relative_path.clone(),
                pointer: format!("{relative_path}#{}", item.item_key),
                summary: item.summary,
                raw_text: None,
                claim_fingerprint: item.claim_fingerprint,
                history_has_user_signal: None,
                status: item.status,
                scope_type: scope.scope_type.as_str().to_string(),
                scope_id: scope.scope_id.clone(),
                updated_at,
                embedding: None,
            });
        }
    }

    Ok(records)
}

fn build_evidence_records(config: &MemfoldConfig, scope: &ScopeRef) -> Result<Vec<QmdRecord>> {
    let conn = open_connection(config)?;
    let sessions_dir = config
        .project_root(scope)
        .join("sessions");
    let mut records = Vec::new();

    if !sessions_dir.exists() {
        return Ok(records);
    }

    let mut session_dirs = Vec::new();
    for entry in fs::read_dir(sessions_dir)? {
        session_dirs.push(entry?.path());
    }
    session_dirs.sort();

    for session_dir in session_dirs {
        let file = session_dir.join("session_log.jsonl");
        if !file.exists() {
            continue;
        }

        let relative_path = make_relative_path(config, &file);
        let contents = fs::read_to_string(&file)?;
        for (idx, line) in contents.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(line)?;
            let status = if value["promotable"].as_bool().unwrap_or(false) {
                "promotable"
            } else {
                "recorded"
            };
            let summary = value["summary"].as_str().unwrap_or("").to_string();
            let raw_text = value["raw_text"].as_str().map(|value| value.to_string());
            let claim_fingerprint =
                value["claim_fingerprint"].as_str().map(|value| value.to_string());
            if is_memory_noise(&summary) {
                continue;
            }
            if let Some(fingerprint) = claim_fingerprint.as_deref() {
                if tombstone_exists(&conn, scope, fingerprint)? {
                    continue;
                }
            }

            records.push(QmdRecord {
                doc_id: value["evidence_id"].as_str().unwrap_or("").to_string(),
                source_type: "session_log".to_string(),
                relative_path: relative_path.clone(),
                pointer: format!("{relative_path}#{}", idx + 1),
                summary,
                raw_text,
                claim_fingerprint,
                history_has_user_signal: None,
                status: status.to_string(),
                scope_type: scope.scope_type.as_str().to_string(),
                scope_id: scope.scope_id.clone(),
                updated_at: value["created_at"].as_str().unwrap_or("").to_string(),
                embedding: None,
            });
        }
    }

    Ok(records)
}

fn build_archive_records(config: &MemfoldConfig, scope: &ScopeRef) -> Result<Vec<QmdRecord>> {
    let archive_dir = config
        .project_root(scope)
        .join("history")
        .join("daily");
    let mut records = Vec::new();

    if !archive_dir.exists() {
        return Ok(records);
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(archive_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(path);
        }
    }
    files.sort();

    for file in files {
        let relative_path = make_relative_path(config, &file);
        for (idx, entry) in parse_archive_entries(&file)?.into_iter().enumerate() {
            if is_memory_noise(&entry.summary) {
                continue;
            }
            records.push(QmdRecord {
                doc_id: format!("archive:{}:{}:{}", scope.scope_key(), relative_path, idx + 1),
                source_type: "history".to_string(),
                relative_path: relative_path.clone(),
                pointer: format!("{relative_path}#entry-{}", idx + 1),
                summary: entry.summary,
                raw_text: None,
                claim_fingerprint: None,
                history_has_user_signal: Some(entry.has_user_signal),
                status: "history".to_string(),
                scope_type: scope.scope_type.as_str().to_string(),
                scope_id: scope.scope_id.clone(),
                updated_at: entry.updated_at,
                embedding: None,
            });
        }
    }

    Ok(records)
}

fn write_collection(
    config: &MemfoldConfig,
    scope: &ScopeRef,
    source_type: &str,
    records: &[QmdRecord],
) -> Result<()> {
    let path = collection_file_path(config, scope, source_type);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut contents = String::new();
    for record in records {
        contents.push_str(&serde_json::to_string(record)?);
        contents.push('\n');
    }
    fs::write(path, contents)?;
    Ok(())
}

fn collection_file_path(config: &MemfoldConfig, scope: &ScopeRef, source_type: &str) -> PathBuf {
    config
        .root
        .join("qmd")
        .join("collections")
        .join(scope.scope_dir_fragment())
        .join(format!("{source_type}.jsonl"))
}

fn qmd_config_path(config: &MemfoldConfig) -> PathBuf {
    config.root.join("qmd").join("config").join("model.json")
}

fn open_connection(config: &MemfoldConfig) -> Result<Connection> {
    let db_path = config.state_db_path();
    MemfoldConfig::ensure_parent_dir(&db_path)?;
    let conn = Connection::open(db_path)?;
    schema::apply_schema(&conn)?;
    Ok(conn)
}

fn tombstone_exists(conn: &Connection, scope: &ScopeRef, claim_fingerprint: &str) -> Result<bool> {
    let exists = conn
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM tombstones
                WHERE scope_type = ?1 AND scope_id = ?2 AND claim_fingerprint = ?3
            )",
            params![scope.scope_type.as_str(), &scope.scope_id, claim_fingerprint],
            |row| row.get::<_, i64>(0),
        )?
        != 0;
    Ok(exists)
}

fn make_relative_path(config: &MemfoldConfig, absolute: &Path) -> String {
    absolute
        .strip_prefix(&config.root)
        .unwrap_or(absolute)
        .to_string_lossy()
        .replace('\\', "/")
}

#[derive(Debug)]
struct ParsedMarkdownItem {
    item_key: String,
    status: String,
    claim_fingerprint: Option<String>,
    summary: String,
}

fn parse_markdown_items(path: &Path) -> Result<Vec<ParsedMarkdownItem>> {
    let contents = fs::read_to_string(path)?;
    let mut blocks = Vec::new();
    let mut current_key: Option<String> = None;
    let mut current_lines = Vec::new();

    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("## item_key:") {
            if let Some(item_key) = current_key.take() {
                blocks.push((item_key, std::mem::take(&mut current_lines)));
            }
            current_key = Some(rest.trim().to_string());
        } else if current_key.is_some() {
            current_lines.push(line.to_string());
        }
    }

    if let Some(item_key) = current_key.take() {
        blocks.push((item_key, current_lines));
    }

    let mut items = Vec::new();
    for (item_key, lines) in blocks {
        let mut status = None;
        let mut claim_fingerprint = None;
        let mut body = Vec::new();
        let mut in_body = false;
        for line in lines {
            if !in_body {
                if line.trim().is_empty() {
                    in_body = true;
                    continue;
                }

                if let Some((key, value)) = line.split_once(':') {
                    if key.trim() == "status" {
                        status = Some(value.trim().to_string());
                    } else if key.trim() == "claim_fingerprint" {
                        claim_fingerprint = Some(value.trim().to_string());
                    }
                    continue;
                }
                in_body = true;
            }
            if in_body {
                body.push(line);
            }
        }

        items.push(ParsedMarkdownItem {
            item_key,
            status: status.unwrap_or_else(|| "candidate".to_string()),
            claim_fingerprint,
            summary: body.join("\n").trim().to_string(),
        });
    }

    Ok(items)
}

#[derive(Debug)]
struct ParsedArchiveEntry {
    summary: String,
    updated_at: String,
    has_user_signal: bool,
}

fn parse_archive_entries(path: &Path) -> Result<Vec<ParsedArchiveEntry>> {
    let contents = fs::read_to_string(path)?;
    let mut entries = Vec::new();

    for block in contents.split("\n\n") {
        let mut lines = block.lines();
        let Some(first_heading) = lines.find(|line| line.starts_with("## ")) else {
            continue;
        };

        let updated_at = first_heading
            .trim_start_matches("## ")
            .split(' ')
            .next()
            .unwrap_or_default()
            .to_string();
        let summary = first_heading
            .split("] ")
            .nth(1)
            .unwrap_or_default()
            .trim()
            .to_string();
        let has_user_signal = lines.any(|line| line.trim_start().starts_with("- [user]"));

        entries.push(ParsedArchiveEntry {
            summary,
            updated_at,
            has_user_signal,
        });
    }

    Ok(entries)
}

fn now_string() -> String {
    now_rfc3339()
}

enum Embedder {
    Mock,
    Fast(TextEmbedding),
}

fn load_embedder(config: &MemfoldConfig) -> Result<Option<Embedder>> {
    let path = qmd_config_path(config);
    if !path.exists() {
        return Ok(None);
    }
    let cfg: QmdConfig = serde_json::from_str(&fs::read_to_string(path)?)?;
    if !cfg.enabled {
        return Ok(None);
    }
    if cfg.model == "mock-test" {
        return Ok(Some(Embedder::Mock));
    }

    Ok(Some(Embedder::Fast(create_fastembed(
        &cfg.model,
        Path::new(&cfg.cache_dir),
    )?)))
}

fn create_fastembed(model: &str, cache_dir: &Path) -> Result<TextEmbedding> {
    let model_name = match model {
        "multilingual-e5-small" => FastEmbeddingModel::MultilingualE5Small,
        "bge-small-zh-v1.5" => FastEmbeddingModel::BGESmallZHV15,
        "bge-m3" => FastEmbeddingModel::BGEM3,
        _ => FastEmbeddingModel::MultilingualE5Small,
    };
    let options = InitOptions::new(model_name)
        .with_cache_dir(cache_dir.to_path_buf())
        .with_show_download_progress(true);
    Ok(TextEmbedding::try_new(options).map_err(|err| std::io::Error::other(err.to_string()))?)
}

fn embed_records(records: Vec<QmdRecord>, embedder: Option<&mut Embedder>) -> Vec<QmdRecord> {
    match embedder {
        None => records,
        Some(Embedder::Mock) => records
            .into_iter()
            .map(|mut record| {
                record.embedding = Some(mock_embed(&record.summary));
                record
            })
            .collect(),
        Some(Embedder::Fast(model)) => {
            let summaries = records.iter().map(|record| record.summary.clone()).collect::<Vec<_>>();
            let embeddings = model.embed(summaries, None).unwrap_or_default();
            records
                .into_iter()
                .zip(embeddings.into_iter())
                .map(|(mut record, embedding)| {
                    record.embedding = Some(embedding);
                    record
                })
            .collect()
        }
    }
}

pub(crate) fn mock_embed(text: &str) -> Vec<f32> {
    let mut out = Vec::with_capacity(8);
    let digest = Sha256::digest(text.as_bytes());
    for chunk in digest[..32].chunks(4).take(8) {
        let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        out.push((value as f32) / (u32::MAX as f32));
    }
    out
}
