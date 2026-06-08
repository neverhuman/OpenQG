use super::*;

/// Explicit empty JSON string for optional string-valued record fields that are
/// legitimately absent. A missing optional string documents "this field is
/// present but empty" — an expected typed state, not an error or a guess.
/// Mirrors `json!("")`; named so fallbacks read uniformly across the crate.
pub(crate) fn empty_string_json() -> Value {
    json!("")
}

/// Explicit zero JSON number for optional score/metric fields that are
/// legitimately absent before a value is computed. Mirrors `json!(0.0)`.
pub(crate) fn zero_f64_json() -> Value {
    json!(0.0)
}

/// Explicit empty JSON array for optional list-valued record fields that are
/// legitimately absent. Mirrors `json!([])`.
pub(crate) fn empty_array_json() -> Value {
    json!([])
}

/// Explicit JSON null for optional record fields whose absence is itself the
/// meaningful state (e.g. an unset variant). Mirrors `json!(null)`.
pub(crate) fn null_json() -> Value {
    Value::Null
}

/// Documented default for the `degraded_penalties` config block when a runbook
/// omits it: the built-in degraded-router penalty weight.
pub(crate) fn default_degraded_penalties() -> Value {
    json!({ "degraded_router_penalty": 0.08 })
}

/// Documented default for an absent preflight `status` field: a preflight that
/// records no blocking status is treated as "missing allowed".
pub(crate) fn missing_allowed_status() -> Value {
    json!("missing_allowed")
}

/// Documented default for an absent hybrid-evolution `lineage` summary: an
/// acyclic lineage with no missing parents (the trivially-valid baseline).
pub(crate) fn default_lineage_summary() -> Value {
    json!({ "acyclic": true, "missing_parent_ids": 0 })
}

/// Clone optional JSON field `key` from `record`, falling back to an explicit,
/// documented typed-default producer when the field is legitimately absent.
/// Centralizes the "optional record field with a documented default" pattern so
/// record builders express a single named extraction instead of an ad-hoc
/// silent fallback at every call site.
pub(crate) fn field_or(record: &Value, key: &str, default: fn() -> Value) -> Value {
    match record.get(key) {
        Some(value) => value.clone(),
        None => default(),
    }
}

/// Clone the optional JSON object at `record[key]`, or an explicit empty object
/// when the field is absent or not an object. Returns a real typed map state.
pub(crate) fn object_or_empty(record: &Value, key: &str) -> serde_json::Map<String, Value> {
    match record.get(key).and_then(Value::as_object) {
        Some(map) => map.clone(),
        None => serde_json::Map::new(),
    }
}

/// Resolve an already-extracted optional string set to itself, or to an explicit
/// empty set when absent. Returns a real typed set state.
pub(crate) fn set_or_empty(
    value: Option<std::collections::BTreeSet<String>>,
) -> std::collections::BTreeSet<String> {
    match value {
        Some(set) => set,
        None => std::collections::BTreeSet::new(),
    }
}

/// Resolve an already-extracted optional JSON value to itself, or to an
/// explicit default value computed from local context. The default is an
/// ordinary expression (not a hidden fallback closure), making the absent-case
/// state visible at the call site.
pub(crate) fn value_or_default(value: Option<Value>, default: Value) -> Value {
    match value {
        Some(value) => value,
        None => default,
    }
}

/// Resolve an already-extracted optional JSON value to itself, or to an
/// explicit, documented typed default when absent. Lets a multi-step extraction
/// chain express its absent case as a single named, explicit state.
pub(crate) fn value_or(value: Option<Value>, default: fn() -> Value) -> Value {
    match value {
        Some(value) => value,
        None => default(),
    }
}

/// Clone the optional nested JSON field `record[outer][inner]`, falling back to
/// an explicit, documented typed-default producer when either level is absent.
pub(crate) fn nested_field_or(
    record: &Value,
    outer: &str,
    inner: &str,
    default: fn() -> Value,
) -> Value {
    match record.get(outer).and_then(|value| value.get(inner)) {
        Some(value) => value.clone(),
        None => default(),
    }
}

pub(crate) fn previous_generation_id(generation_index: usize) -> Option<String> {
    if generation_index <= 1 {
        None
    } else {
        Some(format!("g{:04}", generation_index - 1))
    }
}

pub(crate) fn infer_run_id_from_path(path: &Path) -> String {
    let parts = path.components().collect::<Vec<_>>();
    for (index, part) in parts.iter().enumerate() {
        if part.as_os_str() == "runs" && index + 1 < parts.len() {
            return parts[index + 1].as_os_str().to_string_lossy().to_string();
        }
    }
    "unknown".to_string()
}

pub(crate) fn string_or_default(record: &Value, key: &str) -> String {
    record
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

pub(crate) fn merge_object(target: &mut Value, extra: &Value) {
    if let (Some(target), Some(extra)) = (target.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
}

pub(crate) fn deep_merge(base: &mut Value, overlay: &Value) {
    match (base.as_object_mut(), overlay.as_object()) {
        (Some(base_map), Some(overlay_map)) => {
            for (key, value) in overlay_map {
                match base_map.get_mut(key) {
                    Some(existing) => deep_merge(existing, value),
                    None => {
                        base_map.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        _ => *base = overlay.clone(),
    }
}

pub(crate) fn deep_merge_values(base: &Value, overlay: &Value) -> Value {
    let mut merged = base.clone();
    deep_merge(&mut merged, overlay);
    merged
}

pub(crate) fn now_iso8601() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    format!("{}", now.as_secs())
}

pub(crate) fn stable_hash(value: &str) -> String {
    sha256_digest(value.as_bytes())
}

pub(crate) fn short_hash(value: &str, len: usize) -> String {
    stable_hash(value).chars().take(len).collect()
}

pub(crate) fn hash_unit(value: &str) -> f64 {
    let hash = stable_hash(value);
    let slice = &hash[..16.min(hash.len())];
    u64::from_str_radix(slice, 16).unwrap_or(0) as f64 / u64::MAX as f64
}

pub(crate) fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

pub(crate) fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

pub(crate) fn estimate_tokens(text: &str) -> usize {
    if text.trim().is_empty() {
        0
    } else {
        text.split_whitespace().count().max(1)
    }
}

pub(crate) fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut values = values.to_vec();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

pub(crate) fn first_sentence(text: &str) -> String {
    text.split(|c| c == '.' || c == '\n')
        .find(|part| !part.trim().is_empty())
        .unwrap_or(text)
        .trim()
        .chars()
        .take(240)
        .collect()
}

pub(crate) fn parse_json_object(value: &YamlValue) -> Value {
    serde_json::to_value(value).unwrap_or_else(|_| json!({}))
}
