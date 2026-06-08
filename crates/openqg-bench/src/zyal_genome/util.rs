use super::*;

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
