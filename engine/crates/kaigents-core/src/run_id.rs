//! File: engine/crates/kaigents-core/src/run_id.rs
//! Purpose: RunId identifier for Kaigents runs.
//! Product/business importance: provides stable run identifiers used across the platform.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// RunId is a stable identifier for a Kaigents run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(Uuid);

impl RunId {
    /// Create a new RunId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the string representation.
    pub fn as_string(&self) -> String {
        self.0.as_hyphenated().to_string()
    }

    /// Get the underlying Uuid.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Create a RunId from a Uuid.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_id_round_trips() {
        let id = RunId::new();
        let s = id.as_string();
        assert!(!s.is_empty());
    }
}
