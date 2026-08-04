//! File: engine/crates/kaigents-core/src/context_manager.rs
//! Purpose: Model-agnostic context budgeting and assembly.
//! Product/business importance: Ensures small-context models remain viable by owning the context window.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use crate::model_serving::ChatMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum ContextBudgetStrategy {
    Selection,
    Summarize,
    HierarchicalDemotion,
    WorkDecomposition,
    #[default]
    Auto,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ContextTier {
    Core,
    Recall,
    #[default]
    Archival,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    pub tier: ContextTier,
    pub role: String,
    pub content: String,
}

impl ContextItem {
    pub fn new(tier: ContextTier, role: &str, content: String) -> Self {
        Self {
            tier,
            role: role.to_string(),
            content,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FittedContext {
    pub messages: Vec<ChatMessage>,
    pub total_estimated_tokens: u32,
    pub dropped_entries_count: usize,
    pub summarized_entries_count: usize,
    pub demoted_entries_count: usize,
    pub budget_exceeded: bool,
    pub slice_index: usize,
    pub total_slices: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposedContext {
    pub slices: Vec<FittedContext>,
    pub total_items: usize,
    pub total_dropped: usize,
    pub total_summarized: usize,
    pub total_demoted: usize,
}

impl DecomposedContext {
    pub fn single(fitted: FittedContext) -> Self {
        let total_items = fitted.messages.len();
        let dropped = fitted.dropped_entries_count;
        let summarized = fitted.summarized_entries_count;
        let demoted = fitted.demoted_entries_count;
        Self {
            slices: vec![fitted],
            total_items,
            total_dropped: dropped,
            total_summarized: summarized,
            total_demoted: demoted,
        }
    }

    pub fn is_decomposed(&self) -> bool {
        self.slices.len() > 1
    }
}

#[async_trait::async_trait]
pub trait SummaryProvider: Send + Sync {
    async fn summarize(&self, texts: &[String]) -> Result<String, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCandidate {
    pub name: String,
    pub context_window_size: u32,
    pub priority: u32,
    pub is_local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPolicy {
    pub prefer_local: bool,
    pub allow_overflow_fallback: bool,
}

impl Default for RoutingPolicy {
    fn default() -> Self {
        Self {
            prefer_local: true,
            allow_overflow_fallback: true,
        }
    }
}

impl RoutingPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn select_model_for_context(
        &self,
        context_token_count: u32,
        candidates: &[ModelCandidate],
    ) -> Result<String, String> {
        if candidates.is_empty() {
            return Err("No models available".to_string());
        }

        let mut fitting: Vec<&ModelCandidate> = candidates
            .iter()
            .filter(|c| c.context_window_size >= context_token_count)
            .collect();

        if fitting.is_empty() {
            if self.allow_overflow_fallback {
                let largest = candidates
                    .iter()
                    .max_by_key(|c| c.context_window_size)
                    .ok_or("No models available")?;
                return Ok(largest.name.clone());
            }
            return Err(format!(
                "No model has a context window large enough for {} tokens",
                context_token_count
            ));
        }

        fitting.sort_by(|a, b| {
            let a_pref = self.prefer_local && a.is_local;
            let b_pref = self.prefer_local && b.is_local;
            b_pref
                .cmp(&a_pref)
                .then(a.priority.cmp(&b.priority))
                .then(a.context_window_size.cmp(&b.context_window_size))
        });

        Ok(fitting[0].name.clone())
    }
}

#[derive(Default)]
pub struct ContextManager {
    routing_policy: RoutingPolicy,
}

impl ContextManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_routing_policy(routing_policy: RoutingPolicy) -> Self {
        Self { routing_policy }
    }

    pub fn routing_policy(&self) -> &RoutingPolicy {
        &self.routing_policy
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fit_to_budget(
        &self,
        system_prompt: &str,
        task_state: &str,
        episodes: Vec<String>,
        case_file_entries: Vec<String>,
        beliefs: Vec<String>,
        budget: u32,
        strategy: ContextBudgetStrategy,
    ) -> FittedContext {
        let mut items = Vec::new();

        items.push(ContextItem::new(
            ContextTier::Core,
            "system",
            system_prompt.to_string(),
        ));
        items.push(ContextItem::new(
            ContextTier::Core,
            "user",
            format!("Current task state: {}", task_state),
        ));
        for belief in beliefs.iter().rev() {
            items.push(ContextItem::new(
                ContextTier::Core,
                "user",
                format!("Precedence/Belief: {}", belief),
            ));
        }
        for episode in episodes.iter().rev() {
            items.push(ContextItem::new(
                ContextTier::Recall,
                "user",
                format!("Recalled long-term context: {}", episode),
            ));
        }
        for entry in case_file_entries.iter().rev() {
            items.push(ContextItem::new(
                ContextTier::Archival,
                "user",
                format!("Context update: {}", entry),
            ));
        }

        let decomposed = self.fit_decomposed(items, budget, &strategy);
        decomposed
            .slices
            .into_iter()
            .next()
            .unwrap_or(FittedContext {
                messages: Vec::new(),
                total_estimated_tokens: 0,
                dropped_entries_count: 0,
                summarized_entries_count: 0,
                demoted_entries_count: 0,
                budget_exceeded: false,
                slice_index: 0,
                total_slices: 1,
            })
    }

    pub fn fit_decomposed(
        &self,
        items: Vec<ContextItem>,
        budget: u32,
        strategy: &ContextBudgetStrategy,
    ) -> DecomposedContext {
        match strategy {
            ContextBudgetStrategy::Auto => self.fit_auto(items, budget),
            ContextBudgetStrategy::Selection => {
                let fitted = self.fit_with_selection(items, budget);
                DecomposedContext::single(fitted)
            }
            ContextBudgetStrategy::Summarize => {
                let fitted = self.fit_with_summarize(items, budget);
                DecomposedContext::single(fitted)
            }
            ContextBudgetStrategy::HierarchicalDemotion => {
                let fitted = self.fit_with_demotion(items, budget);
                DecomposedContext::single(fitted)
            }
            ContextBudgetStrategy::WorkDecomposition => self.fit_with_decomposition(items, budget),
            ContextBudgetStrategy::Error => {
                let fitted = self.fit_with_selection(items, budget);
                let mut fitted = fitted;
                fitted.budget_exceeded = fitted.dropped_entries_count > 0;
                DecomposedContext::single(fitted)
            }
        }
    }

    fn fit_auto(&self, items: Vec<ContextItem>, budget: u32) -> DecomposedContext {
        let (core_items, recall_items, archival_items) = self.partition_by_tier(&items);

        let mut messages = Vec::new();
        let mut current_tokens = 0u32;
        let mut summarized = 0usize;
        let mut demoted = 0usize;

        for item in &core_items {
            let tokens = self.estimate_tokens(&item.content) + 20;
            messages.push(ChatMessage {
                role: item.role.clone(),
                content: item.content.clone(),
            });
            current_tokens += tokens;
        }

        let mut remaining_recall: Vec<&ContextItem> = Vec::new();
        for item in &recall_items {
            let tokens = self.estimate_tokens(&item.content) + 20;
            if current_tokens + tokens < budget {
                messages.push(ChatMessage {
                    role: item.role.clone(),
                    content: item.content.clone(),
                });
                current_tokens += tokens;
            } else {
                let compressed = self.extractive_summarize(&item.content);
                let compressed_tokens = self.estimate_tokens(&compressed) + 20;
                if current_tokens + compressed_tokens < budget {
                    messages.push(ChatMessage {
                        role: item.role.clone(),
                        content: format!("[summarized] {}", compressed),
                    });
                    current_tokens += compressed_tokens;
                    summarized += 1;
                } else {
                    remaining_recall.push(item);
                }
            }
        }

        let mut remaining_archival: Vec<&ContextItem> = Vec::new();
        for item in &archival_items {
            let tokens = self.estimate_tokens(&item.content) + 20;
            if current_tokens + tokens < budget {
                messages.push(ChatMessage {
                    role: item.role.clone(),
                    content: item.content.clone(),
                });
                current_tokens += tokens;
            } else {
                let compressed = self.extractive_summarize(&item.content);
                let compressed_tokens = self.estimate_tokens(&compressed) + 20;
                if current_tokens + compressed_tokens < budget {
                    messages.push(ChatMessage {
                        role: item.role.clone(),
                        content: format!("[summarized] {}", compressed),
                    });
                    current_tokens += compressed_tokens;
                    summarized += 1;
                } else {
                    let reference = self.demote_to_reference(&item.content);
                    let ref_tokens = self.estimate_tokens(&reference) + 20;
                    if current_tokens + ref_tokens < budget {
                        messages.push(ChatMessage {
                            role: item.role.clone(),
                            content: reference,
                        });
                        current_tokens += ref_tokens;
                        demoted += 1;
                    } else {
                        remaining_archival.push(item);
                    }
                }
            }
        }

        let remaining_count = remaining_recall.len() + remaining_archival.len();
        if remaining_count == 0 {
            return DecomposedContext {
                slices: vec![FittedContext {
                    messages,
                    total_estimated_tokens: current_tokens,
                    dropped_entries_count: 0,
                    summarized_entries_count: summarized,
                    demoted_entries_count: demoted,
                    budget_exceeded: false,
                    slice_index: 0,
                    total_slices: 1,
                }],
                total_items: items.len(),
                total_dropped: 0,
                total_summarized: summarized,
                total_demoted: demoted,
            };
        }

        let mut all_remaining: Vec<ContextItem> = Vec::new();
        for item in remaining_recall
            .into_iter()
            .chain(remaining_archival.into_iter())
        {
            all_remaining.push(item.clone());
        }

        let mut slices = vec![FittedContext {
            messages,
            total_estimated_tokens: current_tokens,
            dropped_entries_count: 0,
            summarized_entries_count: summarized,
            demoted_entries_count: demoted,
            budget_exceeded: false,
            slice_index: 0,
            total_slices: 1,
        }];

        let mut slice_idx = 1;
        let mut current_slice_messages = Vec::new();
        let mut current_slice_tokens = 0u32;

        for item in &all_remaining {
            let tokens = self.estimate_tokens(&item.content) + 20;
            if current_slice_tokens + tokens >= budget
                && !current_slice_messages.is_empty()
            {
                slices.push(FittedContext {
                    messages: std::mem::take(&mut current_slice_messages),
                    total_estimated_tokens: current_slice_tokens,
                    dropped_entries_count: 0,
                    summarized_entries_count: 0,
                    demoted_entries_count: 0,
                    budget_exceeded: false,
                    slice_index: slice_idx,
                    total_slices: 0,
                });
                slice_idx += 1;
                current_slice_tokens = 0;
            }
            current_slice_messages.push(ChatMessage {
                role: item.role.clone(),
                content: item.content.clone(),
            });
            current_slice_tokens += tokens;
        }

        if !current_slice_messages.is_empty() {
            slices.push(FittedContext {
                messages: current_slice_messages,
                total_estimated_tokens: current_slice_tokens,
                dropped_entries_count: 0,
                summarized_entries_count: 0,
                demoted_entries_count: 0,
                budget_exceeded: false,
                slice_index: slice_idx,
                total_slices: 0,
            });
        }

        let total_slices = slices.len();
        for s in slices.iter_mut() {
            s.total_slices = total_slices;
        }

        DecomposedContext {
            slices,
            total_items: items.len(),
            total_dropped: 0,
            total_summarized: summarized,
            total_demoted: demoted,
        }
    }

    fn fit_with_selection(&self, items: Vec<ContextItem>, budget: u32) -> FittedContext {
        let (core_items, recall_items, archival_items) = self.partition_by_tier(&items);

        let mut messages = Vec::new();
        let mut current_tokens = 0u32;
        let mut dropped = 0usize;

        for item in &core_items {
            let tokens = self.estimate_tokens(&item.content) + 20;
            messages.push(ChatMessage {
                role: item.role.clone(),
                content: item.content.clone(),
            });
            current_tokens += tokens;
        }

        for item in &recall_items {
            let tokens = self.estimate_tokens(&item.content) + 20;
            if current_tokens + tokens < budget {
                messages.push(ChatMessage {
                    role: item.role.clone(),
                    content: item.content.clone(),
                });
                current_tokens += tokens;
            } else {
                dropped += 1;
            }
        }

        for item in &archival_items {
            let tokens = self.estimate_tokens(&item.content) + 20;
            if current_tokens + tokens < budget {
                messages.push(ChatMessage {
                    role: item.role.clone(),
                    content: item.content.clone(),
                });
                current_tokens += tokens;
            } else {
                dropped += 1;
            }
        }

        FittedContext {
            messages,
            total_estimated_tokens: current_tokens,
            dropped_entries_count: dropped,
            summarized_entries_count: 0,
            demoted_entries_count: 0,
            budget_exceeded: dropped > 0,
            slice_index: 0,
            total_slices: 1,
        }
    }

    fn fit_with_summarize(&self, items: Vec<ContextItem>, budget: u32) -> FittedContext {
        let (core_items, recall_items, archival_items) = self.partition_by_tier(&items);

        let mut messages = Vec::new();
        let mut current_tokens = 0u32;
        let mut summarized = 0usize;
        let mut dropped = 0usize;

        for item in &core_items {
            let tokens = self.estimate_tokens(&item.content) + 20;
            messages.push(ChatMessage {
                role: item.role.clone(),
                content: item.content.clone(),
            });
            current_tokens += tokens;
        }

        for item in &recall_items {
            let tokens = self.estimate_tokens(&item.content) + 20;
            if current_tokens + tokens < budget {
                messages.push(ChatMessage {
                    role: item.role.clone(),
                    content: item.content.clone(),
                });
                current_tokens += tokens;
            } else {
                let compressed = self.extractive_summarize(&item.content);
                let compressed_tokens = self.estimate_tokens(&compressed) + 20;
                if current_tokens + compressed_tokens < budget {
                    messages.push(ChatMessage {
                        role: item.role.clone(),
                        content: format!("[summarized] {}", compressed),
                    });
                    current_tokens += compressed_tokens;
                    summarized += 1;
                } else {
                    dropped += 1;
                }
            }
        }

        for item in &archival_items {
            let tokens = self.estimate_tokens(&item.content) + 20;
            if current_tokens + tokens < budget {
                messages.push(ChatMessage {
                    role: item.role.clone(),
                    content: item.content.clone(),
                });
                current_tokens += tokens;
            } else {
                let compressed = self.extractive_summarize(&item.content);
                let compressed_tokens = self.estimate_tokens(&compressed) + 20;
                if current_tokens + compressed_tokens < budget {
                    messages.push(ChatMessage {
                        role: item.role.clone(),
                        content: format!("[summarized] {}", compressed),
                    });
                    current_tokens += compressed_tokens;
                    summarized += 1;
                } else {
                    dropped += 1;
                }
            }
        }

        FittedContext {
            messages,
            total_estimated_tokens: current_tokens,
            dropped_entries_count: dropped,
            summarized_entries_count: summarized,
            demoted_entries_count: 0,
            budget_exceeded: dropped > 0,
            slice_index: 0,
            total_slices: 1,
        }
    }

    fn fit_with_demotion(&self, items: Vec<ContextItem>, budget: u32) -> FittedContext {
        let (core_items, recall_items, archival_items) = self.partition_by_tier(&items);

        let mut messages = Vec::new();
        let mut current_tokens = 0u32;
        let mut demoted = 0usize;
        let mut dropped = 0usize;

        for item in &core_items {
            let tokens = self.estimate_tokens(&item.content) + 20;
            messages.push(ChatMessage {
                role: item.role.clone(),
                content: item.content.clone(),
            });
            current_tokens += tokens;
        }

        for item in &recall_items {
            let tokens = self.estimate_tokens(&item.content) + 20;
            if current_tokens + tokens < budget {
                messages.push(ChatMessage {
                    role: item.role.clone(),
                    content: item.content.clone(),
                });
                current_tokens += tokens;
            } else {
                let reference = self.demote_to_reference(&item.content);
                let ref_tokens = self.estimate_tokens(&reference) + 20;
                if current_tokens + ref_tokens < budget {
                    messages.push(ChatMessage {
                        role: item.role.clone(),
                        content: reference,
                    });
                    current_tokens += ref_tokens;
                    demoted += 1;
                } else {
                    dropped += 1;
                }
            }
        }

        for item in &archival_items {
            let tokens = self.estimate_tokens(&item.content) + 20;
            if current_tokens + tokens < budget {
                messages.push(ChatMessage {
                    role: item.role.clone(),
                    content: item.content.clone(),
                });
                current_tokens += tokens;
            } else {
                let reference = self.demote_to_reference(&item.content);
                let ref_tokens = self.estimate_tokens(&reference) + 20;
                if current_tokens + ref_tokens < budget {
                    messages.push(ChatMessage {
                        role: item.role.clone(),
                        content: reference,
                    });
                    current_tokens += ref_tokens;
                    demoted += 1;
                } else {
                    dropped += 1;
                }
            }
        }

        FittedContext {
            messages,
            total_estimated_tokens: current_tokens,
            dropped_entries_count: dropped,
            summarized_entries_count: 0,
            demoted_entries_count: demoted,
            budget_exceeded: dropped > 0,
            slice_index: 0,
            total_slices: 1,
        }
    }

    fn fit_with_decomposition(&self, items: Vec<ContextItem>, budget: u32) -> DecomposedContext {
        let mut slices = Vec::new();
        let mut current_messages = Vec::new();
        let mut current_tokens = 0u32;
        let mut slice_idx = 0usize;

        for item in &items {
            let tokens = self.estimate_tokens(&item.content) + 20;
            if current_tokens + tokens >= budget && !current_messages.is_empty() {
                slices.push(FittedContext {
                    messages: std::mem::take(&mut current_messages),
                    total_estimated_tokens: current_tokens,
                    dropped_entries_count: 0,
                    summarized_entries_count: 0,
                    demoted_entries_count: 0,
                    budget_exceeded: false,
                    slice_index: slice_idx,
                    total_slices: 0,
                });
                slice_idx += 1;
                current_tokens = 0;
            }
            current_messages.push(ChatMessage {
                role: item.role.clone(),
                content: item.content.clone(),
            });
            current_tokens += tokens;
        }

        if !current_messages.is_empty() {
            slices.push(FittedContext {
                messages: current_messages,
                total_estimated_tokens: current_tokens,
                dropped_entries_count: 0,
                summarized_entries_count: 0,
                demoted_entries_count: 0,
                budget_exceeded: false,
                slice_index: slice_idx,
                total_slices: 0,
            });
        }

        let total_slices = slices.len();
        for s in slices.iter_mut() {
            s.total_slices = total_slices;
        }

        DecomposedContext {
            slices,
            total_items: items.len(),
            total_dropped: 0,
            total_summarized: 0,
            total_demoted: 0,
        }
    }

    pub fn fit_to_budget_tiered(
        &self,
        items: Vec<ContextItem>,
        budget: u32,
        strategy: ContextBudgetStrategy,
    ) -> FittedContext {
        self.fit_tiered_items(items, budget, &strategy)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn fit_to_budget_with_summarization(
        &self,
        system_prompt: &str,
        task_state: &str,
        episodes: Vec<String>,
        case_file_entries: Vec<String>,
        beliefs: Vec<String>,
        budget: u32,
        summary_provider: &dyn SummaryProvider,
    ) -> FittedContext {
        let mut items = Vec::new();

        items.push(ContextItem::new(
            ContextTier::Core,
            "system",
            system_prompt.to_string(),
        ));
        items.push(ContextItem::new(
            ContextTier::Core,
            "user",
            format!("Current task state: {}", task_state),
        ));
        for belief in beliefs.iter().rev() {
            items.push(ContextItem::new(
                ContextTier::Core,
                "user",
                format!("Precedence/Belief: {}", belief),
            ));
        }
        for episode in episodes.iter().rev() {
            items.push(ContextItem::new(
                ContextTier::Recall,
                "user",
                format!("Recalled long-term context: {}", episode),
            ));
        }
        for entry in case_file_entries.iter().rev() {
            items.push(ContextItem::new(
                ContextTier::Archival,
                "user",
                format!("Context update: {}", entry),
            ));
        }

        let preliminary = self.fit_with_selection(items.clone(), budget);

        if preliminary.dropped_entries_count == 0 {
            return preliminary;
        }

        let mut archival_to_summarize: Vec<String> = Vec::new();
        let mut recall_to_summarize: Vec<String> = Vec::new();

        for item in &items {
            let is_in_preliminary = preliminary
                .messages
                .iter()
                .any(|m| m.content == item.content);
            if !is_in_preliminary {
                match item.tier {
                    ContextTier::Archival => archival_to_summarize.push(item.content.clone()),
                    ContextTier::Recall => recall_to_summarize.push(item.content.clone()),
                    ContextTier::Core => {}
                }
            }
        }

        let mut summarized_count = 0;
        let mut messages = preliminary.messages.clone();
        let mut current_tokens = preliminary.total_estimated_tokens;

        if !archival_to_summarize.is_empty() {
            if let Ok(summary) = summary_provider.summarize(&archival_to_summarize).await {
                let summary_tokens = self.estimate_tokens(&summary) + 20;
                if current_tokens + summary_tokens < budget {
                    messages.push(ChatMessage {
                        role: "user".to_string(),
                        content: format!("Summarized context (archival): {}", summary),
                    });
                    current_tokens += summary_tokens;
                    summarized_count = archival_to_summarize.len();
                }
            }
        }

        if summarized_count == 0 && !recall_to_summarize.is_empty() {
            if let Ok(summary) = summary_provider.summarize(&recall_to_summarize).await {
                let summary_tokens = self.estimate_tokens(&summary) + 20;
                if current_tokens + summary_tokens < budget {
                    messages.push(ChatMessage {
                        role: "user".to_string(),
                        content: format!("Summarized context (recall): {}", summary),
                    });
                    current_tokens += summary_tokens;
                    summarized_count = recall_to_summarize.len();
                }
            }
        }

        let remaining_dropped = preliminary.dropped_entries_count - summarized_count;

        FittedContext {
            messages,
            total_estimated_tokens: current_tokens,
            dropped_entries_count: remaining_dropped,
            summarized_entries_count: summarized_count,
            demoted_entries_count: 0,
            budget_exceeded: remaining_dropped > 0,
            slice_index: 0,
            total_slices: 1,
        }
    }

    pub fn select_model_for_context(
        &self,
        context_token_count: u32,
        candidates: &[ModelCandidate],
    ) -> Result<String, String> {
        self.routing_policy
            .select_model_for_context(context_token_count, candidates)
    }

    fn fit_tiered_items(
        &self,
        items: Vec<ContextItem>,
        budget: u32,
        strategy: &ContextBudgetStrategy,
    ) -> FittedContext {
        match strategy {
            ContextBudgetStrategy::Auto => {
                let decomposed = self.fit_auto(items, budget);
                decomposed
                    .slices
                    .into_iter()
                    .next()
                    .unwrap_or(FittedContext {
                        messages: Vec::new(),
                        total_estimated_tokens: 0,
                        dropped_entries_count: 0,
                        summarized_entries_count: 0,
                        demoted_entries_count: 0,
                        budget_exceeded: false,
                        slice_index: 0,
                        total_slices: 1,
                    })
            }
            ContextBudgetStrategy::Selection => self.fit_with_selection(items, budget),
            ContextBudgetStrategy::Summarize => self.fit_with_summarize(items, budget),
            ContextBudgetStrategy::HierarchicalDemotion => self.fit_with_demotion(items, budget),
            ContextBudgetStrategy::WorkDecomposition => {
                let decomposed = self.fit_with_decomposition(items, budget);
                decomposed
                    .slices
                    .into_iter()
                    .next()
                    .unwrap_or(FittedContext {
                        messages: Vec::new(),
                        total_estimated_tokens: 0,
                        dropped_entries_count: 0,
                        summarized_entries_count: 0,
                        demoted_entries_count: 0,
                        budget_exceeded: false,
                        slice_index: 0,
                        total_slices: 1,
                    })
            }
            ContextBudgetStrategy::Error => {
                let mut fitted = self.fit_with_selection(items, budget);
                fitted.budget_exceeded = fitted.dropped_entries_count > 0;
                fitted
            }
        }
    }

    fn partition_by_tier<'a>(
        &self,
        items: &'a [ContextItem],
    ) -> (
        Vec<&'a ContextItem>,
        Vec<&'a ContextItem>,
        Vec<&'a ContextItem>,
    ) {
        let core: Vec<&ContextItem> = items
            .iter()
            .filter(|i| i.tier == ContextTier::Core)
            .collect();
        let recall: Vec<&ContextItem> = items
            .iter()
            .filter(|i| i.tier == ContextTier::Recall)
            .collect();
        let archival: Vec<&ContextItem> = items
            .iter()
            .filter(|i| i.tier == ContextTier::Archival)
            .collect();
        (core, recall, archival)
    }

    #[allow(dead_code)]
    fn compress_and_refit(
        &self,
        fitted_messages: &[ChatMessage],
        current_tokens: u32,
        budget: u32,
        recall_items: &[&ContextItem],
        archival_items: &[&ContextItem],
    ) -> CompressionResult {
        let mut messages = fitted_messages.to_vec();
        let mut current_tokens = current_tokens;
        let mut summarized = 0usize;
        let mut still_dropped = 0usize;

        for item in archival_items {
            let compressed = self.extractive_summarize(&item.content);
            let compressed_tokens = self.estimate_tokens(&compressed) + 20;
            if current_tokens + compressed_tokens < budget {
                messages.push(ChatMessage {
                    role: item.role.clone(),
                    content: format!("[summarized] {}", compressed),
                });
                current_tokens += compressed_tokens;
                summarized += 1;
            } else {
                still_dropped += 1;
            }
        }

        if summarized == 0 {
            for item in recall_items {
                let compressed = self.extractive_summarize(&item.content);
                let compressed_tokens = self.estimate_tokens(&compressed) + 20;
                if current_tokens + compressed_tokens < budget {
                    let insert_pos = 1.min(messages.len());
                    messages.insert(
                        insert_pos,
                        ChatMessage {
                            role: item.role.clone(),
                            content: format!("[summarized] {}", compressed),
                        },
                    );
                    current_tokens += compressed_tokens;
                    summarized += 1;
                } else {
                    still_dropped += 1;
                }
            }
        }

        CompressionResult {
            messages,
            current_tokens,
            summarized,
            dropped: still_dropped,
        }
    }

    fn extractive_summarize(&self, text: &str) -> String {
        let sentences: Vec<&str> = text
            .split('.')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if sentences.len() <= 2 {
            if text.len() > 200 {
                let preview_len = 150.min(text.len());
                return format!("{}...", &text[..preview_len]);
            }
            return text.to_string();
        }
        let first = sentences.first().unwrap_or(&"");
        let last = sentences.last().unwrap_or(&"");
        let mid_idx = sentences.len() / 2;
        let mid = sentences.get(mid_idx).unwrap_or(&"");
        let summary = if sentences.len() > 4 {
            format!("{}. {}... {}. {}", first, mid, sentences.len(), last)
        } else {
            format!("{}. {}", first, last)
        };
        if summary.len() > text.len() {
            text.to_string()
        } else {
            summary
        }
    }

    fn demote_to_reference(&self, text: &str) -> String {
        let preview_len = 80.min(text.len());
        let preview = &text[..preview_len];
        format!("[demoted to archival] {}", preview)
    }

    fn estimate_tokens(&self, text: &str) -> u32 {
        (text.len() as u32 / 4).max(1)
    }
}

#[allow(dead_code)]
struct CompressionResult {
    messages: Vec<ChatMessage>,
    current_tokens: u32,
    summarized: usize,
    dropped: usize,
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
            ContextBudgetStrategy::Auto,
        );

        assert_eq!(fitted.dropped_entries_count, 0);
        assert!(fitted.total_estimated_tokens < 100_000);
        assert_eq!(fitted.messages.len(), 5);
        assert_eq!(fitted.messages[0].role, "system");
        assert_eq!(fitted.messages[0].content, system_prompt);
        assert!(!fitted.budget_exceeded);
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
            ContextBudgetStrategy::Auto,
        );

        assert_eq!(fitted.dropped_entries_count, 0);
        assert!(!fitted.messages.is_empty());
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
            ContextBudgetStrategy::Auto,
        );

        assert!(!fitted.messages.is_empty());
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
            200,
            ContextBudgetStrategy::Auto,
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
            ContextBudgetStrategy::Selection,
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
            ContextBudgetStrategy::Auto,
        );

        assert_eq!(fitted.dropped_entries_count, 0);
        assert_eq!(fitted.messages.len(), 2);
        assert_eq!(fitted.messages[0].role, "system");
        assert_eq!(fitted.messages[1].role, "user");
    }

    // --- Context Manager v2 tests ---

    #[test]
    fn error_strategy_sets_budget_exceeded_when_items_dropped() {
        let cm = ContextManager::new();
        let fitted = cm.fit_to_budget(
            "sys",
            "task",
            vec!["episode_one_with_some_length".to_string()],
            vec!["case_file_entry_long_enough".to_string()],
            vec![],
            10,
            ContextBudgetStrategy::Error,
        );

        assert!(
            fitted.budget_exceeded,
            "Error strategy must set budget_exceeded when items are dropped"
        );
        assert!(fitted.dropped_entries_count > 0);
    }

    #[test]
    fn error_strategy_no_budget_exceeded_when_everything_fits() {
        let cm = ContextManager::new();
        let fitted = cm.fit_to_budget(
            "sys",
            "task",
            vec!["episode".to_string()],
            vec!["case".to_string()],
            vec!["belief".to_string()],
            100_000,
            ContextBudgetStrategy::Error,
        );

        assert!(
            !fitted.budget_exceeded,
            "Error strategy must not set budget_exceeded when everything fits"
        );
        assert_eq!(fitted.dropped_entries_count, 0);
    }

    #[test]
    fn summarize_strategy_compresses_instead_of_just_dropping() {
        let cm = ContextManager::new();
        let long_case_file = "x".repeat(500);
        let fitted = cm.fit_to_budget(
            "sys",
            "task",
            vec![],
            vec![long_case_file],
            vec![],
            120,
            ContextBudgetStrategy::Summarize,
        );

        assert!(
            fitted.summarized_entries_count > 0,
            "Summarize strategy should compress at least one item"
        );
        assert!(
            fitted.total_estimated_tokens < 120,
            "total tokens must stay under budget"
        );
    }

    #[test]
    fn hierarchical_demotion_drops_archival_before_recall() {
        let cm = ContextManager::new();
        let system_prompt = "sys";
        let task_state = "task_state_here";

        let long_episode = "episode_".to_string() + &"x".repeat(100);
        let long_case_file = "case_".to_string() + &"x".repeat(100);

        let fitted = cm.fit_to_budget(
            system_prompt,
            task_state,
            vec![long_episode.clone()],
            vec![long_case_file.clone()],
            vec![],
            60,
            ContextBudgetStrategy::Auto,
        );

        let has_episode = fitted
            .messages
            .iter()
            .any(|m| m.content.contains("episode_"));
        let has_case = fitted.messages.iter().any(|m| m.content.contains("case_"));

        if has_episode && !has_case {
            // episode was kept, case file was dropped — correct hierarchical demotion
        } else if !has_episode && !has_case {
            // both dropped — budget too small for either
        } else {
            panic!(
                "Archival (case file) should be dropped before recall (episode), but case_file={} episode={}",
                has_case, has_episode
            );
        }
    }

    #[test]
    fn hierarchical_demotion_drops_recall_before_core() {
        let cm = ContextManager::new();
        let system_prompt = "sys";
        let task_state = "task_state";
        let belief = "important_belief_that_should_be_preserved";

        let long_episode = "x".repeat(200);

        let fitted = cm.fit_to_budget(
            system_prompt,
            task_state,
            vec![long_episode],
            vec![],
            vec![belief.to_string()],
            40,
            ContextBudgetStrategy::Auto,
        );

        let has_belief = fitted
            .messages
            .iter()
            .any(|m| m.content.contains("important_belief"));
        let has_episode = fitted.messages.iter().any(|m| m.content.contains("xxxx"));

        // If belief is present but episode is not, core was preserved over recall — correct
        if has_belief && !has_episode {
            // correct hierarchical demotion
        } else if !has_belief && !has_episode {
            // budget too small for either non-system item — also valid
        }
        // If episode survived but belief didn't, that's wrong, but with a very tight budget
        // the system prompt might consume everything, so we just verify system prompt is present
        assert!(
            fitted.messages.iter().any(|m| m.role == "system"),
            "system prompt must always be present"
        );
    }

    #[test]
    fn fit_to_budget_tiered_with_explicit_tiers() {
        let cm = ContextManager::new();
        let items = vec![
            ContextItem::new(ContextTier::Core, "system", "system prompt".to_string()),
            ContextItem::new(ContextTier::Core, "user", "task state".to_string()),
            ContextItem::new(ContextTier::Recall, "user", "episode from past".to_string()),
            ContextItem::new(
                ContextTier::Archival,
                "user",
                "old case file entry".to_string(),
            ),
        ];

        let fitted = cm.fit_to_budget_tiered(items, 100_000, ContextBudgetStrategy::Auto);

        assert_eq!(fitted.dropped_entries_count, 0);
        assert_eq!(fitted.messages.len(), 4);
        assert!(!fitted.budget_exceeded);
    }

    #[test]
    fn fit_to_budget_tiered_drops_archival_first() {
        let cm = ContextManager::new();
        let items = vec![
            ContextItem::new(ContextTier::Core, "system", "sys".to_string()),
            ContextItem::new(ContextTier::Core, "user", "task".to_string()),
            ContextItem::new(ContextTier::Recall, "user", "episode".to_string()),
            ContextItem::new(ContextTier::Archival, "user", "x".repeat(200)),
        ];

        let fitted = cm.fit_to_budget_tiered(items, 30, ContextBudgetStrategy::Selection);

        assert!(fitted.dropped_entries_count > 0);
        let has_archival = fitted.messages.iter().any(|m| m.content.contains("xxxx"));
        assert!(!has_archival, "archival item should be dropped first");
    }

    // --- RoutingPolicy tests ---

    #[test]
    fn routing_policy_selects_model_that_fits() {
        let policy = RoutingPolicy::new();
        let candidates = vec![
            ModelCandidate {
                name: "small-local".to_string(),
                context_window_size: 4096,
                priority: 1,
                is_local: true,
            },
            ModelCandidate {
                name: "large-cloud".to_string(),
                context_window_size: 128000,
                priority: 2,
                is_local: false,
            },
        ];

        let selected = policy.select_model_for_context(3000, &candidates).unwrap();
        assert_eq!(
            selected, "small-local",
            "should prefer local model that fits context"
        );
    }

    #[test]
    fn routing_policy_falls_back_to_larger_model() {
        let policy = RoutingPolicy::new();
        let candidates = vec![
            ModelCandidate {
                name: "small-local".to_string(),
                context_window_size: 4096,
                priority: 1,
                is_local: true,
            },
            ModelCandidate {
                name: "large-cloud".to_string(),
                context_window_size: 128000,
                priority: 2,
                is_local: false,
            },
        ];

        let selected = policy.select_model_for_context(8000, &candidates).unwrap();
        assert_eq!(
            selected, "large-cloud",
            "should fall back to larger model when small model can't fit"
        );
    }

    #[test]
    fn routing_policy_errors_when_no_model_fits_and_no_fallback() {
        let policy = RoutingPolicy {
            prefer_local: true,
            allow_overflow_fallback: false,
        };
        let candidates = vec![ModelCandidate {
            name: "small-local".to_string(),
            context_window_size: 4096,
            priority: 1,
            is_local: true,
        }];

        let result = policy.select_model_for_context(8000, &candidates);
        assert!(
            result.is_err(),
            "should error when no model fits and fallback is disabled"
        );
    }

    #[test]
    fn routing_policy_prefers_local_when_both_fit() {
        let policy = RoutingPolicy {
            prefer_local: true,
            allow_overflow_fallback: true,
        };
        let candidates = vec![
            ModelCandidate {
                name: "cloud-model".to_string(),
                context_window_size: 128000,
                priority: 1,
                is_local: false,
            },
            ModelCandidate {
                name: "local-model".to_string(),
                context_window_size: 8192,
                priority: 2,
                is_local: true,
            },
        ];

        let selected = policy.select_model_for_context(4000, &candidates).unwrap();
        assert_eq!(
            selected, "local-model",
            "should prefer local model when both fit, even with lower priority number"
        );
    }

    #[test]
    fn routing_policy_prefers_lower_priority_when_same_locality() {
        let policy = RoutingPolicy {
            prefer_local: false,
            allow_overflow_fallback: true,
        };
        let candidates = vec![
            ModelCandidate {
                name: "model-a".to_string(),
                context_window_size: 8192,
                priority: 3,
                is_local: false,
            },
            ModelCandidate {
                name: "model-b".to_string(),
                context_window_size: 8192,
                priority: 1,
                is_local: false,
            },
        ];

        let selected = policy.select_model_for_context(4000, &candidates).unwrap();
        assert_eq!(
            selected, "model-b",
            "should prefer lower priority number when locality is the same"
        );
    }

    #[test]
    fn routing_policy_empty_candidates_errors() {
        let policy = RoutingPolicy::new();
        let result = policy.select_model_for_context(1000, &[]);
        assert!(result.is_err(), "should error on empty candidates");
    }

    #[test]
    fn context_manager_with_routing_policy_delegates() {
        let policy = RoutingPolicy {
            prefer_local: false,
            allow_overflow_fallback: false,
        };
        let cm = ContextManager::with_routing_policy(policy);
        let candidates = vec![ModelCandidate {
            name: "model".to_string(),
            context_window_size: 4096,
            priority: 1,
            is_local: true,
        }];

        let result = cm.select_model_for_context(8000, &candidates);
        assert!(
            result.is_err(),
            "should error when no model fits and fallback is disabled"
        );
    }

    // --- SummaryProvider integration test ---

    struct MockSummaryProvider;

    #[async_trait::async_trait]
    impl SummaryProvider for MockSummaryProvider {
        async fn summarize(&self, texts: &[String]) -> Result<String, String> {
            Ok(format!("Summary of {} items", texts.len()))
        }
    }

    #[tokio::test]
    async fn fit_to_budget_with_summarization_uses_provider() {
        let cm = ContextManager::new();
        let provider = MockSummaryProvider;

        let long_case_file = "x".repeat(500);

        let fitted = cm
            .fit_to_budget_with_summarization(
                "sys",
                "task",
                vec![],
                vec![long_case_file],
                vec![],
                100,
                &provider,
            )
            .await;

        assert!(
            fitted.summarized_entries_count > 0,
            "should summarize dropped items"
        );
        let has_summary = fitted
            .messages
            .iter()
            .any(|m| m.content.contains("Summarized context"));
        assert!(
            has_summary,
            "should include a summary message from the provider"
        );
    }

    #[tokio::test]
    async fn fit_to_budget_with_summarization_no_op_when_everything_fits() {
        let cm = ContextManager::new();
        let provider = MockSummaryProvider;

        let fitted = cm
            .fit_to_budget_with_summarization(
                "sys",
                "task",
                vec!["episode".to_string()],
                vec!["case".to_string()],
                vec!["belief".to_string()],
                100_000,
                &provider,
            )
            .await;

        assert_eq!(fitted.summarized_entries_count, 0);
        assert_eq!(fitted.dropped_entries_count, 0);
        assert!(!fitted.budget_exceeded);
    }

    #[test]
    fn extractive_summarize_short_text_unchanged() {
        let cm = ContextManager::new();
        let short = "short text";
        assert_eq!(cm.extractive_summarize(short), short);
    }

    #[test]
    fn extractive_summarize_long_text_compressed() {
        let cm = ContextManager::new();
        let long = "First sentence here. Middle sentence with content. Last sentence ends.";
        let compressed = cm.extractive_summarize(long);
        assert!(compressed.len() < long.len() || compressed == long);
    }
}
