//! File: engine/crates/kaigents-core/src/lib.rs
//! Purpose: Core domain primitives for the Kaigents execution engine.
//! Product/business importance: defines stable run identifiers used across the platform.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

/// RunId is a stable identifier for a Kaigents run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunId(pub String);

impl RunId {
    /// Creates a new RunId from a string-like value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the underlying run identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_id_round_trips() {
        let run_id = RunId::new("run-001");
        assert_eq!(run_id.as_str(), "run-001");
    }
}
