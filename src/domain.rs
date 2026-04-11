use std::str::FromStr;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeType {
    User,
    Project,
}

impl ScopeType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
        }
    }
}

impl FromStr for ScopeType {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "user" => Ok(Self::User),
            "project" => Ok(Self::Project),
            _ => Err(Error::InvalidEnumValue {
                kind: "scope_type",
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Fresh,
    Sterile,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Fresh => "fresh",
            Self::Sterile => "sterile",
        }
    }
}

impl FromStr for Mode {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "normal" => Ok(Self::Normal),
            "fresh" => Ok(Self::Fresh),
            "sterile" => Ok(Self::Sterile),
            _ => Err(Error::InvalidEnumValue {
                kind: "mode",
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Startup,
    Continue,
    KnowledgeLookup,
    Reset,
}

impl Intent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Continue => "continue",
            Self::KnowledgeLookup => "knowledge_lookup",
            Self::Reset => "reset",
        }
    }
}

impl FromStr for Intent {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "startup" => Ok(Self::Startup),
            "continue" => Ok(Self::Continue),
            "knowledge_lookup" => Ok(Self::KnowledgeLookup),
            "reset" => Ok(Self::Reset),
            _ => Err(Error::InvalidEnumValue {
                kind: "intent",
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    User,
    Tool,
    Code,
    Test,
    Decision,
    Feedback,
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Tool => "tool",
            Self::Code => "code",
            Self::Test => "test",
            Self::Decision => "decision",
            Self::Feedback => "feedback",
        }
    }
}

impl FromStr for SourceKind {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "user" => Ok(Self::User),
            "tool" => Ok(Self::Tool),
            "code" => Ok(Self::Code),
            "test" => Ok(Self::Test),
            "decision" => Ok(Self::Decision),
            "feedback" => Ok(Self::Feedback),
            _ => Err(Error::InvalidEnumValue {
                kind: "source_kind",
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeRef {
    pub scope_type: ScopeType,
    pub scope_id: String,
}

impl ScopeRef {
    pub fn new(scope_type: ScopeType, scope_id: impl Into<String>) -> Result<Self> {
        let scope_id = scope_id.into();
        if scope_id.trim().is_empty()
            || scope_id == "."
            || scope_id == ".."
            || scope_id.chars().any(std::path::is_separator)
        {
            return Err(Error::InvalidScopeId(scope_id));
        }

        Ok(Self {
            scope_type,
            scope_id,
        })
    }

    pub fn scope_key(&self) -> String {
        format!("{}:{}", self.scope_type.as_str(), self.scope_id)
    }

    pub fn scope_dir_fragment(&self) -> String {
        match self.scope_type {
            ScopeType::User => "user".to_string(),
            ScopeType::Project => format!("projects/{}", self.scope_id),
        }
    }
}
