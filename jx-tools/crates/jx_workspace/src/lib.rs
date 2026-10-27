#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
//! Operacoes de workspace: bootstrap, rebase de base e empacotamento.
//!
//! Espelha `scripts/dev-setup.sh`, `scripts/rebase.sh` e `scripts/package.sh`.
//! `scripts/consolidate-patches.sh` e migracao one-shot 47->26 ja executada
//! e fica como legado: nao e portado.

/// Empacotamento reproduzivel do painel com a serie aplicada.
pub mod package;
/// Troca da base fixada com replay da serie.
pub mod rebase;
/// Bootstrap do workspace: clone, pin, serie e dependencias.
pub mod setup;
