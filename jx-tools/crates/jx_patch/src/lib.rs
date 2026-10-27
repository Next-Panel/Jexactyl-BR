#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
//! Serie de patches, cabecalhos DEP-3 e caminhos tocados.
//!
//! Espelha `scripts/lib/common.sh` (serie) e `scripts/lint-patches.sh`
//! (validacao), com tipos fechados em vez de texto livre.

pub mod dep3;
pub mod paths;
pub mod series;

pub use dep3::{
    AppliedUpstream, Forwarded, IsoDate, Origin, PatchFile, PatchFileName, PatchHeader, Signoff,
    Subject,
};
pub use paths::{forbidden_touches, is_forbidden_path, touched_paths, DiffTouch};
pub use series::{
    domain_label, insert_into_text, next_after, parse_series, PatchNumber, SeriesEntry, SeriesError,
};
