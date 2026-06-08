use super::*;

pub(crate) fn promote_generation_candidates(candidates: &[Value]) -> Vec<Value> {
    let mut ranked = candidates.to_vec();
    ranked.sort_by(|a, b| {
        candidate_score(b)
            .partial_cmp(&candidate_score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let target = if ranked.len() > 3 {
        ranked.len().min(6)
    } else {
        ranked.len()
    };
    let mut promoted = Vec::new();
    for mode in ["novelty", "wildcard", "exploitation"] {
        if let Some(candidate) = ranked
            .iter()
            .find(|candidate| candidate.get("mode").and_then(Value::as_str) == Some(mode))
        {
            push_unique_candidate(&mut promoted, candidate);
        }
    }
    let islands = ranked
        .iter()
        .filter_map(|candidate| candidate.get("island").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    for island in islands {
        if promoted.len() >= target {
            break;
        }
        if let Some(candidate) = ranked.iter().find(|candidate| {
            candidate.get("island").and_then(Value::as_str) == Some(island.as_str())
        }) {
            push_unique_candidate(&mut promoted, candidate);
        }
    }
    for candidate in &ranked {
        if promoted.len() >= target {
            break;
        }
        push_unique_candidate(&mut promoted, candidate);
    }
    promoted.sort_by(|a, b| {
        candidate_score(b)
            .partial_cmp(&candidate_score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    promoted
}

pub(crate) fn push_unique_candidate(promoted: &mut Vec<Value>, candidate: &Value) {
    let candidate_id = candidate
        .get("candidate_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !promoted.iter().any(|entry| {
        entry
            .get("candidate_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            == candidate_id
    }) {
        promoted.push(candidate.clone());
    }
}

pub(crate) fn candidate_score(candidate: &Value) -> f64 {
    candidate
        .get("scores")
        .and_then(|scores| scores.get("final_score"))
        .and_then(Value::as_f64)
        .or_else(|| candidate.get("final_score").and_then(Value::as_f64))
        .unwrap_or(0.0)
}

pub(crate) fn select_balanced_champion(
    generation_index: usize,
    promoted: &[Value],
    all_candidates: &[Value],
    previous_champions: &[Value],
    island_names: &[String],
    novelty_champion_rate_min: f64,
) -> ChampionSelection {
    let pool = if promoted.is_empty() {
        all_candidates
    } else {
        promoted
    };
    if pool.is_empty() {
        return ChampionSelection {
            champion: json!({}),
            reason: PromotionReason::TopScore,
        };
    }
    let rolling_start = previous_champions.len().saturating_sub(99);
    let rolling = &previous_champions[rolling_start..];
    let top = best_candidate(pool);
    if let Some(previous_elite) = previous_champions.last() {
        if candidate_score(&top) < candidate_score(previous_elite) - QUALITY_REGRESSION_TOLERANCE {
            return ChampionSelection {
                champion: retain_elite_champion(previous_elite, generation_index),
                reason: PromotionReason::EliteRetain,
            };
        }
    }
    let novelty_champions = rolling
        .iter()
        .filter(|champion| champion.get("mode").and_then(Value::as_str) == Some("novelty"))
        .count();
    let novelty_rate = if rolling.is_empty() {
        0.0
    } else {
        novelty_champions as f64 / rolling.len() as f64
    };
    if novelty_rate < novelty_champion_rate_min {
        if let Some(candidate) = best_candidate_matching(pool, "mode", "novelty") {
            let novelty_score = candidate_score(&candidate);
            let competitive_floor = NOVELTY_CHAMPION_SCORE_FLOOR
                .max(candidate_score(&top) - QUALITY_REGRESSION_TOLERANCE);
            if novelty_score >= competitive_floor {
                return ChampionSelection {
                    champion: candidate,
                    reason: PromotionReason::NoveltyQuota,
                };
            }
        }
    }
    let represented_islands = rolling
        .iter()
        .filter_map(|champion| champion.get("island").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    let target_island_coverage = island_names.len().min(4);
    if generation_index <= 100 && represented_islands.len() < target_island_coverage {
        for island in island_names {
            if !represented_islands.contains(island.as_str()) {
                if let Some(candidate) = best_candidate_matching(pool, "island", island) {
                    return ChampionSelection {
                        champion: candidate,
                        reason: PromotionReason::IslandBalance,
                    };
                }
            }
        }
    }
    let counts = count_string_field(rolling, "island");
    if let Some(top_island) = top.get("island").and_then(Value::as_str) {
        let projected_count = counts.get(top_island).copied().unwrap_or(0) + 1;
        let projected_share = projected_count as f64 / (rolling.len() + 1).max(1) as f64;
        if projected_share > 0.60 {
            if let Some(candidate) = pool
                .iter()
                .filter(|candidate| {
                    candidate.get("island").and_then(Value::as_str) != Some(top_island)
                })
                .max_by(|a, b| {
                    candidate_score(a)
                        .partial_cmp(&candidate_score(b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .cloned()
            {
                return ChampionSelection {
                    champion: candidate,
                    reason: PromotionReason::IslandCap,
                };
            }
        }
    }
    ChampionSelection {
        champion: top,
        reason: PromotionReason::TopScore,
    }
}

pub(crate) fn retain_elite_champion(previous_elite: &Value, generation_index: usize) -> Value {
    let mut elite = previous_elite.clone();
    let retained_from_generation_id = field_or(&elite, "generation_id", empty_string_json);
    elite["retained_from_generation_id"] = retained_from_generation_id;
    elite["selection_generation_id"] = json!(format!("g{generation_index:04}"));
    elite
}

pub(crate) fn best_candidate(candidates: &[Value]) -> Value {
    value_or(
        candidates
            .iter()
            .max_by(|a, b| {
                candidate_score(a)
                    .partial_cmp(&candidate_score(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned(),
        empty_object,
    )
}

pub(crate) fn best_candidate_matching(
    candidates: &[Value],
    field: &str,
    value: &str,
) -> Option<Value> {
    candidates
        .iter()
        .filter(|candidate| candidate.get(field).and_then(Value::as_str) == Some(value))
        .max_by(|a, b| {
            candidate_score(a)
                .partial_cmp(&candidate_score(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
}

pub(crate) fn all_candidates_extend(dst: &mut Vec<Value>, src: &[Value]) {
    dst.extend(src.iter().cloned());
}

pub(crate) fn all_candidates_from_generation(
    candidates: &[Value],
    generation_id: &str,
) -> Vec<Value> {
    candidates
        .iter()
        .filter(|candidate| {
            candidate.get("generation_id").and_then(Value::as_str) == Some(generation_id)
        })
        .cloned()
        .collect()
}
