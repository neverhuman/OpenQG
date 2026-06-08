# Optional Boltzmann backend (CLASS / CAMB / hi_class)

The default engine is pure-Rust and deterministic (`BackgroundForwardModel`): it covers the
**Tier-0** observables exactly computable from the FLRW background (BAO distances, SNe μ(z),
BBN Y_p, CMB distance priors R / ℓ_A). The **Tier-1/2** observables — the full CMB C_ℓ, σ8/S8,
nonlinear P(k) — require a Boltzmann solver. Rather than bake a Python/C dependency into the Rust
build, the engine plugs one in through a generic subprocess seam:
`openqg_core::cosmology::SubprocessForwardModel`.

## Protocol
The Rust side (`crates/openqg-core/src/cosmology/subprocess.rs`) runs a command (`sh -c`) and:
- writes `{"params": <CosmologyParams JSON>, "observables": ["<id>", ...]}` to its **stdin**;
- reads a JSON array of `PredictionRecord` (`{"observable_id","value","uncertainty","unit"}`) from
  its **stdout**.

A non-zero exit / crash / timeout is a hard error (a pathological cosmology that makes the solver
fail is a *lethal candidate*, never a silent default — see
`forward-model-and-unification.md` §5). An observable the backend cannot derive is *omitted*, never
faked, so coverage honestly reflects what was produced.

## Why a subprocess (not a Rust feature flag)
The repository enforces strict Python containment: a heavy physics stack (numpy + a compiled
Boltzmann code) must not live in the Rust product tree. The subprocess seam keeps the physics stack
entirely in *your* adapter command — "just a command you pass," exactly like the LLM proposer hook
(`--proposer-cmd`). The Rust mechanism is unit-tested with shell mocks; the production adapter is
the reference below.

## Reference adapter
Save this as your own adapter (outside the audited product tree) and pass
`python3 path/to/boltzmann_adapter.py` as the command. It answers the parameter-passthrough
observables with no dependency, and is the extension point for a real `classy` (CLASS) solve.

```python
#!/usr/bin/env python3
"""Reference Boltzmann backend adapter for openqg-core SubprocessForwardModel.
stdin : {"params": {<CosmologyParams>}, "observables": ["<id>", ...]}
stdout: [{"observable_id","value","uncertainty","unit"}, ...]
"""
import json, sys

def passthrough(p):
    return {
        "h0": (100.0 * p["h"], 0.001, "km s^-1 Mpc^-1"),
        "h0_local": (100.0 * p["h"], 0.001, "km s^-1 Mpc^-1"),
        "omega_m": (p["omega_m"], 1e-6, "dimensionless"),
        "sum_mnu": (p["sum_mnu"], 1e-6, "eV"),
        "n_eff": (p["n_eff"], 1e-6, "dimensionless"),
        "omega_b_h2": (p["omega_b_h2"], 1e-9, "dimensionless"),
    }

def main():
    req = json.load(sys.stdin)
    p, ids = req["params"], req["observables"]
    table = passthrough(p)
    try:
        from classy import Class   # production: configure CLASS from `p` and fill cmb_R, cmb_lA,
        # s8, dm_over_rd@z, ... from the real solve into `table`.
        _ = Class
    except Exception:
        pass
    preds = [
        {"observable_id": oid, "value": v, "uncertainty": u, "unit": unit}
        for oid in ids
        for (v, u, unit) in [table[oid]] if oid in table
    ]
    json.dump(preds, sys.stdout)

if __name__ == "__main__":
    main()
```

## Determinism / reproducibility (production)
Pin the stack (CLASS git SHA + precision files, `classy`/`camb`/`clik` versions, single BLAS,
`OMP_NUM_THREADS=1`), containerize, and have the adapter stamp a code+data hash into
`ForwardManifest.provenance_hash` so each score is reproducible across hosts.
