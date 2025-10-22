use super::*;
use anyhow::Result;

pub struct SafetyClassifier;

impl SafetyClassifier {
    pub fn new() -> Self {
        Self
    }

    /// Classify refactoring safety level
    pub fn classify(
        &self,
        refactoring: &RefactoringCandidate,
        context: &AnalysisContext,
    ) -> Result<SafetyLevel> {
        // SAFE: Low-risk, localized changes
        if self.is_safe_refactoring(refactoring, context) {
            return Ok(SafetyLevel::Safe);
        }

        // EXPERIMENTAL: High-risk, complex changes
        if self.is_experimental_refactoring(refactoring, context) {
            return Ok(SafetyLevel::Experimental);
        }

        // REQUIRES_REVIEW: Default for everything else
        Ok(SafetyLevel::RequiresReview)
    }

    fn is_safe_refactoring(
        &self,
        refactoring: &RefactoringCandidate,
        _context: &AnalysisContext,
    ) -> bool {
        match refactoring.refactor_type {
            // Always safe
            RefactorType::RemoveUnusedImport => true,
            RefactorType::RemoveUnusedVariable => {
                // Safe if truly unused (no references)
                refactoring.reference_count == Some(0)
            }
            RefactorType::SimplifyBooleanExpression => {
                // Safe if local scope, no side effects
                refactoring.scope == Scope::Local && !refactoring.has_side_effects
            }
            RefactorType::RemoveDeadCode => {
                // Safe if unreachable and no references
                refactoring.is_unreachable && refactoring.reference_count == Some(0)
            }
            // Everything else requires review or is experimental
            _ => false,
        }
    }

    fn is_experimental_refactoring(
        &self,
        refactoring: &RefactoringCandidate,
        _context: &AnalysisContext,
    ) -> bool {
        // Experimental if:
        // 1. Cross-crate changes
        if refactoring.scope == Scope::CrossCrate {
            return true;
        }

        // 2. Recursive operations
        if refactoring.is_recursive {
            return true;
        }

        // 3. Generic/template transformations
        if refactoring.involves_generics {
            return true;
        }

        // 4. Macro transformations
        if refactoring.involves_macros {
            return true;
        }

        false
    }
}
