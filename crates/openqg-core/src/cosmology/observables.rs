//! One canonical observable-id grammar (`name` or `name@<z>`) shared by datasets,
//! novel-prediction witnesses, and the forward model — so a witness naming `fsigma8_z051`
//! can be checked against the model's `fsigma8@0.51` prediction.
//!
//! The canonicalizer folds the naming dialects seen in the wild into the forward model's
//! grammar (see [`super::BackgroundForwardModel`]):
//!
//! - witness suffix forms: `fsigma8_z051` / `fsigma8_z0p51` → `fsigma8@0.51`
//! - dataset prefixes and r_s/r_d aliasing: `bao_dv_over_rs_z038` → `dv_over_rd@0.38`
//! - scalar aliases: `S8`→`s8`, `H0`→`h0`, `h0_local`→`h0`, `yp`→`bbn_yp`,
//!   `cmb_r`→`cmb_R`, `cmb_la`→`cmb_lA`
//!
//! An id outside the grammar canonicalizes to `None` — "unverifiable", not an error — so
//! callers can degrade gracefully instead of crashing on a novel witness name.

/// The redshift-dependent observable families the forward model computes via the
/// `name@<z>` convention (`forward.rs` `parse_z` dispatch).
const AT_FAMILIES: [&str; 5] = ["fsigma8", "dv_over_rd", "dm_over_rd", "dh_over_rd", "mu"];

/// Two redshifts refer to the same observable when they agree to within this tolerance.
const Z_MATCH_TOLERANCE: f64 = 1e-9;

/// A parsed, canonical observable identity: either a scalar (`h0`, `s8`, `r_drag`, ...) or a
/// member of a redshift-indexed family (`fsigma8@0.51`, `dv_over_rd@0.38`, ...).
#[derive(Debug, Clone, PartialEq)]
pub enum CanonicalObservable {
    /// A redshift-free observable, stored under its canonical name
    /// (e.g. `"h0"`, `"s8"`, `"sigma8"`, `"r_drag"`, `"bbn_yp"`, `"cmb_R"`, `"cmb_lA"`).
    Scalar(String),
    /// A redshift-indexed observable: canonical family name plus the redshift it is
    /// evaluated at (e.g. `fsigma8` at z = 0.51).
    AtRedshift {
        /// Canonical family name: `fsigma8`, `dv_over_rd`, `dm_over_rd`, `dh_over_rd`, `mu`.
        name: String,
        /// Redshift the observable is evaluated at (finite, >= 0).
        z: f64,
    },
}

impl CanonicalObservable {
    /// Render the canonical id in the forward model's grammar, e.g. `"fsigma8@0.51"`.
    /// The redshift uses the minimal decimal rendering (trailing zeros and a trailing
    /// decimal point are trimmed: 0.510 → `"0.51"`, 2.0 → `"2"`), so the output
    /// round-trips through [`canonicalize_observable_id`] unchanged.
    pub fn to_id(&self) -> String {
        match self {
            CanonicalObservable::Scalar(name) => name.clone(),
            CanonicalObservable::AtRedshift { name, z } => format!("{name}@{}", format_z(*z)),
        }
    }
}

/// Minimal decimal rendering of a redshift: `f64`'s `Display` is already the shortest
/// round-tripping decimal, and the trim guards against any trailing `0`s / `.` (only applied
/// when a decimal point is present, so integer renderings like `"10"` are never shortened).
fn format_z(z: f64) -> String {
    let s = format!("{z}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// Fold the r_s/r_d naming dialects onto the forward model's family names
/// (`dv_over_rs` → `dv_over_rd`, etc.). Unknown names pass through unchanged.
fn map_family_alias(name: &str) -> &str {
    match name {
        "dv_over_rs" => "dv_over_rd",
        "dm_over_rs" => "dm_over_rd",
        "dh_over_rs" => "dh_over_rd",
        other => other,
    }
}

/// Resolve a (lowercased) scalar id to its canonical spelling, applying the alias table.
/// `cmb_R` / `cmb_lA` are the only mixed-case canonical ids; everything else is lowercase.
fn resolve_scalar(lower: &str) -> Option<&'static str> {
    match lower {
        "h0" | "h0_local" => Some("h0"),
        "omega_m" => Some("omega_m"),
        "sum_mnu" => Some("sum_mnu"),
        "n_eff" => Some("n_eff"),
        "omega_b_h2" => Some("omega_b_h2"),
        "r_drag" => Some("r_drag"),
        "bbn_yp" | "yp" => Some("bbn_yp"),
        "cmb_r" => Some("cmb_R"),
        "cmb_la" => Some("cmb_lA"),
        "cmb_omega_b_h2" => Some("cmb_omega_b_h2"),
        "s8" => Some("s8"),
        "sigma8" => Some("sigma8"),
        _ => None,
    }
}

/// Decode a witness-style redshift suffix (the part after `_z`).
///
/// Two encodings are accepted:
/// - digits only, where the first digit is the integer part and the rest are the decimals:
///   `051` → 0.51, `038` → 0.38, `148` → 1.48, `2` → 2.0, `0067` → 0.067;
/// - an explicit decimal point spelled `p`: `0p51` → 0.51 (digits required on both sides).
fn decode_z_suffix(suffix: &str) -> Option<f64> {
    if suffix.is_empty() {
        return None;
    }
    let decimal = if let Some((int_part, frac_part)) = suffix.split_once('p') {
        if int_part.is_empty()
            || frac_part.is_empty()
            || !int_part.bytes().all(|b| b.is_ascii_digit())
            || !frac_part.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        format!("{int_part}.{frac_part}")
    } else {
        if !suffix.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let (head, tail) = suffix.split_at(1);
        if tail.is_empty() {
            head.to_string()
        } else {
            format!("{head}.{tail}")
        }
    };
    // Digits-and-one-dot strings always parse to a finite, non-negative f64.
    decimal.parse::<f64>().ok()
}

/// Canonicalize a raw observable id from any supported dialect (forward-model `name@<z>`,
/// witness `name_z051` / `name_z0p51`, dataset `bao_*` / `*_over_rs`, scalar aliases).
///
/// `None` means the id is not a recognized observable grammar — it is treated as
/// "unverifiable" by callers, not as an error.
pub fn canonicalize_observable_id(raw: &str) -> Option<CanonicalObservable> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Rule 1: case-normalize. Lowercasing folds `S8`→`s8`, `H0`→`h0`, `CMB_R`→`cmb_r`;
    // `resolve_scalar` then restores the two mixed-case canonical ids (`cmb_R`, `cmb_lA`).
    let lower = trimmed.to_ascii_lowercase();
    // Rule 2: strip a dataset-style `bao_` prefix (the rs→rd family aliasing is applied
    // where the family name is extracted, in rules 3 and 4 below).
    let id = lower.strip_prefix("bao_").unwrap_or(&lower);

    // Rule 3: explicit `name@<z>` grammar.
    if let Some((name, z_str)) = id.split_once('@') {
        let family = map_family_alias(name);
        if !AT_FAMILIES.contains(&family) {
            return None;
        }
        let z = z_str.trim().parse::<f64>().ok()?;
        if !z.is_finite() || z < 0.0 {
            return None;
        }
        return Some(CanonicalObservable::AtRedshift {
            name: family.to_string(),
            z,
        });
    }

    // Rule 4: witness-style `<name>_z<digits>` / `<name>_z<d>p<d>` suffix.
    if let Some((name, z_suffix)) = id.rsplit_once("_z") {
        let family = map_family_alias(name);
        if AT_FAMILIES.contains(&family) {
            if let Some(z) = decode_z_suffix(z_suffix) {
                return Some(CanonicalObservable::AtRedshift {
                    name: family.to_string(),
                    z,
                });
            }
            // A known family with an undecodable suffix is not a scalar either.
            return None;
        }
        // Unknown family: fall through to the scalar table (no known scalar contains
        // `_z`, but the fallthrough keeps the rules orthogonal).
    }

    // Rule 5: known scalar ids (with aliases). Rule 6: anything else is None.
    resolve_scalar(id).map(|canonical| CanonicalObservable::Scalar(canonical.to_string()))
}

/// True iff both ids canonicalize and refer to the same observable: identical canonical
/// scalar names, or the same redshift family with |z_a − z_b| ≤ 1e-9.
///
/// Unrecognized ids never match anything (including other unrecognized ids).
pub fn observables_match(a: &str, b: &str) -> bool {
    match (canonicalize_observable_id(a), canonicalize_observable_id(b)) {
        (Some(CanonicalObservable::Scalar(na)), Some(CanonicalObservable::Scalar(nb))) => na == nb,
        (
            Some(CanonicalObservable::AtRedshift { name: na, z: za }),
            Some(CanonicalObservable::AtRedshift { name: nb, z: zb }),
        ) => na == nb && (za - zb).abs() <= Z_MATCH_TOLERANCE,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: canonicalize and render, panicking with the raw id on failure.
    fn canon(raw: &str) -> String {
        canonicalize_observable_id(raw)
            .unwrap_or_else(|| panic!("expected {raw:?} to canonicalize"))
            .to_id()
    }

    #[test]
    fn normalizer_table() {
        // (raw input, expected canonical id)
        let table = [
            // Witness `_z<digits>` suffix forms.
            ("fsigma8_z051", "fsigma8@0.51"),
            ("fsigma8_z038", "fsigma8@0.38"),
            ("fsigma8_z148", "fsigma8@1.48"),
            ("fsigma8_z2", "fsigma8@2"),
            ("fsigma8_z0067", "fsigma8@0.067"),
            ("fsigma8_z10", "fsigma8@1"),
            ("dv_over_rd_z038", "dv_over_rd@0.38"),
            // Witness `p`-as-decimal-point form.
            ("fsigma8_z0p51", "fsigma8@0.51"),
            ("mu_z1p5", "mu@1.5"),
            // Dataset `bao_` prefix + rs→rd aliasing.
            ("bao_dv_over_rs_z038", "dv_over_rd@0.38"),
            ("bao_dm_over_rs_z051", "dm_over_rd@0.51"),
            ("bao_dh_over_rs@0.51", "dh_over_rd@0.51"),
            ("bao_dv_over_rd_z038", "dv_over_rd@0.38"),
            ("dh_over_rs_z061", "dh_over_rd@0.61"),
            // Forward-model `name@<z>` grammar passes through.
            ("fsigma8@0.51", "fsigma8@0.51"),
            ("mu@0.5", "mu@0.5"),
            ("dm_over_rd@1.48", "dm_over_rd@1.48"),
            // Scalar aliases.
            ("S8", "s8"),
            ("H0", "h0"),
            ("h0_local", "h0"),
            ("yp", "bbn_yp"),
            ("cmb_r", "cmb_R"),
            ("cmb_la", "cmb_lA"),
            ("CMB_LA", "cmb_lA"),
            // Whitespace is trimmed.
            ("  h0  ", "h0"),
            (" fsigma8@0.51 ", "fsigma8@0.51"),
        ];
        for (raw, expected) in table {
            assert_eq!(canon(raw), expected, "raw id: {raw:?}");
        }
    }

    #[test]
    fn known_scalars_canonicalize_to_themselves() {
        for id in [
            "h0",
            "omega_m",
            "sum_mnu",
            "n_eff",
            "omega_b_h2",
            "r_drag",
            "bbn_yp",
            "cmb_R",
            "cmb_lA",
            "cmb_omega_b_h2",
            "s8",
            "sigma8",
        ] {
            assert_eq!(
                canonicalize_observable_id(id),
                Some(CanonicalObservable::Scalar(id.to_string())),
                "scalar id: {id:?}"
            );
        }
    }

    #[test]
    fn unrecognized_ids_are_none_not_errors() {
        for raw in [
            // Unknown @-family: the C_ℓ band powers are the Boltzmann backend's job and are
            // deliberately outside this grammar.
            "cl_tt@220",
            "cl_tt_z051",
            "not_an_observable",
            "",
            "   ",
            "fsigma8",      // known family but no redshift
            "dv_over_rd",   // known family but no redshift
            "fsigma8_z",    // empty redshift suffix
            "fsigma8_zabc", // non-digit suffix
            "fsigma8_z0p",  // `p` with no fractional digits
            "fsigma8_zp5",  // `p` with no integer digits
            "fsigma8@",     // empty redshift
            "fsigma8@abc",  // unparseable redshift
            "fsigma8@-0.5", // negative redshift
            "fsigma8@nan",  // non-finite redshift
            "fsigma8@inf",  // non-finite redshift
            "@0.51",        // missing name
            "bao_",         // prefix only
        ] {
            assert_eq!(canonicalize_observable_id(raw), None, "raw id: {raw:?}");
        }
    }

    #[test]
    fn redshift_decoding_parses_into_at_redshift_variant() {
        match canonicalize_observable_id("fsigma8_z148") {
            Some(CanonicalObservable::AtRedshift { name, z }) => {
                assert_eq!(name, "fsigma8");
                assert!((z - 1.48).abs() < 1e-12, "z = {z}");
            }
            other => panic!("expected AtRedshift, got {other:?}"),
        }
        match canonicalize_observable_id("fsigma8_z0067") {
            Some(CanonicalObservable::AtRedshift { name, z }) => {
                assert_eq!(name, "fsigma8");
                assert!((z - 0.067).abs() < 1e-12, "z = {z}");
            }
            other => panic!("expected AtRedshift, got {other:?}"),
        }
    }

    #[test]
    fn matching_is_alias_and_rendering_insensitive() {
        // Trailing-zero rendering differences match.
        assert!(observables_match("fsigma8@0.510", "fsigma8@0.51"));
        // Witness id vs forward-model id.
        assert!(observables_match("fsigma8_z051", "fsigma8@0.51"));
        assert!(observables_match("fsigma8_z0p51", "fsigma8@0.51"));
        // Dataset id vs forward-model id.
        assert!(observables_match("bao_dv_over_rs_z038", "dv_over_rd@0.38"));
        // Scalar aliases.
        assert!(observables_match("S8", "s8"));
        assert!(observables_match("h0_local", "H0"));
        assert!(observables_match("yp", "bbn_yp"));
        assert!(observables_match("cmb_r", "cmb_R"));
    }

    #[test]
    fn matching_rejects_different_observables() {
        // Different scalars.
        assert!(!observables_match("h0", "s8"));
        // Same family, different redshift (beyond tolerance).
        assert!(!observables_match("fsigma8@0.51", "fsigma8@0.52"));
        // Different families at the same redshift.
        assert!(!observables_match("dv_over_rd@0.51", "dm_over_rd@0.51"));
        // Scalar vs redshifted never match.
        assert!(!observables_match("h0", "fsigma8@0.5"));
        // Unrecognized ids never match anything — not even themselves.
        assert!(!observables_match("not_an_observable", "not_an_observable"));
        assert!(!observables_match("cl_tt@220", "cl_tt@220"));
        assert!(!observables_match("", "h0"));
    }

    #[test]
    fn to_id_uses_minimal_redshift_rendering() {
        let cases = [
            (2.0, "fsigma8@2"),
            (0.51, "fsigma8@0.51"),
            (0.067, "fsigma8@0.067"),
            (1.48, "fsigma8@1.48"),
            (0.0, "fsigma8@0"),
            (10.0, "fsigma8@10"),
        ];
        for (z, expected) in cases {
            let obs = CanonicalObservable::AtRedshift {
                name: "fsigma8".to_string(),
                z,
            };
            assert_eq!(obs.to_id(), expected, "z = {z}");
        }
        assert_eq!(
            CanonicalObservable::Scalar("cmb_R".to_string()).to_id(),
            "cmb_R"
        );
    }

    #[test]
    fn to_id_then_canonicalize_is_stable() {
        let raws = [
            "fsigma8_z051",
            "fsigma8_z0067",
            "fsigma8_z2",
            "fsigma8_z0p51",
            "bao_dv_over_rs_z038",
            "dm_over_rd@1.48",
            "mu@0.5",
            "S8",
            "h0_local",
            "yp",
            "cmb_r",
            "cmb_la",
            "sigma8",
            "cmb_omega_b_h2",
        ];
        for raw in raws {
            let first = canonicalize_observable_id(raw)
                .unwrap_or_else(|| panic!("expected {raw:?} to canonicalize"));
            let rendered = first.to_id();
            let second = canonicalize_observable_id(&rendered).unwrap_or_else(|| {
                panic!("canonical id {rendered:?} (from {raw:?}) must re-canonicalize")
            });
            assert_eq!(
                first, second,
                "roundtrip drifted for {raw:?} -> {rendered:?}"
            );
            assert_eq!(second.to_id(), rendered, "rendering drifted for {raw:?}");
        }
    }
}
