//! ClaimGraph + UnificationClaim (V4 M2).
//!
//! V3 let a theory *assert* a result without ever tying that assertion to a checkable obligation,
//! and let "unification" mean nothing more than "the same model file mentions two sectors". V4 makes
//! both structural:
//!
//! * A [`ClaimGraph`] is a DAG of [`Claim`]s. Each claim names a [`Sector`], a [`ClaimKind`]
//!   (a falsifiable *physics* claim vs. a mere *engineering* claim about the pipeline), the
//!   content-bound [`EvidenceRef`]s it rests on, the obligations it must discharge, and the claims it
//!   depends on. The graph is validated ([`ClaimGraph::validate_dag`]): a dangling dependency or a
//!   cycle is a hard error, and the canonical [`ClaimGraph::digest`] is order-independent so two runs
//!   that emit the same logical graph hash identically regardless of claim ordering.
//!
//! * A V4 scorecard kills any *physics* claim that maps to zero obligations
//!   ([`ClaimGraph::unobligated_physics_claims`]): a physics assertion with nothing to discharge is
//!   prose, not a claim.
//!
//! * A [`UnificationClaim`] declares the [`SharedParam`]s that are supposed to drive *multiple*
//!   sectors at once. [`UnificationClaim::shared_parameter_audit`] checks each shared parameter is a
//!   real theory parameter (value-matched) that genuinely spans ≥2 sectors, and
//!   [`UnificationClaim::no_hidden_knob_test`] checks the theory hides no sector-private free knob
//!   outside the shared set — the structural difference between a *unified* theory and a merely
//!   *self-consistent* one that tunes a separate parameter per sector.
//!
//! This module is pure and deterministic: it reasons over [`super::Theory`] structure and content
//! hashes only, so the whole contract is unit-testable in-memory.

use super::evidence::EvidenceRef;
use super::Provenance;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// The physical sector a claim (or shared parameter) speaks to. A genuine unification claim must
/// touch ≥2 distinct sectors with the *same* parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sector {
    /// Background expansion history H(z).
    Background,
    /// Linear growth of structure (fσ8, RSD).
    Growth,
    /// Tensor sector / GW propagation (α_T, GW friction).
    TensorSector,
    /// Screening + post-Newtonian (PPN) solar-system recovery.
    ScreeningPpn,
    /// Big-bang nucleosynthesis.
    Bbn,
    /// Particle-physics sector.
    Particle,
    /// Quantum sector.
    Quantum,
}

/// Whether a claim is a falsifiable *physics* claim or a book-keeping *engineering* claim about the
/// discovery pipeline. Only physics claims carry the "must map to ≥1 obligation" scorecard rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    /// A falsifiable statement about the physical world / the theory's predictions.
    Physics,
    /// A statement about the pipeline / tooling (reproducibility, packaging, etc.).
    Engineering,
}

/// A claim's stable identifier.
pub type ClaimId = String;

/// One node in the [`ClaimGraph`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    /// Stable id, unique within the graph.
    pub id: ClaimId,
    /// The physical sector this claim addresses.
    pub sector: Sector,
    /// Physics vs. engineering.
    pub kind: ClaimKind,
    /// Human-readable statement of the claim.
    pub statement: String,
    /// Content-bound evidence the claim rests on.
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
    /// Obligation ids this claim must discharge. A physics claim with none is killed by the scorecard.
    #[serde(default)]
    pub obligations: Vec<String>,
    /// Ids of claims this claim depends on (edges of the DAG, child → parents).
    #[serde(default)]
    pub depends_on: Vec<ClaimId>,
}

/// A directed acyclic graph of [`Claim`]s.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimGraph {
    pub claims: Vec<Claim>,
}

impl ClaimGraph {
    /// Validate that the graph is a well-formed DAG: every `depends_on` target is a known claim id,
    /// and there are no cycles. Returns `Err` with a human-readable reason otherwise.
    pub fn validate_dag(&self) -> Result<(), String> {
        let ids: HashSet<&str> = self.claims.iter().map(|c| c.id.as_str()).collect();
        // Duplicate ids make dependency resolution ambiguous; reject them up front.
        if ids.len() != self.claims.len() {
            return Err("duplicate claim id in graph".into());
        }
        for c in &self.claims {
            for dep in &c.depends_on {
                if !ids.contains(dep.as_str()) {
                    return Err(format!(
                        "claim '{}' depends on unknown claim id '{}'",
                        c.id, dep
                    ));
                }
            }
        }
        // Cycle detection via DFS coloring (white/gray/black). A back-edge to a gray node is a cycle.
        self.topo_order().map(|_| ())
    }

    /// Return a topological ordering of the claim ids (parents before dependents), or `Err` on a
    /// cycle. Deterministic: claims are visited in id-sorted order so the output is stable.
    pub fn topo_order(&self) -> Result<Vec<ClaimId>, String> {
        // Adjacency: claim -> its dependencies (the claims that must come first).
        let mut deps: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for c in &self.claims {
            deps.entry(c.id.as_str()).or_default();
        }
        for c in &self.claims {
            for d in &c.depends_on {
                if !deps.contains_key(d.as_str()) {
                    return Err(format!(
                        "claim '{}' depends on unknown claim id '{}'",
                        c.id, d
                    ));
                }
                deps.get_mut(c.id.as_str()).unwrap().push(d.as_str());
            }
        }

        #[derive(Clone, Copy, PartialEq)]
        enum Color {
            White,
            Gray,
            Black,
        }
        let mut color: BTreeMap<&str, Color> = deps.keys().map(|&k| (k, Color::White)).collect();
        let mut order: Vec<ClaimId> = Vec::with_capacity(self.claims.len());

        // Iterative DFS post-order with an explicit stack so deep graphs don't blow the call stack.
        // Visit roots in sorted order for determinism.
        let roots: Vec<&str> = deps.keys().copied().collect();
        for &root in &roots {
            if color[root] != Color::White {
                continue;
            }
            // Stack of (node, entered?) — entered=false means "pre-visit", true means "post-visit".
            let mut stack: Vec<(&str, bool)> = vec![(root, false)];
            while let Some((node, entered)) = stack.pop() {
                if entered {
                    *color.get_mut(node).unwrap() = Color::Black;
                    order.push(node.to_string());
                    continue;
                }
                match color[node] {
                    Color::Black => continue,
                    Color::Gray => {
                        // Reached a node already on the active path → back-edge → cycle.
                        return Err(format!("cycle detected at claim '{}'", node));
                    }
                    Color::White => {}
                }
                *color.get_mut(node).unwrap() = Color::Gray;
                stack.push((node, true));
                // Push dependencies (sorted desc so they pop in ascending, deterministic order).
                let mut children = deps[node].clone();
                children.sort_unstable();
                for child in children.into_iter().rev() {
                    match color[child] {
                        Color::Gray => {
                            return Err(format!(
                                "cycle detected: '{}' depends on active '{}'",
                                node, child
                            ));
                        }
                        Color::Black => {}
                        Color::White => stack.push((child, false)),
                    }
                }
            }
        }
        Ok(order)
    }

    /// All physics claims (`kind == Physics`).
    pub fn physics_claims(&self) -> Vec<&Claim> {
        self.claims
            .iter()
            .filter(|c| c.kind == ClaimKind::Physics)
            .collect()
    }

    /// Physics claims that discharge *no* obligation. The V4 scorecard kills these: every physics
    /// claim must map to ≥1 obligation, or it is prose, not a claim.
    pub fn unobligated_physics_claims(&self) -> Vec<&Claim> {
        self.claims
            .iter()
            .filter(|c| c.kind == ClaimKind::Physics && c.obligations.is_empty())
            .collect()
    }

    /// Canonical, order-independent SHA-256 digest of the graph. Claims are rendered in id-sorted
    /// order; within a claim, evidence (path+sha), obligations, and dependencies are each sorted, so
    /// shuffling the input `claims` (or any of those vecs) yields the same digest, while changing any
    /// statement/sector/kind/evidence/obligation/dependency changes it.
    pub fn digest(&self) -> String {
        let mut sorted: Vec<&Claim> = self.claims.iter().collect();
        sorted.sort_by(|a, b| a.id.cmp(&b.id));

        let mut buf = String::new();
        buf.push_str("claim_graph.v1\n");
        for c in sorted {
            buf.push_str("claim\x1f");
            buf.push_str(&c.id);
            buf.push('\x1f');
            // Sector/kind via their snake_case serde rendering (stable, no quotes needed here).
            buf.push_str(&serde_json::to_string(&c.sector).unwrap_or_default());
            buf.push('\x1f');
            buf.push_str(&serde_json::to_string(&c.kind).unwrap_or_default());
            buf.push('\x1f');
            buf.push_str(&c.statement);
            buf.push('\n');

            // Evidence: sorted by (path, sha256), order-independent.
            let mut ev: Vec<(&str, &str)> = c
                .evidence
                .iter()
                .map(|e| (e.path.as_str(), e.sha256.as_str()))
                .collect();
            ev.sort_unstable();
            for (path, sha) in ev {
                buf.push_str("  ev\x1f");
                buf.push_str(path);
                buf.push('\x1f');
                buf.push_str(sha);
                buf.push('\n');
            }

            // Obligations: sorted, order-independent.
            let mut obl: Vec<&str> = c.obligations.iter().map(|s| s.as_str()).collect();
            obl.sort_unstable();
            for o in obl {
                buf.push_str("  obl\x1f");
                buf.push_str(o);
                buf.push('\n');
            }

            // Dependencies: sorted, order-independent.
            let mut deps: Vec<&str> = c.depends_on.iter().map(|s| s.as_str()).collect();
            deps.sort_unstable();
            for d in deps {
                buf.push_str("  dep\x1f");
                buf.push_str(d);
                buf.push('\n');
            }
        }
        crate::sha256_digest(buf.as_bytes())
    }
}

/// A parameter that a [`UnificationClaim`] asserts is shared across multiple physical sectors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SharedParam {
    /// The parameter symbol; must name a real [`super::Parameter`] in the theory.
    pub symbol: String,
    /// The value claimed (must match the theory parameter within 1e-9).
    pub value: f64,
    /// The sectors this single parameter drives. A genuine sharing needs ≥2 distinct sectors.
    pub sectors: Vec<Sector>,
}

/// A claim that a set of parameters is *shared* across sectors — the structural meaning of
/// "unification" in V4. Empty `shared` ⇒ no unification is being claimed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnificationClaim {
    pub shared: Vec<SharedParam>,
}

impl UnificationClaim {
    /// True iff *every* declared [`SharedParam`] (a) names a symbol present in `theory.parameters`
    /// whose value matches within 1e-9, and (b) declares ≥2 sectors. Empty `shared` ⇒ false
    /// (nothing is shared, so there is no unification claim to honor).
    pub fn shared_parameter_audit(&self, theory: &super::Theory) -> bool {
        if self.shared.is_empty() {
            return false;
        }
        self.shared.iter().all(|sp| {
            if sp.sectors.len() < 2 {
                return false;
            }
            // A symbol must drive ≥2 *distinct* sectors, not the same sector twice.
            let distinct: BTreeSet<Sector> = sp.sectors.iter().copied().collect();
            if distinct.len() < 2 {
                return false;
            }
            let param_ok = theory
                .parameters
                .iter()
                .any(|p| p.symbol == sp.symbol && (p.value - sp.value).abs() <= 1e-9);
            if !param_ok {
                return false;
            }
            // V6.1 (P0.5): a shared symbol that SHADOWS a background coordinate must agree with
            // the background the model actually integrates. The V6-campaign champions wore a
            // decorative H0=67.4 scaffold parameter over a fitted h=0.701 background — that
            // contradiction earned 15/15 unification. No longer.
            background_shadow_consistent(theory, &sp.symbol, sp.value)
        })
    }

    /// True iff the theory hides no sector-private free knob outside the shared set. A genuine
    /// unification has no free degree of freedom that is *not* declared shared: every theory
    /// parameter that is a real free knob — `Provenance::Free`, or `Provenance::Derived` without a
    /// certificate (an uncertified, hence unpinned, value) — must appear among the shared symbols.
    /// `Fundamental` and certified-`Derived` parameters are pinned by the theory, not knobs.
    pub fn no_hidden_knob_test(&self, theory: &super::Theory) -> bool {
        let shared_symbols: BTreeSet<&str> =
            self.shared.iter().map(|s| s.symbol.as_str()).collect();
        for p in &theory.parameters {
            let is_knob = match &p.provenance {
                Provenance::Free => true,
                Provenance::Derived { certificate, .. } => certificate.is_none(),
                Provenance::Fundamental => false,
            };
            if is_knob && !shared_symbols.contains(p.symbol.as_str()) {
                // A free/uncertified knob lives outside the shared set → hidden per-sector knob.
                return false;
            }
        }
        // V6.1 (P0.5): a drifted background coordinate is a fitted dial; when unification is
        // claimed, undeclared background drift is a hidden knob exactly like an uncertified
        // parameter (the alias table mirrors scorecard::background_dof).
        if super::scorecard::background_dof(theory) > 0 {
            return false;
        }
        true
    }
}

/// True when `symbol` either names no background coordinate, or names one whose current value
/// agrees with the declared parameter value (alias table: H0 = 100·h, Omega_m, w0, wa, sigma8).
fn background_shadow_consistent(theory: &super::Theory, symbol: &str, value: f64) -> bool {
    let bg = &theory.background;
    let pairs: [(&str, f64, f64); 5] = [
        ("H0", bg.h, 100.0),
        ("Omega_m", bg.omega_m, 1.0),
        ("w0", bg.w0, 1.0),
        ("wa", bg.wa, 1.0),
        ("sigma8", bg.sigma8, 1.0),
    ];
    for (sym, cur, scale) in pairs {
        if sym == symbol {
            return (value - cur * scale).abs() <= 1e-6 * scale.max(1.0);
        }
    }
    true
}

/// Thin wrapper over [`ClaimGraph::digest`] for callers that prefer a free function.
pub fn claim_graph_digest(cg: &ClaimGraph) -> String {
    cg.digest()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::{Parameter, Provenance, Theory};

    fn claim(id: &str, kind: ClaimKind, deps: &[&str], obligations: &[&str]) -> Claim {
        Claim {
            id: id.into(),
            sector: Sector::Background,
            kind,
            statement: format!("statement for {id}"),
            evidence: Vec::new(),
            obligations: obligations.iter().map(|s| s.to_string()).collect(),
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn valid_graph() -> ClaimGraph {
        ClaimGraph {
            claims: vec![
                claim("a", ClaimKind::Physics, &[], &["obl-a"]),
                claim("b", ClaimKind::Physics, &["a"], &["obl-b"]),
                claim("c", ClaimKind::Engineering, &["a", "b"], &[]),
            ],
        }
    }

    #[test]
    fn valid_dag_validates_and_topo_orders() {
        let g = valid_graph();
        assert!(g.validate_dag().is_ok());
        let order = g.topo_order().expect("topo");
        assert_eq!(order.len(), 3);
        // Parents must precede dependents.
        let pos = |id: &str| order.iter().position(|x| x == id).unwrap();
        assert!(pos("a") < pos("b"));
        assert!(pos("a") < pos("c"));
        assert!(pos("b") < pos("c"));
    }

    #[test]
    fn cyclic_graph_errors() {
        let g = ClaimGraph {
            claims: vec![
                claim("a", ClaimKind::Physics, &["b"], &["o"]),
                claim("b", ClaimKind::Physics, &["a"], &["o"]),
            ],
        };
        assert!(g.validate_dag().is_err());
        assert!(g.topo_order().is_err());
    }

    #[test]
    fn unknown_dependency_errors() {
        let g = ClaimGraph {
            claims: vec![claim("a", ClaimKind::Physics, &["ghost"], &["o"])],
        };
        let err = g.validate_dag().unwrap_err();
        assert!(err.contains("ghost"), "{err}");
        assert!(g.topo_order().is_err());
    }

    #[test]
    fn duplicate_ids_error() {
        let g = ClaimGraph {
            claims: vec![
                claim("a", ClaimKind::Physics, &[], &["o"]),
                claim("a", ClaimKind::Physics, &[], &["o"]),
            ],
        };
        assert!(g.validate_dag().is_err());
    }

    #[test]
    fn digest_is_order_independent_and_statement_sensitive() {
        let g = valid_graph();
        let d1 = g.digest();

        // Shuffle the claim order: digest must be identical.
        let mut shuffled = g.clone();
        shuffled.claims.reverse();
        assert_eq!(d1, shuffled.digest());
        assert_eq!(d1, claim_graph_digest(&shuffled));

        // Shuffle obligations/depends_on within a claim: still identical.
        let mut perm = g.clone();
        perm.claims[2].depends_on = vec!["b".into(), "a".into()];
        assert_eq!(d1, perm.digest());

        // Change a statement: digest must change.
        let mut changed = g.clone();
        changed.claims[0].statement = "a different statement".into();
        assert_ne!(d1, changed.digest());
    }

    #[test]
    fn unobligated_physics_claims_are_found() {
        let g = ClaimGraph {
            claims: vec![
                claim("a", ClaimKind::Physics, &[], &["obl-a"]),
                claim("bare", ClaimKind::Physics, &[], &[]),
                // An engineering claim with no obligations is NOT flagged.
                claim("eng", ClaimKind::Engineering, &[], &[]),
            ],
        };
        let bare = g.unobligated_physics_claims();
        assert_eq!(bare.len(), 1);
        assert_eq!(bare[0].id, "bare");
        assert_eq!(g.physics_claims().len(), 2);
    }

    #[test]
    fn serde_round_trip() {
        let g = ClaimGraph {
            claims: vec![Claim {
                id: "a".into(),
                sector: Sector::Growth,
                kind: ClaimKind::Physics,
                statement: "growth claim".into(),
                evidence: vec![EvidenceRef::new(
                    "fs8.json",
                    "0".repeat(64),
                    crate::theory::EvidenceTier::T3,
                )],
                obligations: vec!["discharge-growth".into()],
                depends_on: vec![],
            }],
        };
        let json = serde_json::to_string(&g).expect("serialize");
        let back: ClaimGraph = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(g, back);
        // Digest survives a round trip.
        assert_eq!(g.digest(), back.digest());
    }

    #[test]
    fn shared_parameter_audit_passes_for_matched_two_sector_param() {
        let theory = Theory::baseline_lcdm();
        // H0 = 67.4 in the baseline; claim it drives two sectors.
        let uc = UnificationClaim {
            shared: vec![SharedParam {
                symbol: "H0".into(),
                value: 67.4,
                sectors: vec![Sector::Background, Sector::Growth],
            }],
        };
        assert!(uc.shared_parameter_audit(&theory));
    }

    #[test]
    fn shared_parameter_audit_fails_on_value_mismatch() {
        let theory = Theory::baseline_lcdm();
        let uc = UnificationClaim {
            shared: vec![SharedParam {
                symbol: "H0".into(),
                value: 70.0, // mismatches 67.4
                sectors: vec![Sector::Background, Sector::Growth],
            }],
        };
        assert!(!uc.shared_parameter_audit(&theory));
    }

    #[test]
    fn shared_parameter_audit_fails_on_single_sector() {
        let theory = Theory::baseline_lcdm();
        let uc = UnificationClaim {
            shared: vec![SharedParam {
                symbol: "H0".into(),
                value: 67.4,
                sectors: vec![Sector::Background], // only one sector → not shared
            }],
        };
        assert!(!uc.shared_parameter_audit(&theory));
    }

    #[test]
    fn empty_shared_is_not_a_unification_claim() {
        let theory = Theory::baseline_lcdm();
        let uc = UnificationClaim { shared: vec![] };
        assert!(!uc.shared_parameter_audit(&theory));
    }

    #[test]
    fn no_hidden_knob_passes_when_only_free_param_is_shared() {
        // baseline_lcdm has only Fundamental params (no knobs). Add one Free param that we DO share.
        let mut theory = Theory::baseline_lcdm();
        theory.parameters.push(Parameter {
            symbol: "xi".into(),
            value: 0.1,
            physical_meaning: "shared coupling".into(),
            provenance: Provenance::Free,
        });
        let uc = UnificationClaim {
            shared: vec![SharedParam {
                symbol: "xi".into(),
                value: 0.1,
                sectors: vec![Sector::Background, Sector::Growth],
            }],
        };
        assert!(uc.no_hidden_knob_test(&theory));
    }

    #[test]
    fn no_hidden_knob_fails_on_undeclared_free_param() {
        let mut theory = Theory::baseline_lcdm();
        theory.parameters.push(Parameter {
            symbol: "xi".into(),
            value: 0.1,
            physical_meaning: "knob".into(),
            provenance: Provenance::Free,
        });
        // The shared set does NOT include xi → hidden per-sector knob.
        let uc = UnificationClaim {
            shared: vec![SharedParam {
                symbol: "H0".into(),
                value: 67.4,
                sectors: vec![Sector::Background, Sector::Growth],
            }],
        };
        assert!(!uc.no_hidden_knob_test(&theory));
    }

    #[test]
    fn uncertified_derived_is_a_knob_certified_is_not() {
        let mut theory = Theory::baseline_lcdm();
        // Uncertified Derived ⇒ knob; not shared ⇒ should fail.
        theory.parameters.push(Parameter {
            symbol: "g_eff".into(),
            value: 1.02,
            physical_meaning: "effective coupling".into(),
            provenance: Provenance::derived("text-only mechanism"),
        });
        let uc_empty = UnificationClaim { shared: vec![] };
        assert!(!uc_empty.no_hidden_knob_test(&theory));

        // Certified Derived ⇒ not a knob; an empty shared set passes (no free knobs at all).
        let mut certified = Theory::baseline_lcdm();
        let cert = crate::theory::DerivedCertificate {
            relation: "coupled_de_geff_over_g".into(),
            inputs: vec![("beta".into(), 0.1)],
            expected: 1.02,
            tolerance: 1e-9,
        };
        certified.parameters.push(Parameter {
            symbol: "g_eff".into(),
            value: 1.02,
            physical_meaning: "effective coupling".into(),
            provenance: Provenance::derived_certified("coupled-DE fifth force", cert),
        });
        assert!(uc_empty.no_hidden_knob_test(&certified));
    }
}
