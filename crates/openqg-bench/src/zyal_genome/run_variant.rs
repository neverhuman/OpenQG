use super::*;

pub fn run_variant(
    variant: GenomeVariant,
    max_generations: Option<usize>,
    generations: Option<usize>,
    population_size: Option<usize>,
    islands: Option<usize>,
    novelty_weight: Option<f64>,
    new_info_refresh: Option<usize>,
    seed: u64,
    output_root: PathBuf,
    live_selective: bool,
    research_cache: Option<PathBuf>,
    resume: bool,
    checkpoint_every: Option<usize>,
    run_id: Option<String>,
    stage_root: Option<PathBuf>,
    runbook: Option<PathBuf>,
    jailgun_available: bool,
    dry_run: bool,
) -> Result<()> {
    let runbook_path =
        runbook.unwrap_or_else(|| PathBuf::from(DEFAULT_RUNBOOK_ROOT).join(variant.runbook_name()));
    let runbook = load_runbook(&runbook_path)?;
    let evaluation = field_or(&runbook, "evaluation", empty_object);
    let jailgun_available = jailgun_available || jailgun_available_from_environment();

    let max_generations = resolve_generation_count(generations, max_generations, &evaluation)?;
    let population = resolve_population_config(
        population_size,
        islands,
        novelty_weight,
        new_info_refresh,
        &evaluation,
    )?;
    let stage_root = stage_root
        .or_else(|| {
            runbook
                .get("context")
                .and_then(|context| context.get("stage_root"))
                .and_then(Value::as_str)
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STAGE_ROOT));
    let output_root = if output_root != PathBuf::from(DEFAULT_OUTPUT_ROOT) {
        output_root
    } else {
        evaluation
            .get("output_root")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT_ROOT))
    };
    let run_id = run_id
        .or_else(|| {
            evaluation
                .get("run_id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| default_run_id(variant.as_str(), seed, max_generations));
    let run_dir = output_root.join("runs").join(&run_id);
    if run_dir.exists() && run_dir.read_dir()?.next().is_some() && !resume {
        bail!(
            "run directory already exists: {}; pass --resume or choose --run-id",
            run_dir.display()
        );
    }
    fs::create_dir_all(&run_dir)?;
    let output_guard = OutputPathGuard::new(&output_root)?;

    let stage_registry = load_stage_registry(&stage_root)?;
    validate_stage_registry(&stage_registry)?;
    let live_config = resolve_live_config(live_selective, &runbook);
    let hard_backend_required = hard_backend_required(&runbook, &variant, &run_id, max_generations);
    let jailgun_status = resolve_jailgun_status(
        strict_jailgun_required(
            &variant,
            &run_id,
            hard_backend_required,
            hard_stage_count(&stage_registry),
        ),
        jailgun_available,
    );
    let jailgun_available = jailgun_status.available;
    let receipt = preflight_receipt(
        &run_id,
        variant.as_str(),
        &runbook,
        &runbook_path,
        &run_dir,
        &stage_registry,
        &live_config,
        hard_backend_required,
        false,
        &jailgun_status,
    );
    output_guard.check()?;
    write_json(&run_dir.join("preflight.json"), &receipt)?;
    if hard_backend_required && hard_stage_count(&stage_registry) > 0 && !jailgun_available {
        bail!(
            "hybrid hard-stage backend unavailable for {}; run preflight or use an alternate run id",
            run_id
        );
    }
    let checkpoint_every = resolve_checkpoint_every(checkpoint_every, &evaluation)?;
    let research_cache = resolve_research_cache_path(research_cache, &runbook);
    let research_cards = load_research_cache_cards(research_cache.as_deref(), &run_id)?;
    let hybrid_mode = matches!(variant, GenomeVariant::Hybrid);
    let resume_state = if resume {
        load_resume_state(&run_dir, &stage_registry)?
    } else {
        empty_resume_state(&stage_registry)
    };

    let raw_completed_generation = resume_state.completed_generation;
    if resume
        && raw_completed_generation >= max_generations
        && run_dir.join("run-summary.json").exists()
    {
        output_guard.check()?;
        emit_run_plot_index(&run_dir)?;
        touch_latest(&output_root, &run_dir)?;
        println!("run already complete through g{raw_completed_generation:04}");
        return Ok(());
    }

    let completed_generation = raw_completed_generation.min(max_generations);
    let start_generation = completed_generation + 1;
    let append_mode = resume && completed_generation > 0;

    let mut run_events = JsonlWriter::open_guarded(
        &run_dir.join("run-events.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut stage_ledger = JsonlWriter::open_guarded(
        &run_dir.join("stage-ledger.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut generation_ledger = JsonlWriter::open_guarded(
        &run_dir.join("generation-ledger.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut metrics_ledger = JsonlWriter::open_guarded(
        &run_dir.join("metrics-timeseries.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut stage_variant_ledger = JsonlWriter::open_guarded(
        &run_dir.join("stage-variant-ledger.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut live_call_ledger = JsonlWriter::open_guarded(
        &run_dir.join("live-call-ledger.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut research_ledger = JsonlWriter::open_guarded(
        &run_dir.join("research-ledger.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut memory_ledger = JsonlWriter::open_guarded(
        &run_dir.join("memory-ledger.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut promotion_ledger = JsonlWriter::open_guarded(
        &run_dir.join("promotion-ledger.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut lineage_ledger = if hybrid_mode {
        Some(JsonlWriter::open_guarded(
            &run_dir.join("lineage-graph.jsonl"),
            append_mode,
            &output_guard,
        )?)
    } else {
        None
    };
    let mut population_ledger = if hybrid_mode {
        Some(JsonlWriter::open_guarded(
            &run_dir.join("population-ledger.jsonl"),
            append_mode,
            &output_guard,
        )?)
    } else {
        None
    };
    let mut concept_gene_ledger = if hybrid_mode {
        Some(JsonlWriter::open_guarded(
            &run_dir.join("concept-gene-ledger.jsonl"),
            append_mode,
            &output_guard,
        )?)
    } else {
        None
    };
    let mut information_ledger = if hybrid_mode {
        Some(JsonlWriter::open_guarded(
            &run_dir.join("information-ledger.jsonl"),
            append_mode,
            &output_guard,
        )?)
    } else {
        None
    };
    let mut stage_concept_ledger = if hybrid_mode {
        Some(JsonlWriter::open_guarded(
            &run_dir.join("stage-concept-ledger.jsonl"),
            append_mode,
            &output_guard,
        )?)
    } else {
        None
    };

    if !append_mode {
        write_memory_cards(&mut memory_ledger, &stage_registry, &run_id)?;
        write_research_cards(&mut research_ledger, &research_cards)?;
    }

    let mut stage_ledgers = resume_state.stage_ledgers.clone();
    let mut event_count = resume_state.event_count;
    let mut generation_scores = resume_state.generation_scores.clone();
    let mut stage_score_history = resume_state.stage_score_history.clone();
    let mut router_state = resume_state.router_state.clone();
    let mut degraded_router = resume_state.degraded_router;
    let mut previous_generation_ids = resume_state.previous_generation_ids.clone();

    let mut accepted_cards = Vec::new();
    let mut concepts = Vec::new();
    if hybrid_mode {
        let seed_cards = if research_cards.is_empty() {
            synthesize_information_cards(&stage_registry, &run_id)
        } else {
            research_cards.clone()
        };
        for card in seed_cards {
            accepted_cards.push(card.clone());
            if let Some(ref mut ledger) = information_ledger {
                ledger.write(&card)?;
            }
            let concept = concept_gene_from_card(&card);
            concepts.push(concept.clone());
            if let Some(ref mut ledger) = concept_gene_ledger {
                ledger.write(&concept)?;
            }
        }
    }

    if start_generation <= max_generations {
        if !append_mode {
            run_events.write(&run_event(
                &run_id,
                variant.as_str(),
                "run_start",
                "g0000",
                "run",
                None,
                None,
                "run",
                "jnoccio",
                "standard",
                "nominal",
                artifact_paths(&run_dir, &run_dir, &[], &[]),
                judge_block("jnoccio", "scripted-run-start", "standard", 256, 64),
                constant_score_block(0.0),
            ))?;
            event_count += 1;
        }

        for generation_index in start_generation..=max_generations {
            output_guard.check()?;
            let generation_id = format!("g{:04}", generation_index);
            run_events.write(&run_event(
                &run_id,
                variant.as_str(),
                "generation_start",
                &generation_id,
                &generation_id,
                previous_generation_id(generation_index),
                None,
                "generation",
                "jnoccio",
                "standard",
                "nominal",
                artifact_paths(&run_dir, &run_dir, &[], &[]),
                judge_block("jnoccio", "scripted-generation", "standard", 256, 64),
                constant_score_block(0.0),
            ))?;
            event_count += 1;

            let mode_counts = population_mode_counts(population.population_size);
            let mode_list = build_mode_list(&mode_counts);
            let mut generation_stage_scores = Vec::new();
            let mut generation_passes = 0usize;
            let mut generation_failures = 0usize;
            let mut generation_candidates = Vec::new();
            let mut used_parent_ids = BTreeSet::new();

            if hybrid_mode {
                let _ = (generation_index == 1)
                    || (generation_index % population.new_info_refresh == 0);
                let mut info_was_written = false;
                if generation_index == 1 || generation_index % population.new_info_refresh == 0 {
                    if let Some(ref mut ledger) = information_ledger {
                        for card in synthesize_additional_information_cards(
                            &stage_registry,
                            &generation_id,
                            &run_id,
                            1,
                        ) {
                            accepted_cards.push(card.clone());
                            ledger.write(&card)?;
                            let concept = concept_gene_from_card(&card);
                            concepts.push(concept.clone());
                            if let Some(ref mut concept_ledger) = concept_gene_ledger {
                                concept_ledger.write(&concept)?;
                            }
                        }
                    }
                    info_was_written = true;
                }
                if !info_was_written && accepted_cards.is_empty() {
                    let synthesized = synthesize_information_cards(&stage_registry, &run_id);
                    for card in synthesized {
                        accepted_cards.push(card.clone());
                        if let Some(ref mut ledger) = information_ledger {
                            ledger.write(&card)?;
                        }
                        let concept = concept_gene_from_card(&card);
                        concepts.push(concept.clone());
                        if let Some(ref mut concept_ledger) = concept_gene_ledger {
                            concept_ledger.write(&concept)?;
                        }
                    }
                }
            }

            for (stage_index, stage) in stage_registry.iter().enumerate() {
                output_guard.check()?;
                let stage_dir = run_dir.join("stages").join(&stage.stage_id);
                let generation_dir = stage_dir.join("generations").join(&generation_id);
                fs::create_dir_all(&generation_dir)?;
                let candidate_id = format!("{generation_id}-{}", stage.stage_id);
                let route =
                    route_for_variant(&variant, stage, live_config.enabled, jailgun_available);
                let research_refs = select_research_refs_for_stage(&research_cards, stage);
                let live_purpose = live_call_purpose(
                    &live_config,
                    stage,
                    generation_index,
                    stage_index,
                    stage_registry.len(),
                );
                let live_records = if let Some(purpose) = live_purpose {
                    let record = run_live_call(
                        &run_dir,
                        stage,
                        &route,
                        &generation_id,
                        &candidate_id,
                        &purpose,
                        &live_config,
                        &research_refs,
                        Some(&output_guard),
                    )?;
                    live_call_ledger.write(&record)?;
                    vec![record]
                } else {
                    Vec::new()
                };

                let scores = compute_scores(
                    variant.as_str(),
                    stage,
                    generation_index,
                    seed,
                    &route,
                    jailgun_available,
                );
                let score_breakdown =
                    compute_score_breakdown(&scores, stage, &live_records, &research_refs);
                let artifact_paths_block =
                    artifact_paths(&run_dir, &stage_dir, &stage.inputs, &stage.outputs);
                let judge = judge_block(
                    &route.judge_family,
                    &route.provenance,
                    &route.route_tier,
                    scores
                        .get("prompt_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize,
                    scores
                        .get("completion_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize,
                );

                let parent_ids = candidate_parent_ids(&previous_generation_ids, stage_index);
                used_parent_ids.extend(parent_ids.iter().cloned());

                let stage_variant = stage_variant_record(
                    &run_id,
                    variant.as_str(),
                    &generation_id,
                    &candidate_id,
                    stage,
                    &parent_ids,
                    &score_breakdown,
                    &research_refs,
                );
                stage_variant_ledger.write(&stage_variant)?;

                let stage_start = run_event(
                    &run_id,
                    variant.as_str(),
                    "stage_start",
                    &generation_id,
                    &stage.stage_id,
                    previous_generation_id(generation_index),
                    Some(stage.mutation_op.clone()),
                    &stage.family,
                    &route.route_backend,
                    &route.route_tier,
                    &route.router_state,
                    artifact_paths_block.clone(),
                    judge.clone(),
                    constant_score_block(0.0),
                );
                run_events.write(&stage_start)?;
                event_count += 1;

                let mut router_decision = stage_start.clone();
                router_decision["event_type"] = json!("router_decision");
                merge_object(&mut router_decision, &scores);
                run_events.write(&router_decision)?;
                event_count += 1;

                let mut candidate_event = stage_start.clone();
                candidate_event["event_type"] = json!("candidate_evaluated");
                merge_object(&mut candidate_event, &scores);
                run_events.write(&candidate_event)?;
                event_count += 1;

                let mut stage_end = stage_start.clone();
                stage_end["event_type"] = json!("stage_end");
                merge_object(&mut stage_end, &scores);
                run_events.write(&stage_end)?;
                event_count += 1;

                let final_score = scores
                    .get("final_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let delta_score =
                    final_score - generation_stage_scores.last().copied().unwrap_or(0.0);
                let pass_rate = if final_score >= 0.45 { 1.0 } else { 0.0 };
                if pass_rate < 1.0 {
                    generation_failures += 1;
                } else {
                    generation_passes += 1;
                }
                generation_stage_scores.push(final_score);
                stage_score_history
                    .entry(stage.stage_id.clone())
                    .or_default()
                    .push(final_score);
                let ledger_entry = stage_ledger_record(
                    &run_id,
                    variant.as_str(),
                    &generation_id,
                    &stage.stage_id,
                    &candidate_id,
                    &stage.name,
                    &route,
                    Some(stage.mutation_op.clone()),
                    previous_generation_id(generation_index),
                    delta_score,
                    pass_rate,
                    scores
                        .get("time_seconds")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                    scores
                        .get("failure_modes")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_else(Vec::new),
                    artifact_paths_block.clone(),
                    &scores,
                );
                stage_ledger.write(&ledger_entry)?;
                stage_ledgers.push(ledger_entry);
                event_count += 1;

                metrics_ledger.write(&metrics_point(
                    &run_id,
                    variant.as_str(),
                    &generation_id,
                    "stage_final_score",
                    final_score,
                    "stage",
                    Some(&stage.stage_id),
                    Some(&candidate_id),
                    json!({
                        "stage_variant_id": stage_variant["stage_variant_id"],
                        "island": stage.track,
                        "score_breakdown": score_breakdown,
                    }),
                ))?;

                if route.router_state != "nominal" {
                    degraded_router = true;
                    router_state = route.router_state.clone();
                }

                if hybrid_mode {
                    let stage_concepts = choose_stage_concepts(
                        &stage_registry,
                        &concepts,
                        generation_index,
                        stage_index + 1,
                        seed,
                    );
                    let candidate = hybrid_candidate_record(
                        &generation_id,
                        stage,
                        &candidate_id,
                        &parent_ids,
                        &mode_list[stage_index % mode_list.len()],
                        &research_refs,
                        &stage_concepts,
                        &route.route_policy,
                        &scores,
                        &score_breakdown,
                    );
                    generation_candidates.push(candidate.clone());
                    let promoted = promote_generation_candidates(&generation_candidates);
                    if let Some(ref mut ledger) = population_ledger {
                        ledger.write(&population_snapshot(
                            &run_id,
                            &generation_id,
                            population.population_size,
                            &population.island_names,
                            &mode_counts,
                            &generation_candidates,
                            &promoted,
                            candidate.get("candidate_id").unwrap_or(&json!("")),
                            final_score,
                            json!({
                                "concept_entropy": 2.5,
                                "island_balance": 0.75,
                                "source_diversity": 0.50,
                                "stage_concept_churn": 0.25,
                            }),
                        ))?;
                    }
                    if let Some(ref mut ledger) = stage_concept_ledger {
                        for (stage_id, concept_id) in stage_concepts {
                            ledger.write(&json!({
                                "schema_version": SCHEMA_VERSION,
                                "record_kind": "stage_concept",
                                "run_id": run_id,
                                "generation_id": generation_id,
                                "stage_id": stage_id,
                                "candidate_id": candidate_id,
                                "concept_id": concept_id,
                                "family": stage.family,
                                "source_card_ids": research_refs,
                                "mutation_op": stage.mutation_op,
                            }))?;
                        }
                    }
                    if let Some(ref mut ledger) = lineage_ledger {
                        for parent_id in &parent_ids {
                            ledger.write(&lineage_edge_record(
                                &run_id,
                                &generation_id,
                                json!(parent_id),
                                field_or(&candidate, "candidate_id", empty_string_json),
                                stage.mutation_op.clone(),
                                stage.track.clone(),
                            ))?;
                        }
                        if parent_ids.is_empty() {
                            ledger.write(&lineage_edge_record(
                                &run_id,
                                &generation_id,
                                Value::Null,
                                field_or(&candidate, "candidate_id", empty_string_json),
                                stage.mutation_op.clone(),
                                stage.track.clone(),
                            ))?;
                        }
                    }
                }
            }

            let generation_final = if generation_stage_scores.is_empty() {
                0.0
            } else {
                generation_stage_scores.iter().sum::<f64>() / generation_stage_scores.len() as f64
            };
            generation_scores.push(generation_final);
            if hybrid_mode {
                previous_generation_ids = generation_candidates
                    .iter()
                    .filter_map(|candidate| {
                        candidate
                            .get("candidate_id")
                            .and_then(Value::as_str)
                            .map(ToString::to_string)
                    })
                    .collect();
            }

            run_events.write(&run_event(
                &run_id,
                variant.as_str(),
                "generation_end",
                &generation_id,
                "generation",
                previous_generation_id(generation_index),
                Some("generation_rollup".to_string()),
                "generation",
                "jnoccio",
                "standard",
                &router_state,
                artifact_paths(&run_dir, &run_dir, &[], &[]),
                judge_block("jnoccio", "scripted-generation-end", "standard", 256, 64),
                constant_score_block(generation_final),
            ))?;
            event_count += 1;

            let generation_metric = metrics_point(
                &run_id,
                variant.as_str(),
                &generation_id,
                "deterministic_rollup_score",
                generation_final,
                "generation_rollup",
                None,
                Some(&generation_id),
                json!({
                    "stage_count": stage_registry.len(),
                    "stage_passes": generation_passes,
                    "stage_failures": generation_failures,
                    "best_stage_score": generation_stage_scores.iter().copied().fold(0.0, f64::max),
                    "mean_stage_score": generation_final,
                }),
            );
            generation_ledger.write(&generation_metric)?;
            metrics_ledger.write(&generation_metric)?;
            if checkpoint_every > 0
                && (generation_index % checkpoint_every == 0 || generation_index == max_generations)
            {
                output_guard.check()?;
                write_checkpoint(
                    &run_dir,
                    &run_id,
                    variant.as_str(),
                    generation_index,
                    max_generations,
                    Some(&output_guard),
                )?;
            }
        }

        if start_generation <= max_generations {
            run_events.write(&run_event(
                &run_id,
                variant.as_str(),
                "run_end",
                &format!("g{:04}", max_generations),
                "run",
                previous_generation_id(max_generations),
                None,
                "run",
                "jnoccio",
                "standard",
                &router_state,
                artifact_paths(&run_dir, &run_dir, &[], &[]),
                judge_block("jnoccio", "scripted-run-end", "standard", 256, 64),
                constant_score_block(*generation_scores.last().unwrap_or(&0.0)),
            ))?;
            event_count += 1;
        }
    }

    run_events.flush()?;
    stage_ledger.flush()?;
    generation_ledger.flush()?;
    metrics_ledger.flush()?;
    stage_variant_ledger.flush()?;
    live_call_ledger.flush()?;
    research_ledger.flush()?;
    memory_ledger.flush()?;
    promotion_ledger.flush()?;
    if let Some(ref mut writer) = lineage_ledger {
        writer.flush()?;
    }
    if let Some(ref mut writer) = population_ledger {
        writer.flush()?;
    }
    if let Some(ref mut writer) = concept_gene_ledger {
        writer.flush()?;
    }
    if let Some(ref mut writer) = information_ledger {
        writer.flush()?;
    }
    if let Some(ref mut writer) = stage_concept_ledger {
        writer.flush()?;
    }

    output_guard.check()?;
    let stage_summary_paths = write_stage_summaries(
        &run_dir,
        &stage_registry,
        &stage_ledgers,
        &stage_score_history,
        variant.as_str(),
        &run_id,
    )?;
    output_guard.check()?;
    let hybrid_evolution = if hybrid_mode {
        Some(emit_hybrid_evolution_artifacts(
            &run_dir,
            &run_id,
            &stage_registry,
            max_generations,
            seed,
            jailgun_available,
            &population,
            start_generation,
            &live_config,
            &research_cards,
        )?)
    } else {
        None
    };

    output_guard.check()?;
    let summary = build_run_summary(
        &run_id,
        variant.as_str(),
        &runbook,
        &runbook_path,
        max_generations,
        seed,
        dry_run,
        jailgun_available,
        &generation_scores,
        &stage_ledgers,
        &stage_registry,
        &router_state,
        degraded_router,
        &stage_summary_paths,
        &run_dir,
        hybrid_evolution.as_ref(),
        &population,
    );
    output_guard.check()?;
    write_json(&run_dir.join("run-summary.json"), &summary)?;
    output_guard.check()?;
    write_json(
        &run_dir.join("pareto-snapshot.json"),
        summary.get("pareto_snapshot").unwrap_or(&json!({})),
    )?;
    output_guard.check()?;
    let offline_eval = emit_run_offline_eval(&run_dir)?;
    output_guard.check()?;
    write_json(&run_dir.join("offline-eval.json"), &offline_eval)?;
    output_guard.check()?;
    write_markdown(
        &run_dir.join("offline-eval.md"),
        &render_run_markdown(&offline_eval),
    )?;
    output_guard.check()?;
    emit_run_plot_index(&run_dir)?;
    output_guard.check()?;
    write_checkpoint(
        &run_dir,
        &run_id,
        variant.as_str(),
        max_generations,
        max_generations,
        Some(&output_guard),
    )?;
    output_guard.check()?;
    touch_latest(&output_root, &run_dir)?;
    println!(
        "wrote genome run for {} to {}",
        variant.as_str(),
        run_dir.display()
    );
    Ok(())
}
