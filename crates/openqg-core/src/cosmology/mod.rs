//! Real, deterministic cosmology: the background forward model that replaces an identity
//! `forward_map` with genuine theory→observable computation, plus the `ForwardModel` seam the
//! evolution loop selects through. See `docs/research/forward-model-and-unification.md`.

mod background;
mod forward;
mod growth;
mod observables;
mod subprocess;

pub use background::{BackgroundError, CosmologyParams, MgFamily, C_KM_S};
pub use forward::{
    BackgroundForwardModel, ForwardFailure, ForwardKind, ForwardManifest, ForwardModel,
    ForwardOutcome, ForwardTier,
};
pub use growth::GrowthHistory;
pub use observables::{canonicalize_observable_id, observables_match, CanonicalObservable};
pub use subprocess::SubprocessForwardModel;
