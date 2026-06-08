use super::*;

pub(crate) fn compute_score_breakdown(
    scores: &Value,
    stage: &StagePackage,
    live_records: &[Value],
    research_refs: &[String],
) -> Value {
    let live_quality = if live_records.is_empty() {
        0.0
    } else {
        live_records
            .iter()
            .map(|record| {
                if record.get("status").and_then(Value::as_str) == Some("ok") {
                    0.70
                } else {
                    0.20
                }
            })
            .sum::<f64>()
            / live_records.len() as f64
    };
    let benchmark_safety = 1.0
        - scores
            .get("failure_penalty")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
    let mut penalties = Vec::new();
    if research_refs.is_empty() {
        penalties.push("missing_research_refs");
    }
    if memory_refs_for_stage(stage).is_empty() {
        penalties.push("missing_memory_refs");
    }
    if !stage.prompt_hash.is_empty() && stage.prompt_hash.len() < 8 {
        penalties.push("missing_prompt_hash");
    }
    json!({
        "artifact_validity": if stage.required_evidence.is_empty() { 0.70 } else { 0.95 },
        "source_grounding": (0.50 + 0.08 * research_refs.len() as f64).clamp(0.0, 1.0),
        "novelty": value_or(or_alt(scores.get("novelty_score"), scores.get("innovation_score")).cloned(), zero_f64_json),
        "stage_reusability": if stage.stage_dir.is_dir() { 0.92 } else { 0.60 },
        "interface_integrity": field_or(scores, "interface_score", zero_f64_json),
        "failure_understanding": value_or_default(
            scores.get("failure_understanding").cloned(),
            json!(1.0 - scores.get("failure_penalty").and_then(Value::as_f64).unwrap_or(0.0)),
        ),
        "live_reasoning_quality": live_quality,
        "benchmark_safety": benchmark_safety.max(0.0),
        "promotion_confidence": field_or(scores, "final_score", zero_f64_json),
        "penalties": penalties,
    })
}

pub(crate) fn population_mode_counts(population_size: usize) -> BTreeMap<String, usize> {
    let exploitation = population_size / 2;
    let novelty = population_size / 3;
    let wildcard = population_size.saturating_sub(exploitation + novelty);
    BTreeMap::from([
        ("exploitation".to_string(), exploitation),
        ("novelty".to_string(), novelty),
        ("wildcard".to_string(), wildcard),
    ])
}

pub(crate) fn build_mode_list(mode_counts: &BTreeMap<String, usize>) -> Vec<String> {
    let mut modes = Vec::new();
    for (mode, count) in mode_counts {
        for _ in 0..*count {
            modes.push(mode.clone());
        }
    }
    if modes.is_empty() {
        modes.extend(
            ["exploitation", "novelty", "wildcard"]
                .iter()
                .map(|s| s.to_string()),
        );
    }
    modes
}

pub(crate) fn route_for_variant(
    variant: &GenomeVariant,
    stage: &StagePackage,
    live_enabled: bool,
    jailgun_available: bool,
) -> RoutePolicy {
    let hard = stage.family == "hard";
    match variant {
        GenomeVariant::PureJnoccio => RoutePolicy {
            route_backend: "jnoccio".to_string(),
            route_tier: if hard {
                "top20_pct".to_string()
            } else {
                "standard".to_string()
            },
            router_state: "nominal".to_string(),
            judge_family: "jnoccio".to_string(),
            provenance: "scripted-route-policy".to_string(),
            route_policy: json!({"backend":"jnoccio","tier":"standard"}),
        },
        GenomeVariant::Hybrid => {
            if hard && live_enabled && jailgun_available {
                RoutePolicy {
                    route_backend: "jailgun".to_string(),
                    route_tier: if hard {
                        "top20_pct".to_string()
                    } else {
                        "standard".to_string()
                    },
                    router_state: "nominal".to_string(),
                    judge_family: "mixed".to_string(),
                    provenance: "backend-wrapper".to_string(),
                    route_policy: json!({"backend":"jailgun","tier":"top20_pct"}),
                }
            } else if hard {
                RoutePolicy {
                    route_backend: "jnoccio".to_string(),
                    route_tier: if hard {
                        "top20_pct".to_string()
                    } else {
                        "standard".to_string()
                    },
                    router_state: "degraded_router".to_string(),
                    judge_family: "mixed".to_string(),
                    provenance: "scripted-degraded".to_string(),
                    route_policy: json!({"backend":"jnoccio","tier":"top20_pct"}),
                }
            } else {
                RoutePolicy {
                    route_backend: "jnoccio".to_string(),
                    route_tier: "standard".to_string(),
                    router_state: "nominal".to_string(),
                    judge_family: "mixed".to_string(),
                    provenance: "scripted-route-policy".to_string(),
                    route_policy: json!({"backend":"jnoccio","tier":"standard"}),
                }
            }
        }
        GenomeVariant::JailgunOnly => RoutePolicy {
            route_backend: "jailgun".to_string(),
            route_tier: "manual".to_string(),
            router_state: if live_enabled {
                "nominal".to_string()
            } else {
                "stubbed_wrapper".to_string()
            },
            judge_family: "jailgun".to_string(),
            provenance: if live_enabled {
                "backend-wrapper".to_string()
            } else {
                "scripted-wrapper".to_string()
            },
            route_policy: json!({"backend":"jailgun","tier":"manual"}),
        },
    }
}

pub(crate) fn compute_scores(
    variant: &str,
    stage: &StagePackage,
    generation_index: usize,
    seed: u64,
    route: &RoutePolicy,
    jailgun_available: bool,
) -> Value {
    let hardness = if stage.family == "hard" { 0.72 } else { 0.28 };
    let jitter = hash_unit(&format!(
        "{variant}:{}:{generation_index}:{seed}",
        stage.stage_id
    ));
    let route_jitter = hash_unit(&format!(
        "{}:{}:{seed}",
        route.route_backend, stage.stage_id
    ));
    let generation_drift = 0.015 * (generation_index.saturating_sub(1).min(50) as f64 / 50.0);
    let variant_shift = match variant {
        "pure-jnoccio" => (0.02, 0.00, 0.01, 0.03, 0.00),
        "hybrid" => (0.01, 0.03, 0.04, 0.02, -0.01),
        _ => (0.02, 0.05, 0.03, 0.02, -0.02),
    };
    let route_bonus = if route.route_backend == "jailgun" {
        0.04
    } else {
        0.01
    };
    let local = clamp(
        0.59 + 0.10 * (1.0 - hardness) + generation_drift + variant_shift.0 + 0.04 * jitter,
        0.0,
        1.0,
    );
    let interface = clamp(
        0.57 + 0.11 * (1.0 - hardness) + variant_shift.1 + 0.03 * route_jitter,
        0.0,
        1.0,
    );
    let macro_score = clamp(
        0.58 + 0.12 * (1.0 - hardness) + generation_drift + variant_shift.2 + 0.04 * jitter,
        0.0,
        1.0,
    );
    let innovation = clamp(
        0.34 + 0.18 * hardness + variant_shift.3 + 0.05 * jitter,
        0.0,
        1.0,
    );
    let novelty = clamp(
        0.38 + 0.10 * hardness + 0.04 * jitter + if variant == "hybrid" { 0.03 } else { 0.0 },
        0.0,
        1.0,
    );
    let route_degradation_penalty = if route.router_state == "degraded_router" {
        0.16
    } else {
        0.0
    };
    let mut failure_penalty = clamp(
        0.03 + 0.08 * hardness
            + route_bonus
            + variant_shift.4
            + route_degradation_penalty
            + 0.02 * route_jitter,
        0.0,
        0.35,
    );
    if variant == "jailgun-only" && !jailgun_available {
        failure_penalty = clamp(failure_penalty + 0.06, 0.0, 0.35);
    }
    let final_score = if variant == "hybrid" {
        clamp(
            0.24 * local
                + 0.16 * interface
                + 0.24 * macro_score
                + 0.18 * innovation
                + 0.13 * novelty
                - 0.10 * failure_penalty,
            0.0,
            1.0,
        )
    } else {
        clamp(
            0.30 * local + 0.20 * interface + 0.30 * macro_score + 0.15 * innovation
                - 0.10 * failure_penalty,
            0.0,
            1.0,
        )
    };
    let final_score = apply_saturation_guard(
        final_score,
        &[local, interface, macro_score, innovation, novelty],
        failure_penalty,
        route.router_state == "nominal",
    );
    let prompt_tokens = 800
        + 110 * generation_index
        + (85.0 * hardness) as usize
        + 20 * stage.stage_id.len()
        + (75.0 * route_bonus * 10.0) as usize;
    let completion_tokens = 350
        + 60 * generation_index
        + (45.0 * hardness) as usize
        + 10 * stage.stage_id.len()
        + (45.0 * route_bonus * 10.0) as usize;
    let time_seconds = 1.5
        + 0.6 * hardness
        + 0.15 * generation_index as f64
        + if route.route_backend == "jailgun" {
            0.45
        } else {
            0.2
        };
    let mut failure_modes = Vec::new();
    if route.router_state != "nominal" {
        failure_modes.push("degraded_router".to_string());
    }
    if variant == "jailgun-only" && !jailgun_available {
        failure_modes.push("stubbed_jailgun".to_string());
    }
    if failure_penalty > 0.18 {
        failure_modes.push("high_failure_penalty".to_string());
    }
    json!({
        "local_score": round6(local),
        "interface_score": round6(interface),
        "macro_score": round6(macro_score),
        "innovation_score": round6(innovation),
        "novelty_score": round6(novelty),
        "failure_penalty": round6(failure_penalty),
        "final_score": round6(final_score),
        "prompt_tokens": prompt_tokens,
        "completion_tokens": completion_tokens,
        "time_seconds": round6(time_seconds),
        "pass_rate": if final_score >= 0.45 { 1.0 } else { 0.0 },
        "failure_modes": failure_modes,
        "failure_understanding": round6((1.0 - failure_penalty).clamp(0.0, 1.0)),
    })
}

pub(crate) fn apply_saturation_guard(
    final_score: f64,
    components: &[f64],
    failure_penalty: f64,
    route_nominal: bool,
) -> f64 {
    if final_score < 0.999999 {
        return final_score;
    }
    let components_clear = components.iter().all(|score| *score >= 0.95);
    if route_nominal && components_clear && failure_penalty <= 0.02 {
        final_score
    } else {
        0.995
    }
}

pub(crate) fn candidate_route_policy(
    mutation_op: &str,
    island: &str,
    jailgun_available: bool,
    degraded_router_penalty: f64,
) -> RoutePolicy {
    let route_backend = if island.contains("repair") || mutation_op.contains("repair") {
        "jailgun"
    } else {
        "jnoccio"
    };
    let router_state = if route_backend == "jailgun" && !jailgun_available {
        "degraded_router"
    } else {
        "nominal"
    };
    RoutePolicy {
        route_backend: route_backend.to_string(),
        route_tier: if route_backend == "jailgun" {
            "top20_pct".to_string()
        } else {
            "standard".to_string()
        },
        router_state: router_state.to_string(),
        judge_family: if route_backend == "jailgun" {
            "mixed".to_string()
        } else {
            "jnoccio".to_string()
        },
        provenance: if router_state == "degraded_router" {
            "scripted-degraded".to_string()
        } else {
            "backend-wrapper".to_string()
        },
        route_policy: json!({
            "backend": route_backend,
            "router_state": router_state,
            "degraded_router_penalty": if router_state == "degraded_router" { degraded_router_penalty } else { 0.0 },
        }),
    }
}

pub(crate) fn build_mode_counts_or_default(population_size: usize) -> BTreeMap<String, usize> {
    population_mode_counts(population_size)
}
