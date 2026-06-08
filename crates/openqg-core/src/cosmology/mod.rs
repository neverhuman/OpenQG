//! Real, deterministic cosmology: the background forward model that replaces an identity
//! `forward_map` with genuine theory→observable computation, plus the `ForwardModel` seam the
//! evolution loop selects through. See `docs/research/forward-model-and-unification.md`.

mod background;
mod forward;
mod subprocess;

pub use background::{CosmologyParams, C_KM_S};
pub use forward::{BackgroundForwardModel, ForwardKind, ForwardManifest, ForwardModel};
pub use subprocess::SubprocessForwardModel;
