//! File: engine/crates/kaigents-core/src/context_manager.rs
//! Purpose: Model-agnostic context budgeting and assembly.
//! Product/business importance: Ensures small-context models remain viable by owning the context window.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use crate::model_serving::ChatMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ContextBudgetStrategy {
    Summarize,
    #[default]
    Truncate,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FittedContext {
    pub messages: Vec<ChatMessage>,
    pub total_estimated_tokens: u32,
    pub dropped_entries_count: usize,
}

#[derive(Default)]
pub struct ContextManager {
    // Future: tokenizer interface, priority rules
}

impl ContextManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Assembles a prompt that fits within the specified token budget.
    /// Milestone 11 (v3): Selection-only + Episodes + Beliefs, supports Truncate strategy.
    #[allow(clippy::too_many_arguments)]
    pub fn fit_to_budget(
        &self,
        system_prompt: &str,
        task_state: &str,
        episodes: Vec<String>,
        case_file_entries: Vec<String>,
        beliefs: Vec<String>,
        budget: u32,
        _strategy: ContextBudgetStrategy,
    ) -> FittedContext {
        let mut messages = Vec::new();
        let mut current_tokens = 0;

        // 1. Always prioritize system prompt
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        });
        current_tokens += self.estimate_tokens(system_prompt);

        // 2. Add task state
        if current_tokens < budget {
            messages.push(ChatMessage {
                role: "user".to_string(),
                content: format!("Current task state: {}", task_state),
            });
            current_tokens += self.estimate_tokens(task_state) + 20; // overhead
        }

        // 2.5 Add Beliefs (Epistemic memory) - Precedence signals
        let mut dropped = 0;
        for belief in beliefs.iter().rev() {
            let belief_tokens = self.estimate_tokens(belief);
            if current_tokens + belief_tokens < budget {
                // Insert after task state
                messages.insert(
                    2,
                    ChatMessage {
                        role: "user".to_string(),
                        content: format!("Precedence/Belief: {}", belief),
                    },
                );
                current_tokens += belief_tokens + 20;
            } else {
                dropped += 1;
            }
        }

        // 3. Add Episodes (Long-term memory) - Most recent/relevant first
        for episode in episodes.iter().rev() {
            let episode_tokens = self.estimate_tokens(episode);
            if current_tokens + episode_tokens < budget {
                // Insert after beliefs
                let insert_pos = 2 + (beliefs.len() - dropped).min(beliefs.len());
                messages.insert(
                    insert_pos,
                    ChatMessage {
                        role: "user".to_string(),
                        content: format!("Recalled long-term context: {}", episode),
                    },
                );
                current_tokens += episode_tokens + 20;
            } else {
                dropped += 1;
            }
        }

        // 4. Add Case File entries (Short-term memory)
        for entry in case_file_entries.iter().rev() {
            let entry_tokens = self.estimate_tokens(entry);
            if current_tokens + entry_tokens < budget {
                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: format!("Context update: {}", entry),
                });
                current_tokens += entry_tokens + 20;
            } else {
                dropped += 1;
            }
        }

        FittedContext {
            messages,
            total_estimated_tokens: current_tokens,
            dropped_entries_count: dropped,
        }
    }

    /// Simple heuristic for token estimation (4 chars per token).
    /// In production, this would use a real tokenizer (BPE/Tiktoken).
    fn estimate_tokens(&self, text: &str) -> u32 {
        (text.len() as u32 / 4).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_tokens_minimum_one() {
        let cm = ContextManager::new();
        assert_eq!(cm.estimate_tokens(""), 1);
        assert_eq!(cm.estimate_tokens("a"), 1);
        assert_eq!(cm.estimate_tokens("ab"), 1);
        assert_eq!(cm.estimate_tokens("abc"), 1);
        assert_eq!(cm.estimate_tokens("abcd"), 1);
        assert_eq!(cm.estimate_tokens("abcde"), 1);
        assert_eq!(cm.estimate_tokens("abcdefgh"), 2);
    }

    #[test]
    fn fit_to_budget_large_budget_includes_everything() {
        let cm = ContextManager::new();
        let system_prompt = "You are a helpful assistant.";
        let task_state = "Writing an essay about AI.";
        let episodes = vec!["Past episode about ML.".to_string()];
        let case_file_entries = vec!["Source text about deep learning.".to_string()];
        let beliefs = vec!["Use a structured approach.".to_string()];

        let fitted = cm.fit_to_budget(
            system_prompt,
            task_state,
            episodes,
            case_file_entries,
            beliefs,
            100_000,
            ContextBudgetStrategy::Truncate,
        );

        assert_eq!(fitted.dropped_entries_count, 0);
        assert!(fitted.total_estimated_tokens < 100_000);
        assert_eq!(fitted.messages.len(), 5);
        assert_eq!(fitted.messages[0].role, "system");
        assert_eq!(fitted.messages[0].content, system_prompt);
    }

    #[test]
    fn fit_to_budget_tiny_budget_only_system_prompt() {
        let cm = ContextManager::new();
        let system_prompt = "You are a helpful assistant.";
        let task_state = "Writing a very long essay about AI that will not fit.";
        let episodes = vec!["Past episode about ML that is also long.".to_string()];
        let case_file_entries = vec!["Source text about deep learning.".to_string()];
        let beliefs = vec!["Use a structured approach for essays.".to_string()];

        let fitted = cm.fit_to_budget(
            system_prompt,
            task_state,
            episodes,
            case_file_entries,
            beliefs,
            2,
            ContextBudgetStrategy::Truncate,
        );

        assert!(fitted.dropped_entries_count > 0);
        assert_eq!(fitted.messages.len(), 1);
        assert_eq!(fitted.messages[0].role, "system");
        assert_eq!(fitted.messages[0].content, system_prompt);
    }

    #[test]
    fn fit_to_budget_system_prompt_always_present() {
        let cm = ContextManager::new();
        let system_prompt = "Critical system instructions here.";
        let fitted = cm.fit_to_budget(
            system_prompt,
            "task",
            vec!["episode".to_string()],
            vec!["case".to_string()],
            vec!["belief".to_string()],
            1,
            ContextBudgetStrategy::Truncate,
        );

        assert_eq!(fitted.messages.len(), 1);
        assert_eq!(fitted.messages[0].content, system_prompt);
    }

    #[test]
    fn fit_to_budget_beliefs_prioritized_before_episodes() {
        let cm = ContextManager::new();
        let system_prompt = "sys";
        let task_state = "task";
        let beliefs = vec!["important_belief".to_string()];
        let episodes = vec!["old_episode".to_string()];
        let case_file_entries = vec![];

        let fitted = cm.fit_to_budget(
            system_prompt,
            task_state,
            episodes,
            case_file_entries,
            beliefs,
            100,
            ContextBudgetStrategy::Truncate,
        );

        assert_eq!(fitted.dropped_entries_count, 0);
        assert!(fitted.messages.len() >= 4);
        let belief_content = fitted
            .messages
            .iter()
            .map(|m| &m.content)
            .find(|c| c.contains("important_belief"));
        assert!(
            belief_content.is_some(),
            "belief must be in the fitted context"
        );
        let episode_content = fitted
            .messages
            .iter()
            .map(|m| &m.content)
            .find(|c| c.contains("old_episode"));
        assert!(
            episode_content.is_some(),
            "episode must be in the fitted context"
        );

        let belief_idx = fitted
            .messages
            .iter()
            .position(|m| m.content.contains("important_belief"))
            .unwrap();
        let episode_idx = fitted
            .messages
            .iter()
            .position(|m| m.content.contains("old_episode"))
            .unwrap();
        assert!(
            belief_idx < episode_idx,
            "belief must appear before episode in context"
        );
    }

    #[test]
    fn fit_to_budget_drops_entries_when_budget_exceeded() {
        let cm = ContextManager::new();
        let system_prompt = "sys";
        let task_state = "task";
        let episodes = vec![
            "episode_one_with_some_length".to_string(),
            "episode_two_with_some_length".to_string(),
            "episode_three_with_some_length".to_string(),
        ];
        let case_file_entries = vec![
            "case_file_entry_one_long_enough".to_string(),
            "case_file_entry_two_long_enough".to_string(),
        ];
        let beliefs = vec![];

        let fitted = cm.fit_to_budget(
            system_prompt,
            task_state,
            episodes,
            case_file_entries,
            beliefs,
            50,
            ContextBudgetStrategy::Truncate,
        );

        assert!(
            fitted.dropped_entries_count > 0,
            "entries should be dropped with a small budget"
        );
        assert!(
            fitted.total_estimated_tokens < 50,
            "total tokens must stay under budget"
        );
    }

    #[test]
    fn fit_to_budget_empty_inputs() {
        let cm = ContextManager::new();
        let fitted = cm.fit_to_budget(
            "system",
            "task",
            vec![],
            vec![],
            vec![],
            1000,
            ContextBudgetStrategy::Truncate,
        );

        assert_eq!(fitted.dropped_entries_count, 0);
        assert_eq!(fitted.messages.len(), 2);
        assert_eq!(fitted.messages[0].role, "system");
        assert_eq!(fitted.messages[1].role, "user");
    }
}
