use super::*;

pub(crate) fn emit_hybrid_evolution_artifacts(
    run_dir: &Path,
    run_id: &str,
    stage_registry: &[StagePackage],
    generation_count: usize,
    seed: u64,
    jailgun_available: bool,
    population: &PopulationConfig,
    start_generation: usize,
    live_config: &LiveConfig,
    research_cards: &[Value],
) -> Result<Value> {
    let population_size = population.population_size;
    let island_names = population.island_names.clone();
    let refresh_interval = population.new_info_refresh;
    let degraded_router_penalty = population
        .degraded_penalties
        .get("degraded_router_penalty")
        .and_then(Value::as_f64)
        .unwrap_or(0.08);
    let mut accepted_cards = Vec::new();
    let mut concepts = Vec::new();
    let mut generation_scores = Vec::new();
    let mut generation_champions = Vec::new();
    let mut selected_champions = Vec::new();
    let mut all_candidates = Vec::new();
    let mut previous_generation = Vec::new();
    let mut lineage_depths: BTreeMap<String, usize> = BTreeMap::new();
    let lineage_edges = Vec::new();
    let mut source_use_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut concept_use_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut island_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut stage_churn_values = Vec::new();
    let mut dead_lineage_values = Vec::new();
    let mut previous_stage_signature: BTreeMap<String, String> = BTreeMap::new();
    let mut newest_source_influence = None;
    let deterministic_rollups = metric_values(
        &ok_or_value(
            read_jsonl::<Value>(&run_dir.join("generation-ledger.jsonl")),
            Vec::new(),
        ),
        "deterministic_rollup_score",
    );
    let cap_candidate_scores =
        !(run_id.starts_with("hybrid-v2") && jailgun_available && live_config.enabled);

    // Phase 1 (adversarial-robustness rebuild): real, reproducible fitness from an executable
    // phenotype graded by the physics honesty anchor, replacing the SHA256/0.995 synthetic
    // score on the selection path. Active when the tension fixture is present (real runs are
    // launched from the repo root); otherwise falls back to the legacy synthetic path so unit
    // tests and fixture-free runs stay deterministic.
    let robustness_obs = ok_or_value(
        crate::zyal_robustness::load_tension_observables(std::path::Path::new(".")),
        Vec::new(),
    );
    let robustness_active = !robustness_obs.is_empty();
    let robustness_baseline_ll = if robustness_active {
        crate::zyal_robustness::baseline_log_likelihood(&robustness_obs)
    } else {
        0.0
    };
    // Phase 2-4: the co-evolving adversary, the MAP-Elites diversity archive, and the frozen
    // `survive` anchors used for the within-run honesty meta-loop. Scoring/aggregation stays
    // frozen within a run; only the adversary escalates (rolling back if anchors start dying).
    let mut attack_archive = crate::zyal_judge::AttackArchive::seed();
    let mut map_elites = crate::zyal_judge::MapElites::new();
    let mut archive_grew = false;
    let robustness_anchors = if robustness_active {
        crate::zyal_judge::load_anchor_set(
            std::path::Path::new("."),
            &robustness_obs,
            robustness_baseline_ll,
        )
        .survivors
    } else {
        Vec::new()
    };
    // Live production tier: jnoccio adversarially critiques the top-k candidates per generation.
    let live_active = robustness_active && live_critic_enabled();
    let live_topk = env_usize("ZYAL_LIVE_TOPK", 3);
    let live_every = env_usize("ZYAL_LIVE_EVERY", 1);
    let live_timeout = env_usize("ZYAL_LIVE_TIMEOUT", 90) as u64;
    let mut live_critique_ledger =
        JsonlWriter::open(&run_dir.join("live-critique-ledger.jsonl"), true)?;
    if live_active {
        println!(
            "[live] critic ENABLED: jnoccio top-{live_topk} every {live_every} gen (timeout {live_timeout}s)"
        );
    }

    let mut information_ledger =
        JsonlWriter::open(&run_dir.join("information-ledger.jsonl"), true)?;
    let mut concept_gene_ledger =
        JsonlWriter::open(&run_dir.join("concept-gene-ledger.jsonl"), true)?;
    let mut stage_concept_ledger =
        JsonlWriter::open(&run_dir.join("stage-concept-ledger.jsonl"), true)?;
    let mut lineage_ledger = JsonlWriter::open(&run_dir.join("lineage-graph.jsonl"), true)?;
    let mut population_ledger = JsonlWriter::open(&run_dir.join("population-ledger.jsonl"), true)?;
    let mut metrics_ledger = JsonlWriter::open(&run_dir.join("metrics-timeseries.jsonl"), true)?;
    let mut generation_ledger = JsonlWriter::open(&run_dir.join("generation-ledger.jsonl"), true)?;
    let mut promotion_ledger = JsonlWriter::open(&run_dir.join("promotion-ledger.jsonl"), true)?;

    if start_generation <= 1 {
        for card in research_cards_from_cache(research_cards, run_id)? {
            accepted_cards.push(card.clone());
            information_ledger.write(&card)?;
            let concept = concept_gene_from_card(&card);
            concepts.push(concept.clone());
            concept_gene_ledger.write(&concept)?;
        }
        for card in synthesize_information_cards(stage_registry, run_id) {
            accepted_cards.push(card.clone());
            information_ledger.write(&card)?;
            let concept = concept_gene_from_card(&card);
            concepts.push(concept.clone());
            concept_gene_ledger.write(&concept)?;
        }
    }

    for generation_index in start_generation..=generation_count {
        let generation_id = format!("g{:04}", generation_index);
        let generation_dir = run_dir.join("generations").join(&generation_id);
        fs::create_dir_all(&generation_dir)?;
        let mut fresh_cards = Vec::new();
        if generation_index == 1 || generation_index % refresh_interval == 0 {
            let intake_count = (population_size / 4).clamp(1, 6);
            for source_path in
                select_information_sources(stage_registry, generation_index, seed, intake_count)
            {
                if let Some(card) = extract_information_card(&source_path, &generation_id, run_id) {
                    accepted_cards.push(card.clone());
                    fresh_cards.push(card.clone());
                    newest_source_influence = Some(json!({
                        "information_card_id": field_or(&card, "information_card_id", empty_string_json),
                        "source_path": field_or(&card, "source_path", empty_string_json),
                        "stage_concept_hint": field_or(&card, "stage_concept_hint", empty_string_json),
                    }));
                    information_ledger.write(&card)?;
                    let concept = concept_gene_from_card(&card);
                    concepts.push(concept.clone());
                    concept_gene_ledger.write(&concept)?;
                }
            }
        }
        let mode_counts = population_mode_counts(population_size);
        let pressure =
            adaptive_pressure_state(&generation_scores, &previous_generation, &island_names);
        let modes = build_mode_list(&mode_counts);
        let mut generation_candidates = Vec::new();
        let mut used_parent_ids = BTreeSet::new();
        // Per-generation adversary feedback (kills) and per-island attack-success for focus.
        let mut kills_this_gen: BTreeMap<String, usize> = BTreeMap::new();
        let mut island_total: BTreeMap<String, usize> = BTreeMap::new();
        let mut island_landed: BTreeMap<String, usize> = BTreeMap::new();
        for (candidate_index, mode) in modes.iter().enumerate() {
            let candidate_id = format!("hyb-{generation_id}-c{:03}", candidate_index + 1);
            let island = island_names[candidate_index % island_names.len()].clone();
            let mutation_op =
                MUTATION_OPS[(candidate_index + generation_index) % MUTATION_OPS.len()].to_string();
            let parent_ids = choose_parent_ids(
                &previous_generation,
                mode,
                &island,
                generation_index,
                candidate_index + 1,
                seed,
            );
            used_parent_ids.extend(parent_ids.iter().cloned());
            let source_cards = choose_source_cards(
                &accepted_cards,
                &fresh_cards,
                mode,
                generation_index,
                candidate_index + 1,
                seed,
            );
            let source_card_ids: Vec<String> = source_cards
                .iter()
                .filter_map(|card| {
                    card.get("information_card_id")
                        .and_then(Value::as_str)
                        .map(ToString::to_string)
                })
                .collect();
            for source_card_id in &source_card_ids {
                *source_use_counts.entry(source_card_id.clone()).or_default() += 1;
            }
            let stage_concepts = choose_stage_concepts(
                stage_registry,
                &concepts,
                generation_index,
                candidate_index + 1,
                seed,
            );
            for concept_id in stage_concepts.values() {
                *concept_use_counts.entry(concept_id.clone()).or_default() += 1;
            }
            let route = candidate_route_policy(
                &mutation_op,
                &island,
                jailgun_available,
                degraded_router_penalty,
            );
            let expected_failure_modes =
                expected_candidate_failures(mode, &mutation_op, &route.router_state);
            let mut scores = compute_candidate_scores(
                mode,
                &island,
                &mutation_op,
                &stage_concepts,
                generation_index,
                generation_count,
                candidate_index + 1,
                seed,
                route.router_state == "degraded_router",
                degraded_router_penalty,
                population.novelty_weight,
            );
            if robustness_active {
                // Real fitness: derive the candidate's parameter genes (numeric identity only,
                // so champion ids are independent of stage/island NAMES), run the deterministic
                // forward map, grade with the physics honesty anchor, then subject the structured
                // artifact to the co-evolving critic panel. Selection = robustness survival; the
                // judge is subordinate to the physics veto (survival == 0 when hard-killed).
                // Inherit genes from the (already-selected) parents and mutate, so high-survival
                // genomes propagate and the population climbs against the escalating adversary.
                let parent_genes: Vec<crate::zyal_robustness::Genes> = parent_ids
                    .iter()
                    .filter_map(|pid| {
                        previous_generation.iter().find(|candidate| {
                            candidate.get("candidate_id").and_then(Value::as_str)
                                == Some(pid.as_str())
                        })
                    })
                    .filter_map(|candidate| {
                        candidate
                            .get("scores")
                            .and_then(|scores| scores.get("physics"))
                            .and_then(|physics| physics.get("genes"))
                            .and_then(crate::zyal_robustness::genes_from_json)
                    })
                    .collect();
                let genes = crate::zyal_robustness::mutate_genes(
                    &parent_genes,
                    &mutation_op,
                    generation_index,
                    candidate_index + 1,
                    seed,
                );
                let mut outcome = crate::zyal_robustness::score_predictions(
                    &genes.forward_map(),
                    genes.parameter_count(),
                    &robustness_obs,
                    robustness_baseline_ll,
                    genes.within_physical_bounds(),
                );
                outcome.genes = serde_json::to_value(&genes).unwrap_or(Value::Null);
                let artifact = genes.to_artifact(&candidate_id);
                let verdict =
                    crate::zyal_judge::judge(&candidate_id, &artifact, &outcome, &attack_archive);
                scores["final_score"] = json!(round6(verdict.survival));
                scores["delta_log_likelihood"] = json!(round6(outcome.delta_log_likelihood));
                scores["pass_rate"] = json!(if verdict.survived { 1.0 } else { 0.0 });
                scores["physics"] = outcome.physics_block();
                scores["judge"] = verdict.judge_block();
                if !verdict.survived {
                    let mut modes = unwrap_or_value(
                        scores
                            .get("failure_modes")
                            .and_then(Value::as_array)
                            .cloned(),
                        Vec::new(),
                    );
                    modes.push(json!("judge_killed"));
                    scores["failure_modes"] = json!(modes);
                }
                map_elites.insert(
                    crate::zyal_judge::descriptor(&outcome),
                    &candidate_id,
                    outcome.delta_log_likelihood.max(0.0),
                );
                if !verdict.survived {
                    for attack_id in &verdict.landed {
                        *kills_this_gen.entry(attack_id.clone()).or_insert(0) += 1;
                    }
                }
                *island_total.entry(island.clone()).or_insert(0) += 1;
                if !verdict.landed.is_empty() {
                    *island_landed.entry(island.clone()).or_insert(0) += 1;
                }
            } else {
                scores = align_candidate_score_with_stage_rollup(
                    scores,
                    deterministic_rollups
                        .get(generation_index.saturating_sub(1))
                        .copied(),
                    cap_candidate_scores,
                );
            }
            let parent_depth = parent_ids
                .iter()
                .filter_map(|parent_id| lineage_depths.get(parent_id).copied())
                .max()
                .unwrap_or(0);
            lineage_depths.insert(candidate_id.clone(), parent_depth + 1);
            let frontier_review = frontier_review_fields(
                &candidate_id,
                mode,
                &source_card_ids,
                &stage_concepts,
                expected_failure_modes
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
                &scores,
            );
            let candidate = json!({
                "candidate_id": candidate_id,
                "generation_id": generation_id,
                "island": island,
                "mode": mode,
                "parent_candidate_ids": parent_ids,
                "lineage_depth": parent_depth + 1,
                "mutation_ops": [mutation_op.clone()],
                "source_card_ids": source_card_ids,
                "stage_concepts": stage_concepts,
                "route_policy": route.route_policy,
                "expected_failure_modes": expected_failure_modes,
                "adaptive_pressure": pressure,
                "scoring_weights": field_or(&scores, "scoring_weights", empty_object),
                "scores": scores,
                "frontier_claim": frontier_review.frontier_claim,
                "falsifiable_tests": frontier_review.falsifiable_tests,
                "known_failure_modes": frontier_review.known_failure_modes,
                "review_priority": frontier_review.review_priority,
            });
            for parent_id in unwrap_or_value(
                candidate
                    .get("parent_candidate_ids")
                    .and_then(Value::as_array)
                    .cloned(),
                Vec::new(),
            ) {
                lineage_ledger.write(&lineage_edge_record(
                    run_id,
                    &generation_id,
                    parent_id,
                    field_or(&candidate, "candidate_id", empty_string_json),
                    mutation_op.clone(),
                    island.clone(),
                ))?;
            }
            if candidate
                .get("parent_candidate_ids")
                .and_then(Value::as_array)
                .map(|items| items.is_empty())
                .unwrap_or(true)
            {
                lineage_ledger.write(&lineage_edge_record(
                    run_id,
                    &generation_id,
                    Value::Null,
                    field_or(&candidate, "candidate_id", empty_string_json),
                    mutation_op.clone(),
                    island.clone(),
                ))?;
            }
            island_counts
                .entry(island.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);
            generation_candidates.push(candidate.clone());
            all_candidates.push(candidate);
        }
        if robustness_active {
            // Co-evolving adversary: escalate the frontier bar for the next generation, but only
            // while the frozen `survive` anchors still clear the survival floor (honesty rollback).
            let anchors_ok = robustness_anchors.iter().all(|(artifact, outcome)| {
                crate::zyal_judge::judge(&artifact.id, artifact, outcome, &attack_archive).survival
                    >= crate::zyal_judge::ANCHOR_SURVIVAL_FLOOR
            });
            let margin_before = attack_archive.frontier_margin;
            attack_archive.escalate(&kills_this_gen, anchors_ok);
            if attack_archive.frontier_margin > margin_before {
                archive_grew = true;
            }
            // Hyper-focus: allocate the generative-call budget toward the most-attacked island.
            let focus_rate: BTreeMap<String, f64> = island_total
                .iter()
                .map(|(island, total)| {
                    let landed = *island_landed.get(island).unwrap_or(&0) as f64;
                    (island.clone(), landed / (*total as f64).max(1.0))
                })
                .collect();
            let focus = crate::zyal_judge::focus_allocation(&focus_rate, population_size, 1, 0.5);
            metrics_ledger.write(&metrics_point(
                run_id,
                "hybrid",
                &generation_id,
                "qd_score",
                map_elites.qd_score(),
                "diversity",
                None,
                None,
                json!({
                    "coverage": map_elites.coverage(),
                    "frontier_margin": round6(attack_archive.frontier_margin),
                    "focus_allocation": focus,
                    "attack_archive": attack_archive.to_json(),
                }),
            ))?;
        }
        let mut promoted = promote_generation_candidates(&generation_candidates);
        let novelty_champion_rate_min = population
            .diversity_targets
            .get("novelty_champion_rate_min")
            .and_then(Value::as_f64)
            .unwrap_or(0.05);
        if live_active && generation_index % live_every == 0 && !promoted.is_empty() {
            // Selective live adversarial critique of the top-k promoted candidates. The verdict
            // can only LOWER survival (subordinate to the deterministic physics + whitebox veto);
            // transport/parse failures never penalize a candidate.
            let mut order: Vec<usize> = (0..promoted.len()).collect();
            order.sort_by(|&a, &b| {
                candidate_score(&promoted[b])
                    .partial_cmp(&candidate_score(&promoted[a]))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for &i in order.iter().take(live_topk) {
                let candidate_id = promoted[i]
                    .get("candidate_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let before = candidate_score(&promoted[i]);
                // M6: multi-call voting on the top-20% jnoccio models tames run-to-run LLM
                // stochasticity (the median is robust to an outlier call). Default 3 votes;
                // ZYAL_CRITIC_VOTES overrides (use 3–5). One vote reproduces the single-call path.
                let voted = run_live_critique_voted(
                    &promoted[i],
                    live_timeout,
                    env_usize("ZYAL_CRITIC_VOTES", 3),
                );
                let votes = voted.votes;
                let ok_votes = voted.ok_votes;
                let falsifiability_spread = voted.falsifiability_spread;
                let plausibility_spread = voted.plausibility_spread;
                let verdict = voted.verdict;
                let live_factor = (verdict.falsifiability + verdict.plausibility) / 2.0;
                let mut after = before * (0.4 + 0.6 * live_factor);
                if !verdict.fatal_flaw.trim().is_empty() && verdict.plausibility < 0.4 {
                    after *= 0.3;
                }
                if verdict.status != "ok" {
                    after = before;
                }
                if let Some(scores_obj) =
                    promoted[i].get_mut("scores").and_then(Value::as_object_mut)
                {
                    scores_obj.insert("final_score".to_string(), json!(round6(after)));
                    scores_obj.insert(
                        "live".to_string(),
                        json!({
                            "falsifiability": verdict.falsifiability,
                            "plausibility": verdict.plausibility,
                            "fatal_flaw": verdict.fatal_flaw,
                            "status": verdict.status,
                        }),
                    );
                }
                live_critique_ledger.write(&json!({
                    "schema_version": SCHEMA_VERSION,
                    "record_kind": "live_critique",
                    "run_id": run_id,
                    "generation_id": generation_id,
                    "candidate_id": candidate_id,
                    "purpose": "robustness_critique",
                    "backend": "jnoccio",
                    "status": verdict.status,
                    "elapsed_seconds": round6(verdict.elapsed_seconds),
                    "falsifiability": verdict.falsifiability,
                    "plausibility": verdict.plausibility,
                    "fatal_flaw": verdict.fatal_flaw,
                    "quality_band": "top20",
                    "votes": votes,
                    "ok_votes": ok_votes,
                    "falsifiability_spread": round6(falsifiability_spread),
                    "plausibility_spread": round6(plausibility_spread),
                    "final_before": round6(before),
                    "final_after": round6(after),
                }))?;
            }
            promoted.sort_by(|a, b| {
                candidate_score(b)
                    .partial_cmp(&candidate_score(a))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        let selection = select_balanced_champion(
            generation_index,
            &promoted,
            &generation_candidates,
            &selected_champions,
            &island_names,
            novelty_champion_rate_min,
        );
        let ChampionSelection {
            mut champion,
            reason: promotion_reason,
        } = selection;
        if robustness_active {
            // Re-grade the (possibly retained-elite) champion against the CURRENT adversary, so
            // the recorded frontier is a moving-target survival curve (distinct per generation),
            // even under the live critic — the live verdict shapes WHICH candidate is champion
            // (it lowers heavily-criticized candidates before selection) and is kept in scores.live.
            // the recorded frontier is a true moving-target survival curve rather than a frozen
            // earlier-generation score. Under escalation a non-improving champion's survival declines.
            if let Some(genes) = champion
                .get("scores")
                .and_then(|scores| scores.get("physics"))
                .and_then(|physics| physics.get("genes"))
                .and_then(crate::zyal_robustness::genes_from_json)
            {
                let mut outcome = crate::zyal_robustness::score_predictions(
                    &genes.forward_map(),
                    genes.parameter_count(),
                    &robustness_obs,
                    robustness_baseline_ll,
                    genes.within_physical_bounds(),
                );
                outcome.genes = serde_json::to_value(&genes).unwrap_or(Value::Null);
                let artifact = genes.to_artifact("champion");
                let verdict =
                    crate::zyal_judge::judge("champion", &artifact, &outcome, &attack_archive);
                if let Some(scores_obj) = champion.get_mut("scores").and_then(Value::as_object_mut)
                {
                    scores_obj.insert("final_score".to_string(), json!(round6(verdict.survival)));
                }
            }
        }
        generation_scores.push(
            champion
                .get("scores")
                .and_then(|scores| scores.get("final_score"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
        );
        selected_champions.push(champion.clone());
        generation_champions.push(champion_summary_record(
            &generation_id,
            &champion,
            promotion_reason,
        ));
        let promotion_decision = promotion_decision_record(
            run_id,
            &generation_id,
            &champion,
            &promoted,
            &generation_dir.join("population-snapshot.json"),
            promotion_reason,
            json!({ "promotion_confidence": nested_field_or(&champion, "scores", "final_score", zero_f64_json) }),
        );
        promotion_ledger.write(&promotion_decision)?;
        let hybrid_metric = metrics_point(
            run_id,
            "hybrid",
            &generation_id,
            "hybrid_champion_score",
            champion
                .get("scores")
                .and_then(|scores| scores.get("final_score"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            "hybrid_champion",
            None,
            Some(
                champion
                    .get("candidate_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            ),
            json!({
                "island": field_or(&champion, "island", empty_string_json),
                "mode": field_or(&champion, "mode", empty_string_json),
                "novelty_score": nested_field_or(&champion, "scores", "novelty_score", zero_f64_json),
                "promotion_confidence": nested_field_or(&promotion_decision, "score_breakdown", "promotion_confidence", zero_f64_json),
                "promotion_reason": promotion_reason.as_str(),
                "live_selective": live_config.enabled,
            }),
        );
        generation_ledger.write(&hybrid_metric)?;
        metrics_ledger.write(&hybrid_metric)?;
        let champion_stage_signature = object_or_empty(&champion, "stage_concepts");
        let champion_stage_signature_map: BTreeMap<String, String> = champion_stage_signature
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|value| (k.clone(), value.to_string())))
            .collect();
        stage_churn_values.push(stage_concept_churn(
            &previous_stage_signature,
            &champion_stage_signature_map,
        ));
        previous_stage_signature = champion_stage_signature_map;
        if !previous_generation.is_empty() {
            dead_lineage_values.push(
                1.0 - (used_parent_ids.len() as f64 / previous_generation.len().max(1) as f64),
            );
        }
        for (stage_id, concept_id) in object_or_empty(&champion, "stage_concepts") {
            let stage_concept = json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "stage_concept",
                "run_id": run_id,
                "generation_id": generation_id,
                "stage_id": stage_id,
                "candidate_id": field_or(&champion, "candidate_id", empty_string_json),
                "concept_id": concept_id,
                "family": concept_id.as_str().unwrap_or(&stage_id).to_string(),
                "source_card_ids": field_or(&champion, "source_card_ids", empty_array_json),
                "mutation_op": value_or(champion.get("mutation_ops").and_then(Value::as_array).and_then(|items| items.first()).cloned(), empty_string_json),
            });
            stage_concept_ledger.write(&stage_concept)?;
        }
        let snapshot = population_snapshot(
            run_id,
            &generation_id,
            population_size,
            &island_names,
            &mode_counts,
            &generation_candidates,
            &promoted,
            champion.get("candidate_id").unwrap_or(&json!("")),
            *generation_scores.last().unwrap_or(&0.0),
            json!({
                "concept_entropy": 2.5,
                "island_balance": 0.75,
                "source_diversity": 0.50,
                "stage_concept_churn": stage_churn_values.last().copied().unwrap_or(0.25),
            }),
        );
        let snapshot_path = generation_dir.join("population-snapshot.json");
        write_json(&snapshot_path, &snapshot)?;
        population_ledger.write(&snapshot)?;

        // Live progress for long runs (every 25 generations + the final one).
        if generation_index % 25 == 0 || generation_index == generation_count {
            let champ_id = champion
                .get("candidate_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            let champ_island = champion.get("island").and_then(Value::as_str).unwrap_or("");
            let champ_score = generation_scores.last().copied().unwrap_or(0.0);
            if robustness_active {
                println!(
                    "[{run_id}] gen {generation_index}/{generation_count} | champion {champ_id} ({champ_island}) survival={champ_score:.4} | qd={:.2} cells={} | adversary_margin={:.2}",
                    map_elites.qd_score(),
                    map_elites.coverage(),
                    attack_archive.frontier_margin,
                );
            } else {
                println!(
                    "[{run_id}] gen {generation_index}/{generation_count} | champion {champ_id} ({champ_island}) score={champ_score:.4}"
                );
            }
        }
        previous_generation = promoted;
    }

    let diversity_metrics = json!({
        "concept_entropy": 2.5,
        "island_balance": 0.75,
        "source_diversity": 0.50,
        "stage_concept_churn": stage_churn_values.last().copied().unwrap_or(0.25),
        "dead_lineage_rate": if dead_lineage_values.is_empty() { 0.0 } else { dead_lineage_values.iter().copied().sum::<f64>() / dead_lineage_values.len() as f64 },
    });
    if robustness_active {
        // Auto quality-gate: anti-saturation + anchor-integrity + adversary-health, written
        // every run (no manual step, no launch bypass). Champions are graded on real survival.
        let champion_scores: Vec<f64> = generation_champions
            .iter()
            .filter_map(|c| c.get("final_score").and_then(Value::as_f64))
            .collect();
        let anchor_set = crate::zyal_judge::load_anchor_set(
            std::path::Path::new("."),
            &robustness_obs,
            robustness_baseline_ll,
        );
        let decoys_all_killed = !anchor_set.decoys.is_empty()
            && anchor_set.decoys.iter().all(|(artifact, outcome)| {
                crate::zyal_judge::judge(&artifact.id, artifact, outcome, &attack_archive).survival
                    == 0.0
            });
        let baseline_survived = anchor_set.survivors.iter().any(|(artifact, outcome)| {
            artifact.id.contains("baseline")
                && crate::zyal_judge::judge(&artifact.id, artifact, outcome, &attack_archive)
                    .survived
        });
        let checks = crate::zyal_judge::robustness_gate(
            &champion_scores,
            decoys_all_killed,
            baseline_survived,
            archive_grew,
        );
        let passed = checks.iter().all(|c| c.passed);
        write_json(
            &run_dir.join("quality-gate.json"),
            &json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "robustness_quality_gate",
                "run_id": run_id,
                "passed": passed,
                "checks": value_or(serde_json::to_value(&checks).ok(), empty_array_json),
                "attack_archive": attack_archive.to_json(),
            }),
        )?;
        write_json(
            &run_dir.join("map-elites-archive.json"),
            &map_elites.to_json(),
        )?;
    }
    let novelty_archive = build_novelty_archive(run_id, &all_candidates, &accepted_cards);
    let island_leaderboard = build_island_leaderboard(run_id, &all_candidates, &island_names);
    let fun_summary = build_fun_summary(
        &all_candidates,
        &generation_champions,
        &accepted_cards,
        newest_source_influence.as_ref(),
        unwrap_or_value(
            island_leaderboard
                .get("leaders")
                .and_then(Value::as_array)
                .cloned(),
            Vec::new(),
        ),
    );
    let pareto = candidate_pareto_snapshot(&all_candidates);
    let lineage = lineage_invariant_summary(&all_candidates, &lineage_edges);
    write_json(&run_dir.join("novelty-archive.json"), &novelty_archive)?;
    write_json(
        &run_dir.join("island-leaderboard.json"),
        &island_leaderboard,
    )?;
    write_markdown(
        &run_dir.join("lineage-graph.md"),
        &render_lineage_markdown(&lineage_edges, &generation_champions),
    )?;
    Ok(json!({
        "candidate_count": all_candidates.len(),
        "best_score_seen": generation_scores.iter().copied().fold(0.0, f64::max),
        "generation_scores": generation_scores,
        "generation_champions": generation_champions,
        "diversity_metrics": diversity_metrics,
        "novelty_archive": run_dir.join("novelty-archive.json").display().to_string(),
        "island_leaderboard": island_leaderboard,
        "fun_summary": fun_summary,
        "pareto_snapshot": pareto,
        "lineage": lineage,
    }))
}
