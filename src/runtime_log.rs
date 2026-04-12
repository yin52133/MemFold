use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::config::MemfoldConfig;
use crate::error::Result;
use crate::timestamps::now_rfc3339;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const HEARTBEAT_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEventKind {
    Started,
    Heartbeat,
    Finished,
    Failed,
}

#[derive(Debug, Serialize)]
pub struct RuntimeLogEvent<'a> {
    pub session_id: &'a str,
    pub command: &'a str,
    pub stage: &'a str,
    pub event: RuntimeEventKind,
    pub elapsed_ms: u128,
    pub message: Option<&'a str>,
    pub created_at: &'a str,
}

pub struct StageLogger {
    config: MemfoldConfig,
    session_id: String,
    command: String,
    stage: String,
    started_at: Instant,
    stop_heartbeat: Arc<AtomicBool>,
    heartbeat_handle: Option<thread::JoinHandle<()>>,
}

impl StageLogger {
    pub fn start(config: &MemfoldConfig, command: &str, stage: &str) -> Self {
        let session_id = current_session_id();
        let logger = Self {
            config: config.clone(),
            session_id,
            command: command.to_string(),
            stage: stage.to_string(),
            started_at: Instant::now(),
            stop_heartbeat: Arc::new(AtomicBool::new(false)),
            heartbeat_handle: None,
        };
        let _ = logger.emit(RuntimeEventKind::Started, Some("stage started"));
        eprintln!("[{}] status=started", logger.stage);
        logger.spawn_heartbeat()
    }

    pub fn finish(mut self, message: Option<&str>) {
        let elapsed_ms = self.started_at.elapsed().as_millis();
        self.stop();
        let _ = self.emit(RuntimeEventKind::Finished, message);
        eprintln!(
            "[{}] status=finished elapsed={:.1}s",
            self.stage,
            elapsed_ms as f64 / 1000.0
        );
    }

    pub fn fail(mut self, message: &str) {
        let elapsed_ms = self.started_at.elapsed().as_millis();
        self.stop();
        let _ = self.emit(RuntimeEventKind::Failed, Some(message));
        eprintln!(
            "[{}] status=failed elapsed={:.1}s error={}",
            self.stage,
            elapsed_ms as f64 / 1000.0,
            message
        );
    }

    fn spawn_heartbeat(mut self) -> Self {
        let stop = self.stop_heartbeat.clone();
        let config = self.config.clone();
        let session_id = self.session_id.clone();
        let command = self.command.clone();
        let stage = self.stage.clone();
        let started_at = self.started_at;
        self.heartbeat_handle = Some(thread::spawn(move || {
            let mut last_heartbeat = Instant::now();
            while !stop.load(Ordering::Relaxed) {
                thread::sleep(HEARTBEAT_POLL_INTERVAL);
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                if last_heartbeat.elapsed() < HEARTBEAT_INTERVAL {
                    continue;
                }
                last_heartbeat = Instant::now();
                let elapsed_ms = started_at.elapsed().as_millis();
                let created_at = now_rfc3339();
                let event = RuntimeLogEvent {
                    session_id: &session_id,
                    command: &command,
                    stage: &stage,
                    event: RuntimeEventKind::Heartbeat,
                    elapsed_ms,
                    message: Some("stage still running"),
                    created_at: &created_at,
                };
                let _ = append_runtime_event(&config, &event);
                eprintln!(
                    "[{}] status=running elapsed={:.1}s",
                    stage,
                    elapsed_ms as f64 / 1000.0
                );
            }
        }));
        self
    }

    fn stop(&mut self) {
        self.stop_heartbeat.store(true, Ordering::Relaxed);
        if let Some(handle) = self.heartbeat_handle.take() {
            let _ = handle.join();
        }
    }

    fn emit(&self, event: RuntimeEventKind, message: Option<&str>) -> Result<()> {
        let created_at = now_rfc3339();
        let payload = RuntimeLogEvent {
            session_id: &self.session_id,
            command: &self.command,
            stage: &self.stage,
            event,
            elapsed_ms: self.started_at.elapsed().as_millis(),
            message,
            created_at: &created_at,
        };
        append_runtime_event(&self.config, &payload)
    }
}

impl Drop for StageLogger {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn current_session_id() -> String {
    std::env::var("MEMFOLD_SESSION_ID").unwrap_or_else(|_| format!("adhoc_{}", std::process::id()))
}

pub fn runtime_log_path(config: &MemfoldConfig, session_id: &str) -> PathBuf {
    config
        .root
        .join("runtime")
        .join("logs")
        .join(format!("{session_id}.jsonl"))
}

pub fn append_runtime_event(config: &MemfoldConfig, event: &RuntimeLogEvent<'_>) -> Result<()> {
    let path = runtime_log_path(config, event.session_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    serde_json::to_writer(&mut file, event)?;
    file.write_all(b"\n")?;
    file.flush()?;
    Ok(())
}
