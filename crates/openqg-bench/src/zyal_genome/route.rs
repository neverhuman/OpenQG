//! Typed route tier (v3.0.0 M6 ZYAL hardening). The `route_tier` was a bare `String` that **drifted**
//! between the code (which emitted `"top20_pct"`) and the ZYAL runbooks / README (which say
//! `"top20_pct_only"`), and even within one `RoutePolicy` the `route_tier` field and the embedded
//! `route_policy` JSON could disagree. A typed enum with an alias table makes the canonical form
//! single-sourced and normalizes every historical spelling to one tier, so a downstream
//! review/compatibility/mutation decision can never be contaminated by a string mismatch.

/// The model-routing tier a stage is dispatched at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteTier {
    /// The most-capable model lane, reserved for hard stages.
    Top20Pct,
    /// The default lane for light stages.
    Standard,
    /// An operator-pinned lane (e.g. jailgun-only).
    Manual,
}

impl RouteTier {
    /// The canonical wire string the code emits and ledgers record.
    pub fn as_str(self) -> &'static str {
        match self {
            RouteTier::Top20Pct => "top20_pct",
            RouteTier::Standard => "standard",
            RouteTier::Manual => "manual",
        }
    }

    /// Parse a tier from any accepted alias (case/space-insensitive). This is the drift fix:
    /// `"top20_pct"` (code) and `"top20_pct_only"` (runbooks/README) resolve to the **same** tier.
    /// Returns `None` for an unrecognized string so a caller can decide whether to fail or default.
    pub fn parse(s: &str) -> Option<RouteTier> {
        match s.trim().to_ascii_lowercase().as_str() {
            "top20_pct" | "top20_pct_only" | "top20pct" | "top-20-pct" | "top20%" => {
                Some(RouteTier::Top20Pct)
            }
            "standard" | "nominal" => Some(RouteTier::Standard),
            "manual" => Some(RouteTier::Manual),
            _ => None,
        }
    }

    /// Parse with the alias table, defaulting an unknown string to [`RouteTier::Standard`].
    pub fn parse_or_standard(s: &str) -> RouteTier {
        Self::parse(s).unwrap_or(RouteTier::Standard)
    }

    /// The jnoccio-fusion **quality band** this tier requests, forwarded to the gateway via the
    /// `JEKKO_RUN_QUALITY_BAND` env var so the router constrains model selection to that win-rate
    /// percentile (see `~/jekko` `jankurai-runner/src/model_policy.rs`: `top10|top20|top50|...`).
    /// `Top20Pct` ⇒ `"top20"` (the top-20%-performing jnoccio models); the other tiers impose no
    /// band (`None`). This is what makes `route_tier` actually influence WHICH model runs, rather
    /// than only being recorded in the ledger.
    pub fn quality_band(self) -> Option<&'static str> {
        match self {
            RouteTier::Top20Pct => Some("top20"),
            RouteTier::Standard | RouteTier::Manual => None,
        }
    }

    /// True if two tier strings refer to the same tier under the alias table (the comparison a
    /// drift-safe equality check should use instead of `==` on raw strings).
    pub fn same_tier(a: &str, b: &str) -> bool {
        match (Self::parse(a), Self::parse(b)) {
            (Some(x), Some(y)) => x == y,
            _ => a.trim().eq_ignore_ascii_case(b.trim()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_normalize_the_historical_drift() {
        // The whole point: the code's "top20_pct" and the runbooks' "top20_pct_only" are one tier.
        assert_eq!(RouteTier::parse("top20_pct"), Some(RouteTier::Top20Pct));
        assert_eq!(RouteTier::parse("top20_pct_only"), Some(RouteTier::Top20Pct));
        assert_eq!(RouteTier::parse("TOP20_PCT_ONLY "), Some(RouteTier::Top20Pct));
        assert!(RouteTier::same_tier("top20_pct", "top20_pct_only"));
        assert!(!RouteTier::same_tier("top20_pct", "standard"));
    }

    #[test]
    fn canonical_roundtrip_and_default() {
        for t in [RouteTier::Top20Pct, RouteTier::Standard, RouteTier::Manual] {
            assert_eq!(RouteTier::parse(t.as_str()), Some(t));
        }
        assert_eq!(RouteTier::parse("bogus"), None);
        assert_eq!(RouteTier::parse_or_standard("bogus"), RouteTier::Standard);
    }
}
