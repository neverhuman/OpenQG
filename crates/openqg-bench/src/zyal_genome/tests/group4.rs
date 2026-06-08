use super::super::*;
use super::*;

#[test]
fn saturation_guard_caps_non_nominal_perfect_score() {
    assert_eq!(
        apply_saturation_guard(1.0, &[0.99, 0.99, 0.99], 0.01, false),
        0.995
    );
    assert_eq!(
        apply_saturation_guard(1.0, &[0.99, 0.99, 0.99], 0.01, true),
        1.0
    );
}

#[test]
fn promotion_preserves_mode_and_island_floor_candidates() {
    let candidates = vec![
        json!({"candidate_id":"a","mode":"exploitation","island":"wildcards","scores":{"final_score":0.99}}),
        json!({"candidate_id":"b","mode":"exploitation","island":"wildcards","scores":{"final_score":0.98}}),
        json!({"candidate_id":"c","mode":"novelty","island":"foundations","scores":{"final_score":0.70}}),
        json!({"candidate_id":"d","mode":"wildcard","island":"coefficients","scores":{"final_score":0.69}}),
        json!({"candidate_id":"e","mode":"exploitation","island":"observables","scores":{"final_score":0.68}}),
        json!({"candidate_id":"f","mode":"exploitation","island":"failure-repair","scores":{"final_score":0.67}}),
    ];
    let promoted = promote_generation_candidates(&candidates);
    let modes = count_string_field(&promoted, "mode");
    let islands = count_string_field(&promoted, "island");
    assert!(modes.contains_key("novelty"));
    assert!(modes.contains_key("wildcard"));
    assert!(islands.len() >= 4);
}

#[test]
fn novelty_quota_selects_viable_novelty_candidate_below_floor() {
    let promoted = vec![
        selection_candidate("top", "exploitation", "foundations", 0.810),
        selection_candidate("novel", "novelty", "coefficients", 0.805),
    ];
    let previous = previous_champions(21, 1, 0.80);

    let selection = select_balanced_champion(150, &promoted, &promoted, &previous, &[], 0.05);

    assert_eq!(selection.reason, PromotionReason::NoveltyQuota);
    assert_eq!(selection.champion["candidate_id"], json!("novel"));
}

#[test]
fn low_score_novelty_does_not_force_major_regression() {
    let promoted = vec![
        selection_candidate("top", "exploitation", "foundations", 0.900),
        selection_candidate("novel", "novelty", "coefficients", 0.720),
    ];
    let previous = previous_champions(20, 0, 0.88);

    let selection = select_balanced_champion(150, &promoted, &promoted, &previous, &[], 0.05);

    assert_eq!(selection.reason, PromotionReason::TopScore);
    assert_eq!(selection.champion["candidate_id"], json!("top"));
}

#[test]
fn elite_carry_forward_reduces_regression_and_preserves_metadata() {
    let previous_elite = selection_candidate("elite", "exploitation", "wildcards", 0.900);
    let promoted = vec![selection_candidate(
        "current-top",
        "novelty",
        "foundations",
        0.870,
    )];
    let mut previous = previous_champions(3, 0, 0.80);
    let mut elite_with_generation = previous_elite.clone();
    elite_with_generation["generation_id"] = json!("g0004");
    previous.push(elite_with_generation);

    let selection = select_balanced_champion(5, &promoted, &promoted, &previous, &[], 0.05);

    assert_eq!(selection.reason, PromotionReason::EliteRetain);
    assert_eq!(selection.champion["candidate_id"], json!("elite"));
    assert_eq!(selection.champion["source_card_ids"], json!(["info-a"]));
    assert_eq!(
        selection.champion["parent_candidate_ids"],
        json!(["parent-a"])
    );
    assert_eq!(selection.champion["island"], json!("wildcards"));
    assert_eq!(selection.champion["mode"], json!("exploitation"));
    assert_eq!(selection.champion["scores"]["final_score"], json!(0.900));
    assert_eq!(
        selection.champion["retained_from_generation_id"],
        json!("g0004")
    );
    assert_eq!(
        selection.champion["selection_generation_id"],
        json!("g0005")
    );
}

#[test]
fn promotion_ledger_records_selection_reason() {
    let champion = selection_candidate("cap", "exploitation", "foundations", 0.82);
    let record = promotion_decision_record(
        "run-1",
        "g0001",
        &champion,
        std::slice::from_ref(&champion),
        Path::new("population-snapshot.json"),
        PromotionReason::IslandCap,
        json!({"promotion_confidence": 0.82}),
    );

    assert_eq!(record["promotion_reason"], json!("island_cap"));
    validate_record_kind(&record).expect("promotion record should validate");
}

#[test]
fn frontier_review_fields_serialize_for_candidates_and_champions() {
    let stage = test_stage("03-generate-genes", "standard", Vec::new());
    let mut stage_concepts = BTreeMap::new();
    stage_concepts.insert("03-generate-genes".to_string(), "concept-a".to_string());
    let scores = json!({
        "final_score": 0.78,
        "novelty_score": 0.71,
        "failure_modes": ["interface_drift"],
        "scoring_weights": {"novelty_score": 0.18},
    });

    let candidate = hybrid_candidate_record(
        "g0001",
        &stage,
        "candidate-a",
        &["parent-a".to_string()],
        "novelty",
        &["info-a".to_string()],
        &stage_concepts,
        &json!({"backend": "jnoccio"}),
        &scores,
        &json!({}),
    );
    let champion = champion_summary_record("g0001", &candidate, PromotionReason::NoveltyQuota);

    assert_eq!(candidate["source_card_ids"], json!(["info-a"]));
    assert!(candidate["frontier_claim"]
        .as_str()
        .unwrap()
        .contains("candidate-a"));
    assert_eq!(candidate["known_failure_modes"], json!(["interface_drift"]));
    assert_eq!(candidate["review_priority"], json!("high"));
    assert_eq!(champion["frontier_claim"], candidate["frontier_claim"]);
    assert_eq!(
        champion["falsifiable_tests"],
        candidate["falsifiable_tests"]
    );
    assert_eq!(champion["promotion_reason"], json!("novelty_quota"));
}

#[test]
fn novelty_weight_changes_candidate_final_score() {
    let stage_concepts =
        BTreeMap::from([("03-generate-genes".to_string(), "concept-a".to_string())]);
    let low_weight = compute_candidate_scores(
        "novelty",
        "foundations",
        "novelty_jump",
        &stage_concepts,
        3,
        8,
        2,
        DEFAULT_SEED,
        false,
        0.08,
        0.0,
    );
    let high_weight = compute_candidate_scores(
        "novelty",
        "foundations",
        "novelty_jump",
        &stage_concepts,
        3,
        8,
        2,
        DEFAULT_SEED,
        false,
        0.08,
        0.18,
    );

    assert!(
        high_weight["final_score"].as_f64().unwrap() > low_weight["final_score"].as_f64().unwrap()
    );
}

#[test]
fn selftest_passes() {
    selftest().expect("selftest");
}
