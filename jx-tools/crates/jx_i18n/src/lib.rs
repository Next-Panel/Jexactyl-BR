#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
//! Motor i18n do overlay: tipos, texto, PHP, catalogo e (em breve) scanner,
//! idioma, auditoria e fila.
//!
//! Reformulacao do `scripts/i18n/i18n_toolkit/` em modulos testaveis e
//! typesafe, sem as camadas legadas (estado em markdown, locks, bridges).

/// Auditoria de traducao (pares, overlays, fonte inglesa).
pub mod audit;
/// Catalogo de categorias e unidades (`catalog.json` -> `catalog.lock.json`).
pub mod catalog;
/// Tipos centrais: severidade e achado de auditoria.
pub mod core;
/// Classificacao de idioma EN/PT (lingua + Zipf + glossario).
pub mod lang;
/// Orquestracao da auditoria sobre git + filesystem (port de `audit()`).
pub mod orchestrate;
/// Parser de arquivos `resources/lang/*.php`.
pub mod php;
/// Scanner de labels, comentarios, skeleton e chaves de lang.
pub mod scan;
/// Maquina de estados da fila i18n sobre SQLite (port de `queue.py`).
pub mod store;
/// Normalizacao de texto, placeholders e URLs.
pub mod text;
