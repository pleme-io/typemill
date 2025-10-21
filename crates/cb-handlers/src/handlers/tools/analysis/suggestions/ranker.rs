#![allow(
    dead_code,
    unused_variables,
    clippy::mutable_key_type,
    clippy::needless_range_loop,
    clippy::ptr_arg,
    clippy::manual_clamp
)]

use super::*;

pub struct SuggestionRanker;

impl SuggestionRanker {
    pub fn new() -> Self {
        Self
    }

    /// Rank suggestions by: safety → confidence → impact
    pub fn rank(&self, suggestions: &mut Vec<ActionableSuggestion>) {
        suggestions.sort_by(|a, b| {
            // 1. Safety first (safe > requires_review > experimental)
            let safety_order = self
                .safety_order(a.safety)
                .cmp(&self.safety_order(b.safety));
            if safety_order != std::cmp::Ordering::Equal {
                return safety_order;
            }

            // 2. Confidence second (higher is better)
            let confidence_order = b
                .confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal);
            if confidence_order != std::cmp::Ordering::Equal {
                return confidence_order;
            }

            // 3. Impact third (lower is better - prefer small changes)
            a.estimated_impact.cmp(&b.estimated_impact)
        });
    }

    fn safety_order(&self, safety: SafetyLevel) -> u8 {
        match safety {
            SafetyLevel::Safe => 0,
            SafetyLevel::RequiresReview => 1,
            SafetyLevel::Experimental => 2,
        }
    }
}
