use super::*;

#[test]
fn test_safety_classifier_unused_import() {
    let classifier = SafetyClassifier::new();
    let refactoring = RefactoringCandidate {
        refactor_type: RefactorType::RemoveUnusedImport,
        reference_count: Some(0),
        scope: Scope::Local,
        has_side_effects: false,
        is_unreachable: false,
        is_recursive: false,
        involves_generics: false,
        involves_macros: false,
        evidence_strength: EvidenceStrength::Strong,
        location: Location {
            file: "test.rs".to_string(),
            line: 1,
            character: 1,
        },
        message: "Unused import".to_string(),
        refactor_call_args: serde_json::Value::Null,
    };
    let context = AnalysisContext {
        file_path: "test.rs".to_string(),
        has_full_type_info: true,
        has_partial_type_info: true,
        ast_parse_errors: 0,
    };

    let safety = classifier.classify(&refactoring, &context).unwrap();
    assert_eq!(safety, SafetyLevel::Safe);
}

#[test]
fn test_confidence_scorer_high_confidence() {
    let scorer = ConfidenceScorer::new();
    let refactoring = RefactoringCandidate {
        refactor_type: RefactorType::RemoveUnusedImport,
        evidence_strength: EvidenceStrength::Strong,
        reference_count: Some(0),
        scope: Scope::Local,
        has_side_effects: false,
        is_unreachable: false,
        is_recursive: false,
        involves_generics: false,
        involves_macros: false,
        location: Location {
            file: "test.rs".to_string(),
            line: 1,
            character: 1,
        },
        message: "Unused import".to_string(),
        refactor_call_args: serde_json::Value::Null,
    };
    let context = AnalysisContext {
        file_path: "test.rs".to_string(),
        has_full_type_info: true,
        has_partial_type_info: true,
        ast_parse_errors: 0,
    };

    let confidence = scorer.score(&refactoring, &context).unwrap();
    assert!(confidence > 0.8, "Expected high confidence, got {}", confidence);
}

#[test]
fn test_suggestion_ranking() {
    let ranker = SuggestionRanker::new();
    let mut suggestions = vec![
        ActionableSuggestion {
            message: "Review".to_string(),
            safety: SafetyLevel::RequiresReview,
            confidence: 0.9,
            reversible: true,
            estimated_impact: ImpactLevel::Medium,
            refactor_call: None,
            metadata: None,
        },
        ActionableSuggestion {
            message: "Safe".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.7,
            reversible: true,
            estimated_impact: ImpactLevel::Low,
            refactor_call: None,
            metadata: None,
        },
    ];

    ranker.rank(&mut suggestions);

    // Safe should come first even with lower confidence
    assert_eq!(suggestions[0].safety, SafetyLevel::Safe);
    assert_eq!(suggestions[1].safety, SafetyLevel::RequiresReview);
}
