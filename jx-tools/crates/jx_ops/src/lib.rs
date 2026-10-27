#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
//! Operacoes do overlay: aplicar serie, validar lint e gerar dumps.
//!
//! Espelha `scripts/apply.sh`, `scripts/lint-patches.sh` e
//! `scripts/dump-patch.sh`, com `GitBackend` e `FileSystem` injetados para testes.

/// Aplicacao da serie de patches e overlays no workspace.
pub mod apply;
/// Geracao de patches novos e refresh do ultimo.
pub mod dump;
/// Validacao da serie (DEP-3, ordem, aplicacao limpa, reversibilidade).
pub mod lint;

#[cfg(any(test, feature = "test-support"))]
pub mod support;
