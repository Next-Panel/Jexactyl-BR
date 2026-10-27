#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
//! Nucleo compartilhado das ferramentas jx: erros, relogio, config e arquivos.

/// Relogio abstrato e formato ISO-8601.
pub mod clock;
/// Leitura e escrita do config.toml preservando formato.
pub mod config;
/// Erros tipados da borda de I/O e configuracao.
pub mod error;
/// Sistema de arquivos real e em memoria.
pub mod fs;

pub use clock::{format_iso_millis, format_rfc2822, format_ymd, Clock, SystemClock};
pub use config::{config_get, config_set, require_inside_root, resolve_path, ConfigKey};
pub use error::BrError;
pub use fs::{FileSystem, MemFs, RealFs};
