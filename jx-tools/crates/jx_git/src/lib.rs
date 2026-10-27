#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
//! Tipos e acesso ao git dos workspaces: SHA, status e backend processado.
//!
//! O trait `GitBackend` e a costura de testabilidade: a producao usa
//! `ProcessGit`, os testes injetam fakes em memoria.

pub mod backend;
pub mod sha;
pub mod status;

pub use backend::{GitBackend, GitError, ProcessGit};
pub use sha::Sha40;
pub use status::{parse_status_porcelain, StatusEntry};
