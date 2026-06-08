use super::*;

pub fn selftest() -> Result<()> {
    assert_eq!(
        population_mode_counts(24),
        BTreeMap::from([
            ("exploitation".to_string(), 12usize),
            ("novelty".to_string(), 8usize),
            ("wildcard".to_string(), 4usize),
        ])
    );
    assert_eq!(
        reject_information_text("", ""),
        Some("empty_provenance".to_string())
    );
    assert_eq!(
        reject_information_text("source.md", "copied benchmark value 0.123"),
        Some("copied_benchmark_values".to_string())
    );

    let stage_registry = vec![
        json!({
            "stage_id": "s1",
            "family": "hard",
            "track": "routing",
            "name": "one",
            "purpose": "p",
            "inputs": ["a"],
            "outputs": ["b"],
            "required_evidence": ["c"],
            "validation_checks": ["d"],
            "mutation_op": "emit_atlas",
            "prompt_hash": "abcdef0123",
        }),
        json!({
            "stage_id": "s2",
            "family": "standard",
            "track": "routing",
            "name": "two",
            "purpose": "p",
            "inputs": ["a"],
            "outputs": ["b"],
            "required_evidence": ["c"],
            "validation_checks": ["d"],
            "mutation_op": "emit_atlas",
            "prompt_hash": "abcdef0123",
        }),
    ];
    let concepts = vec![json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "concept_gene",
        "gene_id": "seed-derivation-first",
        "concept_id": "derivation-first",
        "family": "foundations",
        "domain": "foundations",
        "claim": "derive first",
        "method": "selftest",
        "constraint": "local",
        "failure_risk": "none",
        "stage_concept_hint": "derivation-first",
        "source_card_ids": [],
        "source_path": "selftest",
        "provenance_hash": stable_hash("selftest"),
        "novelty_terms": ["derivation"],
    })];
    let choices_a =
        adapt_stage_concepts_from_values(&stage_registry, &concepts, 3, 2, DEFAULT_SEED);
    let choices_b =
        adapt_stage_concepts_from_values(&stage_registry, &concepts, 3, 2, DEFAULT_SEED);
    assert_eq!(choices_a, choices_b);
    let scores_a = compute_candidate_scores(
        "novelty",
        "foundations",
        "novelty_jump",
        &choices_a,
        3,
        8,
        2,
        DEFAULT_SEED,
        true,
        0.08,
        0.13,
    );
    let scores_b = compute_candidate_scores(
        "novelty",
        "foundations",
        "novelty_jump",
        &choices_b,
        3,
        8,
        2,
        DEFAULT_SEED,
        true,
        0.08,
        0.13,
    );
    assert_eq!(scores_a, scores_b);
    assert!(lineage_edges_are_acyclic(&BTreeMap::from([
        ("b".to_string(), vec!["a".to_string()]),
        ("c".to_string(), vec!["b".to_string()]),
    ])));
    assert!(!lineage_edges_are_acyclic(&BTreeMap::from([
        ("a".to_string(), vec!["c".to_string()]),
        ("b".to_string(), vec!["a".to_string()]),
        ("c".to_string(), vec!["b".to_string()]),
    ])));
    println!("zyal-genome selftest passed");
    Ok(())
}
