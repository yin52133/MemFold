use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use clap::{Parser, Subcommand};
use serde::Serialize;

use memfold::boot::{compile_scope_bundle, load_startup_bundle};
use memfold::config::MemfoldConfig;
use memfold::domain::{Intent, Mode, ScopeRef, ScopeType, SourceKind};
use memfold::dreaming::{maybe_run_scheduled_dream, run_dream};
use memfold::error::Error;
use memfold::evidence::{write_evidence, WriteEvidenceInput};
use memfold::experiments::{default_fixture_path, run_fixture};
use memfold::feedback::apply_feedback;
use memfold::hooks::{capture_event, HookCaptureInput, HookEvent};
use memfold::init::initialize_root;
use memfold::qmd_adapter::sync_scope;
use memfold::repair::run_repair;
use memfold::retrieval::search_memories;

#[derive(Parser, Debug)]
#[command(name = "memfold")]
struct Cli {
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Init {},
    Load {
        #[arg(long)]
        mode: String,
        #[arg(long = "scope-type")]
        scope_type: String,
        #[arg(long = "scope-id")]
        scope_id: String,
        #[arg(long)]
        intent: String,
        #[arg(long)]
        budget: Option<usize>,
    },
    WriteEvidence {
        #[arg(long = "scope-type")]
        scope_type: String,
        #[arg(long = "scope-id")]
        scope_id: String,
        #[arg(long = "session-id")]
        session_id: String,
        #[arg(long = "source-kind")]
        source_kind: String,
        #[arg(long)]
        summary: String,
        #[arg(long)]
        promotable: u8,
        #[arg(long = "origin-mode")]
        origin_mode: String,
        #[arg(long = "claim-fingerprint")]
        claim_fingerprint: Option<String>,
    },
    Search {
        #[arg(long = "scope-type")]
        scope_type: String,
        #[arg(long = "scope-id")]
        scope_id: String,
        #[arg(long)]
        intent: String,
        #[arg(long)]
        query: String,
        #[arg(long)]
        budget: usize,
    },
    Qmd {
        #[command(subcommand)]
        command: QmdCommands,
    },
    Bundle {
        #[command(subcommand)]
        command: BundleCommands,
    },
    Feedback {
        #[arg(long = "scope-type")]
        scope_type: String,
        #[arg(long = "scope-id")]
        scope_id: String,
        #[arg(long = "claim-fingerprint")]
        claim_fingerprint: String,
        #[arg(long)]
        verdict: String,
        #[arg(long)]
        reason: String,
        #[arg(long = "session-id")]
        session_id: Option<String>,
    },
    Dream {
        #[command(subcommand)]
        command: DreamCommands,
    },
    Repair {
        #[arg(long = "scope-type")]
        scope_type: Option<String>,
        #[arg(long = "scope-id")]
        scope_id: Option<String>,
    },
    Experiment {
        #[command(subcommand)]
        command: ExperimentCommands,
    },
    Hook {
        #[command(subcommand)]
        command: HookCommands,
    },
}

#[derive(Subcommand, Debug)]
enum QmdCommands {
    Sync {
        #[arg(long = "scope-type")]
        scope_type: String,
        #[arg(long = "scope-id")]
        scope_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum BundleCommands {
    Compile {
        #[arg(long = "scope-type")]
        scope_type: String,
        #[arg(long = "scope-id")]
        scope_id: String,
        #[arg(long, default_value_t = 900)]
        budget: usize,
    },
}

#[derive(Subcommand, Debug)]
enum DreamCommands {
    Run {
        #[arg(long = "scope-type")]
        scope_type: String,
        #[arg(long = "scope-id")]
        scope_id: String,
        #[arg(long)]
        trigger: String,
    },
    MaybeRun {
        #[arg(long = "scope-type")]
        scope_type: String,
        #[arg(long = "scope-id")]
        scope_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum ExperimentCommands {
    Run {
        #[arg(long)]
        fixture: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum HookCommands {
    Capture {
        #[arg(long)]
        event: String,
        #[arg(long = "scope-type")]
        scope_type: String,
        #[arg(long = "scope-id")]
        scope_id: String,
        #[arg(long = "session-id")]
        session_id: String,
        #[arg(long = "source-kind")]
        source_kind: String,
        #[arg(long)]
        summary: String,
        #[arg(long = "origin-mode")]
        origin_mode: String,
        #[arg(long = "state-changed")]
        state_changed: u8,
        #[arg(long)]
        promotable: u8,
    },
}

#[derive(Serialize)]
struct InitPayload {
    created_paths: Vec<String>,
}

#[derive(Serialize)]
struct ScopePayload<'a> {
    #[serde(rename = "type")]
    scope_type: &'a str,
    id: &'a str,
}

#[derive(Serialize)]
struct LoadItemPayload<'a> {
    item_key: &'a str,
    text: &'a str,
}

#[derive(Serialize)]
struct LoadPayload<'a> {
    mode: &'a str,
    scope: ScopePayload<'a>,
    items: Vec<LoadItemPayload<'a>>,
    total_tokens_estimate: usize,
    degraded: bool,
}

#[derive(Serialize)]
struct ErrorPayload<'a> {
    error_code: i32,
    error_name: &'a str,
    message: String,
    retryable: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let payload = ErrorPayload {
                error_code: error.error_code(),
                error_name: error.error_name(),
                message: error.to_string(),
                retryable: error.retryable(),
            };
            eprintln!("{}", serde_json::to_string(&payload).unwrap());
            ExitCode::from(error.error_code() as u8)
        }
    }
}

fn run(cli: Cli) -> Result<(), Error> {
    let root = MemfoldConfig::resolve_root(cli.root);
    let config = MemfoldConfig::default_for_root(root);

    match cli.command {
        Commands::Init {} => {
            let summary = initialize_root(&config)?;
            println!(
                "{}",
                serde_json::to_string(&InitPayload {
                    created_paths: summary.created_paths,
                })?
            );
        }
        Commands::Load {
            mode,
            scope_type,
            scope_id,
            intent,
            budget,
        } => {
            let mode = Mode::from_str(&mode)?;
            let scope_type = ScopeType::from_str(&scope_type)?;
            let scope = ScopeRef::new(scope_type, scope_id)?;
            let _intent = Intent::from_str(&intent)?;
            let loaded = load_startup_bundle(&config, &scope, mode, budget)?;

            let items = loaded
                .items
                .iter()
                .map(|item| LoadItemPayload {
                    item_key: &item.item_key,
                    text: &item.text,
                })
                .collect::<Vec<_>>();

            println!(
                "{}",
                serde_json::to_string(&LoadPayload {
                    mode: mode.as_str(),
                    scope: ScopePayload {
                        scope_type: scope.scope_type.as_str(),
                        id: &scope.scope_id,
                    },
                    items,
                    total_tokens_estimate: loaded.total_tokens_estimate,
                    degraded: loaded.degraded,
                })?
            );
        }
        Commands::WriteEvidence {
            scope_type,
            scope_id,
            session_id,
            source_kind,
            summary,
            promotable,
            origin_mode,
            claim_fingerprint,
        } => {
            let scope = ScopeRef::new(ScopeType::from_str(&scope_type)?, scope_id)?;
            let input = WriteEvidenceInput {
                scope,
                session_id,
                source_kind: SourceKind::from_str(&source_kind)?,
                summary,
                promotable: promotable == 1,
                origin_mode: Mode::from_str(&origin_mode)?,
                claim_fingerprint,
            };
            let written = write_evidence(&config, &input)?;
            println!("{}", serde_json::to_string(&written)?);
        }
        Commands::Search {
            scope_type,
            scope_id,
            intent,
            query,
            budget,
        } => {
            let scope = ScopeRef::new(ScopeType::from_str(&scope_type)?, scope_id)?;
            let intent = Intent::from_str(&intent)?;
            let response = search_memories(&config, &scope, intent, &query, budget)?;
            println!("{}", serde_json::to_string(&response)?);
        }
        Commands::Qmd { command } => match command {
            QmdCommands::Sync {
                scope_type,
                scope_id,
            } => {
                let scope = ScopeRef::new(ScopeType::from_str(&scope_type)?, scope_id)?;
                let synced = sync_scope(&config, &scope)?;
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({
                        "synced": true,
                        "records": synced,
                    }))?
                );
            }
        },
        Commands::Bundle { command } => match command {
            BundleCommands::Compile {
                scope_type,
                scope_id,
                budget,
            } => {
                let scope = ScopeRef::new(ScopeType::from_str(&scope_type)?, scope_id)?;
                let result = compile_scope_bundle(&config, &scope, budget)?;
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({
                        "compiled": true,
                        "items": result.items.len(),
                        "total_tokens_estimate": result.total_tokens_estimate,
                        "degraded": result.degraded,
                    }))?
                );
            }
        },
        Commands::Feedback {
            scope_type,
            scope_id,
            claim_fingerprint,
            verdict,
            reason,
            session_id,
        } => {
            let scope = ScopeRef::new(ScopeType::from_str(&scope_type)?, scope_id)?;
            let result = apply_feedback(
                &config,
                &scope,
                &claim_fingerprint,
                &verdict,
                &reason,
                session_id.as_deref(),
            )?;
            println!("{}", serde_json::to_string(&result)?);
        }
        Commands::Dream { command } => match command {
            DreamCommands::Run {
                scope_type,
                scope_id,
                trigger,
            } => {
                let scope = ScopeRef::new(ScopeType::from_str(&scope_type)?, scope_id)?;
                let result = run_dream(&config, &scope, &trigger)?;
                println!("{}", serde_json::to_string(&result)?);
            }
            DreamCommands::MaybeRun {
                scope_type,
                scope_id,
            } => {
                let scope = ScopeRef::new(ScopeType::from_str(&scope_type)?, scope_id)?;
                let result = maybe_run_scheduled_dream(&config, &scope)?;
                println!("{}", serde_json::to_string(&result)?);
            }
        },
        Commands::Repair {
            scope_type,
            scope_id,
        } => {
            let scope = match (scope_type, scope_id) {
                (Some(scope_type), Some(scope_id)) => {
                    Some(ScopeRef::new(ScopeType::from_str(&scope_type)?, scope_id)?)
                }
                _ => None,
            };
            let result = run_repair(&config, scope.as_ref())?;
            println!("{}", serde_json::to_string(&result)?);
        }
        Commands::Experiment { command } => match command {
            ExperimentCommands::Run { fixture } => {
                let fixture = fixture.unwrap_or_else(default_fixture_path);
                let result = run_fixture(&fixture)?;
                println!("{}", serde_json::to_string(&result)?);
            }
        },
        Commands::Hook { command } => match command {
            HookCommands::Capture {
                event,
                scope_type,
                scope_id,
                session_id,
                source_kind,
                summary,
                origin_mode,
                state_changed,
                promotable,
            } => {
                let event = HookEvent::from_str(&event).ok_or_else(|| Error::InvalidEnumValue {
                    kind: "hook_event",
                    value: event.clone(),
                })?;
                let scope = ScopeRef::new(ScopeType::from_str(&scope_type)?, scope_id)?;
                let result = capture_event(
                    &config,
                    &HookCaptureInput {
                        event,
                        scope,
                        session_id,
                        source_kind: SourceKind::from_str(&source_kind)?,
                        summary,
                        origin_mode: Mode::from_str(&origin_mode)?,
                        state_changed: state_changed == 1,
                        promotable: promotable == 1,
                    },
                )?;
                println!("{}", serde_json::to_string(&result)?);
            }
        },
    }

    Ok(())
}
