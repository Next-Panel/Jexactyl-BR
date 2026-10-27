//! Catalogo de categorias e unidades (`catalog.json` -> `catalog.lock.json`).
//!
//! Port fiel de `scripts/i18n/i18n_toolkit/catalog.py`: validacao do
//! manifesto, motor de globs proprio (porte do `globToRegExp`; `fnmatch`
//! diverge em `**/`), atribuicao first-claim-wins com `catch_all` por ultimo,
//! deteccao de sobreposicao/descobertos, hash estavel do lock e perfis.
//! `scan_excluded` (varredura heuristica com NLP) vive na fase do
//! `jx_i18n_nlp` e nao entra aqui.

use std::collections::{BTreeMap, HashMap};

use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Raizes varridas no upstream, em ordem.
pub const SOURCE_ROOTS: [&str; 3] = ["resources/scripts", "resources/lang/en", "resources/views"];
/// Extensoes consideradas fonte.
pub const SOURCE_EXTENSIONS: [&str; 4] = [".ts", ".tsx", ".php", ".blade.php"];

/// Erros de validacao e atribuicao do catalogo.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CatalogError {
    /// Manifesto invalido, com o motivo no estilo do `fail()` original.
    #[error("{0}")]
    InvalidManifest(String),
}

/// Arquivo reivindicado por duas unidades fora do fallback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlapEntry {
    /// Caminho relativo.
    pub relative_path: String,
    /// Unidade que ja tinha o arquivo.
    pub existing_unit: String,
    /// Unidade que tentou reivindicar.
    pub claiming_unit: String,
}

/// Unidade construida com os arquivos atribuidos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuiltUnit {
    /// Identificador.
    pub id: String,
    /// Titulo (pass-through do manifesto).
    pub title: serde_json::Value,
    /// Globs (pass-through; ausente vira nulo no lock).
    pub globs: Option<Vec<String>>,
    /// Exclusoes (ausente vira lista vazia).
    pub exclude: Vec<String>,
    /// Se e fallback (`catch_all is True`, estrito).
    pub catch_all: bool,
    /// Arquivos atribuidos, ordenados.
    pub files: Vec<String>,
}

/// Categoria construida com as unidades atribuidas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuiltCategory {
    /// Identificador (pass-through).
    pub id: serde_json::Value,
    /// Ordem (pass-through).
    pub order: serde_json::Value,
    /// Titulo (pass-through).
    pub title: serde_json::Value,
    /// Dependencias (ausente vira lista vazia).
    pub depends_on: Vec<serde_json::Value>,
    /// Unidades com arquivos.
    pub units: Vec<BuiltUnit>,
}

/// Entrada de `excluded` com os arquivos casados.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExcludedEntry {
    /// Glob da regra.
    pub glob: String,
    /// Motivo, quando presente.
    pub reason: Option<serde_json::Value>,
    /// Arquivos casados, na ordem de varredura.
    pub files: Vec<String>,
}

fn glob_chars() -> &'static str {
    "|\\{}()[]^$+*?."
}

/// Converte glob em fonte de regex (porte exato do `globToRegExp`).
///
/// `**/` vira `(?:.*/)?`, `**` vira `.*`, `*` vira `[^/]*`, `?` vira
/// `[^/]`; os metacaracteres de regex sao escapados.
pub fn glob_to_regexp(glob: &str) -> String {
    let chars: Vec<char> = glob.chars().collect();
    let mut fragments = String::from("^");
    let mut cursor = 0;
    while cursor < chars.len() {
        let current = chars[cursor];
        let following = chars.get(cursor + 1).copied().unwrap_or('\0');
        if current == '*' {
            if following == '*' {
                cursor += 1;
                let after_pair = chars.get(cursor + 1).copied().unwrap_or('\0');
                if after_pair == '/' {
                    cursor += 1;
                    fragments.push_str("(?:.*/)?");
                } else {
                    fragments.push_str(".*");
                }
            } else {
                fragments.push_str("[^/]*");
            }
        } else if current == '?' {
            fragments.push_str("[^/]");
        } else if glob_chars().contains(current) {
            fragments.push('\\');
            fragments.push(current);
        } else {
            fragments.push(current);
        }
        cursor += 1;
    }
    fragments.push('$');
    fragments
}

/// Compila o glob com cache (fonte sempre valida por construcao).
fn compile_glob(glob: &str) -> Regex {
    crate::text::cached_regex(&glob_to_regexp(glob))
}

/// Informa se o caminho casa com algum dos globs.
pub fn matches_any(relative_path: &str, globs: &[String]) -> bool {
    globs
        .iter()
        .any(|glob| compile_glob(glob).is_match(relative_path))
}

/// Informa se o caminho e arquivo-fonte varrivel.
pub fn is_source_file(relative_path: &str) -> bool {
    SOURCE_EXTENSIONS
        .iter()
        .any(|extension| relative_path.ends_with(extension))
}

fn missing() -> String {
    "undefined".to_string()
}

fn display_value(value: Option<&serde_json::Value>) -> String {
    match value {
        None => missing(),
        Some(serde_json::Value::Null) => "null".to_string(),
        Some(serde_json::Value::Bool(true)) => "true".to_string(),
        Some(serde_json::Value::Bool(false)) => "false".to_string(),
        Some(serde_json::Value::String(text)) => text.clone(),
        Some(serde_json::Value::Number(number)) => number.to_string(),
        Some(other) => serde_json::to_string(other).unwrap_or_else(|_| missing()),
    }
}

fn display_field(mapping: &serde_json::Map<String, serde_json::Value>, field: &str) -> String {
    display_value(mapping.get(field))
}

fn is_truthy(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(flag) => *flag,
        serde_json::Value::Number(number) => {
            number.as_i64().is_some_and(|n| n != 0)
                || number.as_u64().is_some_and(|n| n != 0)
                || number.as_f64().is_some_and(|n| n != 0.0)
        }
        serde_json::Value::String(text) => !text.is_empty(),
        serde_json::Value::Array(items) => !items.is_empty(),
        serde_json::Value::Object(map) => !map.is_empty(),
    }
}

/// Valida o manifesto, acumulando todos os erros como o original.
pub fn validate_manifest(manifest: &serde_json::Value) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();
    let Some(raw_categories) = manifest
        .get("categories")
        .and_then(|value| value.as_array())
    else {
        return vec!["manifest has no categories".to_string()];
    };
    if raw_categories.is_empty() {
        return vec!["manifest has no categories".to_string()];
    }
    let mut category_ids: Vec<String> = Vec::new();
    let mut orders: Vec<i128> = Vec::new();
    let mut unit_ids: Vec<String> = Vec::new();
    for category in raw_categories {
        let Some(map) = category.as_object() else {
            errors.push(format!("duplicate or empty category id: {}", missing()));
            continue;
        };
        let category_id = map.get("id");
        let id_ok = category_id
            .and_then(|value| value.as_str())
            .is_some_and(|text| !text.is_empty());
        let id_seen = category_id
            .and_then(|value| value.as_str())
            .is_some_and(|text| category_ids.iter().any(|known| known == text));
        if !id_ok || id_seen {
            errors.push(format!(
                "duplicate or empty category id: {}",
                display_value(category_id)
            ));
        }
        if let Some(text) = category_id.and_then(|value| value.as_str()) {
            category_ids.push(text.to_string());
        }
        let order = map.get("order");
        let order_number = order.and_then(|value| {
            value
                .as_i64()
                .map(i128::from)
                .or_else(|| value.as_u64().map(i128::from))
        });
        let order_seen = order_number.is_some_and(|number| orders.contains(&number));
        if order_number.is_none_or(|number| number < 1) || order_seen {
            errors.push(format!(
                "duplicate or invalid order for {}: {}",
                display_field(map, "id"),
                display_field(map, "order")
            ));
        }
        if let Some(number) = order_number {
            orders.push(number);
        }
        let raw_units = map.get("units").and_then(|value| value.as_array());
        if raw_units.is_none_or(Vec::is_empty) {
            errors.push(format!(
                "category {} has no units",
                display_value(category_id)
            ));
        }
        let mut catch_all_count = 0;
        for unit in raw_units.cloned().unwrap_or_default() {
            let Some(unit_map) = unit.as_object() else {
                errors.push(format!("duplicate or empty unit id: {}", missing()));
                continue;
            };
            let unit_id = unit_map.get("id");
            let unit_ok = unit_id
                .and_then(|value| value.as_str())
                .is_some_and(|text| !text.is_empty());
            let unit_seen = unit_id
                .and_then(|value| value.as_str())
                .is_some_and(|text| unit_ids.iter().any(|known| known == text));
            if !unit_ok || unit_seen {
                errors.push(format!(
                    "duplicate or empty unit id: {}",
                    display_value(unit_id)
                ));
            }
            if let Some(text) = unit_id.and_then(|value| value.as_str()) {
                unit_ids.push(text.to_string());
            }
            let unit_globs = unit_map.get("globs").and_then(|value| value.as_array());
            if unit_globs.is_none_or(Vec::is_empty) {
                errors.push(format!("unit {} has no globs", display_value(unit_id)));
            }
            if unit_map.get("catch_all").is_some_and(is_truthy) {
                catch_all_count += 1;
            }
        }
        if catch_all_count > 1 {
            errors.push(format!(
                "category {} has {catch_all_count} catch-all units",
                display_value(category_id)
            ));
        }
    }
    errors
}

/// Resultado da atribuicao: categorias, mapa arquivo->unidade, sobreposicoes.
pub type AssignOutcome = (
    Vec<BuiltCategory>,
    HashMap<String, String>,
    Vec<OverlapEntry>,
);

/// Atribui arquivos as unidades (first-claim-wins, `catch_all` por ultimo).
pub fn assign_units(
    raw_categories: &[serde_json::Value],
    all_files: &[String],
) -> Result<AssignOutcome, CatalogError> {
    let mut assigned: HashMap<String, String> = HashMap::new();
    let mut overlaps: Vec<OverlapEntry> = Vec::new();
    let mut built_categories: Vec<BuiltCategory> = Vec::new();
    let mut ordered: Vec<&serde_json::Value> = raw_categories.iter().collect();
    ordered.sort_by_key(|category| {
        category
            .get("order")
            .and_then(|value| value.as_i64().or_else(|| value.as_u64().map(|n| n as i64)))
            .unwrap_or(0)
    });
    for category in ordered {
        let Some(map) = category.as_object() else {
            return Err(CatalogError::InvalidManifest(
                "manifesto invalido: categoria fora do formato esperado".to_string(),
            ));
        };
        let raw_units = map
            .get("units")
            .and_then(|value| value.as_array())
            .ok_or_else(|| {
                CatalogError::InvalidManifest(
                    "manifesto invalido: categoria sem lista de unidades".to_string(),
                )
            })?;
        let mut ordered_units: Vec<&serde_json::Value> = raw_units
            .iter()
            .filter(|unit| unit.get("catch_all").is_none_or(|value| !is_truthy(value)))
            .collect();
        ordered_units.extend(
            raw_units
                .iter()
                .filter(|unit| unit.get("catch_all").is_some_and(is_truthy)),
        );
        let mut built_units = Vec::new();
        for unit in ordered_units {
            let Some(unit_map) = unit.as_object() else {
                return Err(CatalogError::InvalidManifest(
                    "manifesto invalido: unidade fora do formato esperado".to_string(),
                ));
            };
            let unit_id = unit_map
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| {
                    CatalogError::InvalidManifest(
                        "manifesto invalido: unidade sem id textual".to_string(),
                    )
                })?
                .to_string();
            let unit_globs = match unit_map.get("globs") {
                None => None,
                Some(serde_json::Value::Array(items)) => {
                    let mut globs = Vec::new();
                    for item in items {
                        globs.push(item.as_str().ok_or_else(|| CatalogError::InvalidManifest("manifesto invalido: globs da unidade fora do formato esperado".to_string()))?.to_string());
                    }
                    Some(globs)
                }
                Some(_) => {
                    return Err(CatalogError::InvalidManifest(
                        "manifesto invalido: globs da unidade fora do formato esperado".to_string(),
                    ))
                }
            };
            let unit_exclude = match unit_map.get("exclude") {
                None => Vec::new(),
                Some(serde_json::Value::Array(items)) => {
                    let mut exclude = Vec::new();
                    for item in items {
                        exclude.push(item.as_str().ok_or_else(|| CatalogError::InvalidManifest("manifesto invalido: exclude da unidade fora do formato esperado".to_string()))?.to_string());
                    }
                    exclude
                }
                Some(_) => {
                    return Err(CatalogError::InvalidManifest(
                        "manifesto invalido: exclude da unidade fora do formato esperado"
                            .to_string(),
                    ))
                }
            };
            let catch_all = unit_map
                .get("catch_all")
                .is_some_and(|value| value == &serde_json::Value::Bool(true));
            let claiming_catch_all = unit_map.get("catch_all").is_some_and(is_truthy);
            let mut claimed: Vec<String> = Vec::new();
            for relative_path in all_files {
                if !matches_any(relative_path, unit_globs.as_deref().unwrap_or(&[])) {
                    continue;
                }
                if matches_any(relative_path, &unit_exclude) {
                    continue;
                }
                if let Some(existing) = assigned.get(relative_path) {
                    if !claiming_catch_all {
                        overlaps.push(OverlapEntry {
                            relative_path: relative_path.clone(),
                            existing_unit: existing.clone(),
                            claiming_unit: unit_id.clone(),
                        });
                    }
                    continue;
                }
                assigned.insert(relative_path.clone(), unit_id.clone());
                claimed.push(relative_path.clone());
            }
            claimed.sort();
            built_units.push(BuiltUnit {
                id: unit_id,
                title: unit_map
                    .get("title")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                globs: unit_globs,
                exclude: unit_exclude,
                catch_all,
                files: claimed,
            });
        }
        built_categories.push(BuiltCategory {
            id: map.get("id").cloned().unwrap_or(serde_json::Value::Null),
            order: map.get("order").cloned().unwrap_or(serde_json::Value::Null),
            title: map.get("title").cloned().unwrap_or(serde_json::Value::Null),
            depends_on: map
                .get("depends_on")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default(),
            units: built_units,
        });
    }
    Ok((built_categories, assigned, overlaps))
}

/// Casa arquivos com as regras de `excluded` (primeira regra vence).
pub fn collect_excluded(
    manifest: &serde_json::Value,
    all_files: &[String],
    assigned: &HashMap<String, String>,
) -> Result<(Vec<ExcludedEntry>, Vec<String>), CatalogError> {
    let mut entries = Vec::new();
    let mut excluded_set: Vec<String> = Vec::new();
    let raw_excluded = manifest
        .get("excluded")
        .filter(|value| !value.is_null())
        .map_or_else(Vec::new, |value| {
            value.as_array().cloned().unwrap_or_default()
        });
    if manifest
        .get("excluded")
        .is_some_and(|value| !value.is_null() && !value.is_array())
    {
        return Err(CatalogError::InvalidManifest(
            "manifesto invalido: bloco excluded fora do formato esperado".to_string(),
        ));
    }
    for entry in raw_excluded {
        let Some(map) = entry.as_object() else {
            return Err(CatalogError::InvalidManifest(
                "manifesto invalido: entrada excluded fora do formato esperado".to_string(),
            ));
        };
        let glob_text = match map.get("glob") {
            None => String::new(),
            Some(serde_json::Value::String(text)) => text.clone(),
            Some(_) => {
                return Err(CatalogError::InvalidManifest(
                    "manifesto invalido: glob excluded fora do formato esperado".to_string(),
                ))
            }
        };
        let matched: Vec<String> = all_files
            .iter()
            .filter(|path| {
                matches_any(path, std::slice::from_ref(&glob_text))
                    && !assigned.contains_key(*path)
                    && !excluded_set.contains(path)
            })
            .cloned()
            .collect();
        let record = ExcludedEntry {
            glob: glob_text,
            reason: map.get("reason").cloned(),
            files: matched.clone(),
        };
        entries.push(record);
        for relative_path in matched {
            if !excluded_set.contains(&relative_path) {
                excluded_set.push(relative_path);
            }
        }
    }
    Ok((entries, excluded_set))
}

/// Valor canonico com chaves ordenadas (para o hash estavel).
fn stable_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(stable_value).collect())
        }
        serde_json::Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (key, item) in map {
                sorted.insert(key.clone(), stable_value(item));
            }
            serde_json::Value::Object(sorted.into_iter().collect())
        }
        _ => value.clone(),
    }
}

/// Hash SHA-256 do lock, ignorando `generated_at` e `source_head`.
pub fn hash_lock(lock: &serde_json::Value) -> String {
    let empty = serde_json::Map::new();
    let map = lock.as_object().unwrap_or(&empty);
    let mut stable = BTreeMap::new();
    for (key, value) in map {
        if key == "generated_at" || key == "source_head" {
            continue;
        }
        stable.insert(key.clone(), stable_value(value));
    }
    let payload = serde_json::Value::Object(stable.into_iter().collect()).to_string();
    let digest = Sha256::digest(payload.as_bytes());
    format!("{digest:x}")
}

/// Renderiza o perfil Markdown da categoria (para `i18n/categories/`).
pub fn render_profile(category: &BuiltCategory) -> String {
    let title = display_value(Some(&category.title));
    let id = display_value(Some(&category.id));
    let order = display_value(Some(&category.order));
    let dependency_label = if category.depends_on.is_empty() {
        "nada".to_string()
    } else {
        category
            .depends_on
            .iter()
            .map(|item| format!("`{}`", display_value(Some(item))))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut lines = vec![
        format!("# {title}"),
        String::new(),
        format!("- Categoria: `{id}`"),
        format!("- Ordem: {order}"),
        format!("- Depende de: {dependency_label}"),
        String::new(),
        "## Escopo".to_string(),
        String::new(),
    ];
    for unit in &category.units {
        let unit_title = display_value(Some(&unit.title));
        lines.push(format!("### `{}` — {unit_title}", unit.id));
        lines.push(String::new());
        lines.push(format!("- Arquivos: {}", unit.files.len()));
        lines.extend(
            unit.files
                .iter()
                .map(|relative_path| format!("  - `{relative_path}`")),
        );
        lines.push(String::new());
    }
    lines.extend([
        "## Terminologia da categoria".to_string(),
        String::new(),
        "Preencher durante a analise. Termos que divergirem do glossario global".to_string(),
        "precisam de justificativa e revisao.".to_string(),
        String::new(),
        "## Riscos conhecidos".to_string(),
        String::new(),
        "- Strings dinamicas vindas da API nao sao labels e nao entram na nota.".to_string(),
        "- Comentarios de codigo nao sao traduzidos.".to_string(),
        "- Placeholders e variaveis precisam ser preservados.".to_string(),
        String::new(),
        "## Checklist de saida".to_string(),
        String::new(),
        "- [ ] Analise registrada para todas as unidades.".to_string(),
        "- [ ] Traducao aplicada apenas em labels.".to_string(),
        "- [ ] Revisao com nota 10 em todas as unidades.".to_string(),
        "- [ ] Consistencia da categoria com nota 10.".to_string(),
        "- [ ] Gates de verificacao executados.".to_string(),
        String::new(),
    ]);
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn glob_engine_matches_python_vectors() {
        assert_eq!(
            glob_to_regexp("resources/scripts/**/x.ts"),
            "^resources/scripts/(?:.*/)?x\\.ts$"
        );
        assert_eq!(glob_to_regexp("*.ts"), "^[^/]*\\.ts$");
        assert_eq!(glob_to_regexp("a?b"), "^a[^/]b$");
        assert!(matches_any(
            "resources/scripts/a/b/x.ts",
            &["resources/scripts/**/x.ts".to_string()]
        ));
        assert!(matches_any(
            "resources/scripts/x.ts",
            &["resources/scripts/**/x.ts".to_string()]
        ));
        assert!(!matches_any(
            "resources/scriptsx/x.ts",
            &["resources/scripts/**/x.ts".to_string()]
        ));
        assert!(matches_any("a.ts", &["*.ts".to_string()]));
        assert!(!matches_any("a/b.ts", &["*.ts".to_string()]));
        assert!(matches_any("ab/c.ts", &["**/*.ts".to_string()]));
    }

    #[test]
    fn validate_manifest_reports_all_problems() {
        let manifest = json!({
            "categories": [
                {"id": "a", "order": 1, "units": [{"id": "u", "globs": ["*.ts"]}]},
                {"id": "a", "order": true, "units": [{"id": "u", "globs": []}]},
                {"id": "", "order": 1, "units": [{"id": "v", "globs": ["x"], "catch_all": true}, {"id": "w", "globs": ["y"], "catch_all": 1}]},
            ]
        });
        let errors = validate_manifest(&manifest);
        assert!(errors
            .iter()
            .any(|e| e.contains("duplicate or empty category id: a")));
        assert!(errors
            .iter()
            .any(|e| e.contains("duplicate or invalid order for a: true")));
        assert!(errors
            .iter()
            .any(|e| e.contains("duplicate or invalid order for : 1")));
        assert!(errors
            .iter()
            .any(|e| e.contains("duplicate or empty unit id: u")));
        assert!(errors.iter().any(|e| e.contains("unit u has no globs")));
        assert!(errors.iter().any(|e| e.contains("has 2 catch-all units")));
        assert!(!errors.is_empty());
        assert!(validate_manifest(&json!({})).contains(&"manifest has no categories".to_string()));
    }

    #[test]
    fn assign_units_first_claim_wins_with_catch_all_last() {
        let raw = vec![json!({
            "id": "cat", "order": 1, "title": "T", "units": [
                {"id": "catch", "globs": ["**/*.ts"], "catch_all": true},
                {"id": "specific", "globs": ["a/*.ts"]},
            ]
        })];
        let files = vec!["a/x.ts".to_string(), "b/y.ts".to_string()];
        let (built, assigned, overlaps) = assign_units(&raw, &files).unwrap();
        assert!(overlaps.is_empty());
        assert_eq!(assigned.get("a/x.ts").map(String::as_str), Some("specific"));
        assert_eq!(assigned.get("b/y.ts").map(String::as_str), Some("catch"));
        assert_eq!(
            built[0]
                .units
                .iter()
                .find(|u| u.id == "specific")
                .unwrap()
                .files,
            vec!["a/x.ts".to_string()]
        );
    }

    #[test]
    fn assign_units_records_overlap_for_specific_claims() {
        let raw = vec![json!({
            "id": "cat", "order": 1, "units": [
                {"id": "one", "globs": ["**/*.ts"]},
                {"id": "two", "globs": ["a/*.ts"]},
            ]
        })];
        let files = vec!["a/x.ts".to_string()];
        let (_, assigned, overlaps) = assign_units(&raw, &files).unwrap();
        assert_eq!(assigned.get("a/x.ts").map(String::as_str), Some("one"));
        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0].claiming_unit, "two");
        assert_eq!(overlaps[0].existing_unit, "one");
    }

    #[test]
    fn collect_excluded_first_match_wins_and_skips_assigned() {
        let manifest =
            json!({"excluded": [{"glob": "**/*.ts", "reason": "r"}, {"glob": "a/*.ts"}]});
        let files = vec!["a/x.ts".to_string(), "b/y.ts".to_string()];
        let mut assigned = HashMap::new();
        assigned.insert("b/y.ts".to_string(), "u".to_string());
        let (entries, excluded) = collect_excluded(&manifest, &files, &assigned).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].files, vec!["a/x.ts".to_string()]);
        assert!(entries[0].reason.is_some());
        assert!(entries[1].files.is_empty());
        assert_eq!(excluded, vec!["a/x.ts".to_string()]);
    }

    #[test]
    fn hash_lock_is_stable_and_ignores_generated_fields() {
        let lock = json!({
            "version": 1, "upstream_sha": "abc", "source_head": "HEAD-X",
            "generated_at": "2026-01-01T00:00:00.000Z", "policy": {"max_agents": 3},
            "stats": {"units": 2},
            "categories": [{"id": "a", "units": [{"id": "u", "files": ["x.ts"]}]}]
        });
        assert_eq!(
            hash_lock(&lock),
            "1cf0d504b5f80efb4d5defde64bd1bb484c6db97fe338a2a7ae9d4b1c1eebf68"
        );
        let mut changed = lock.clone();
        changed["generated_at"] = json!("outra-data");
        changed["source_head"] = json!("outro-head");
        assert_eq!(hash_lock(&changed), hash_lock(&lock));
        let mut different = lock.clone();
        different["stats"] = json!({"units": 3});
        assert_ne!(hash_lock(&different), hash_lock(&lock));
    }

    #[test]
    fn renders_profile_like_python() {
        let category = BuiltCategory {
            id: json!("cat"),
            order: json!(1),
            title: json!("Titulo"),
            depends_on: vec![],
            units: vec![BuiltUnit {
                id: "u".to_string(),
                title: json!("Unidade"),
                globs: Some(vec!["*.ts".to_string()]),
                exclude: vec![],
                catch_all: false,
                files: vec!["a.ts".to_string()],
            }],
        };
        let profile = render_profile(&category);
        assert!(
            profile.starts_with("# Titulo\n\n- Categoria: `cat`\n- Ordem: 1\n- Depende de: nada\n")
        );
        assert!(profile.contains("### `u` — Unidade"));
        assert!(profile.contains("- Arquivos: 1\n  - `a.ts`"));
    }
}
