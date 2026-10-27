//! Validacao da serie de patches.
//!
//! Espelha `scripts/lint-patches.sh`: estrutura, DEP-3, ordem, arquivos
//! orfaos, aplicacao limpa em clone isolado, idempotencia, reversibilidade
//! e colisao de overlays. Falhas acumulam (sem fail-fast), como `FAILURES`.

use std::collections::HashSet;
use std::path::PathBuf;

use jx_core::FileSystem;
use jx_git::{GitBackend, Sha40};
use jx_patch::dep3;
use jx_patch::series::{parse_series, PatchNumber};
use jx_patch::{forbidden_touches, PatchFileName};

/// Caminhos e pin da validacao.
#[derive(Debug, Clone)]
pub struct LintPaths {
    /// Raiz do repo.
    pub root: PathBuf,
    /// Diretorio `patches/`.
    pub patches_dir: PathBuf,
    /// Arquivo `series`.
    pub series_file: PathBuf,
    /// Diretorio `overlays/`.
    pub overlays_dir: PathBuf,
    /// Workspace do upstream (fonte do clone).
    pub workspace_dir: PathBuf,
    /// URL do upstream (fallback de clone).
    pub upstream_url: String,
    /// Pin do upstream.
    pub base_sha: Sha40,
    /// Diretorio do clone isolado (`.br-cache/lint`).
    pub cache_dir: PathBuf,
}

/// Falha de validacao, opcionalmente ligada a uma entrada do series.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintFailure {
    /// Entrada do series ou caminho relativo; `None` = serie/config.
    pub target: Option<String>,
    /// Mensagem no estilo do `lint-patches.sh`.
    pub message: String,
}

/// Resultado da validacao.
#[derive(Debug, Clone, Default)]
pub struct LintReport {
    /// Todas as falhas encontradas.
    pub failures: Vec<LintFailure>,
    /// Patches aplicados no clone isolado.
    pub applied: usize,
    /// Arquivos de overlay contados.
    pub overlays: usize,
}

impl LintReport {
    /// Serie valida: nenhuma falha.
    pub fn is_ok(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Acumula uma falha no relatorio (sem fail-fast, como `FAILURES`).
fn push_failure(report: &mut LintReport, target: Option<String>, message: String) {
    report.failures.push(LintFailure { target, message });
}
const REQUIRED_FIELDS: [&str; 8] = [
    "From:",
    "Date:",
    "Subject:",
    "Description:",
    "Origin:",
    "Forwarded:",
    "Applied-Upstream:",
    "Last-Update:",
];

/// Valida um arquivo de patch acumulando todas as falhas.
pub fn lint_patch_content(name: &str, text: &str) -> Vec<LintFailure> {
    let mut failures: Vec<LintFailure> = Vec::new();
    let target = || Some(name.to_string());
    let fail = |message: String| LintFailure {
        target: target(),
        message,
    };

    // Terminador antes do primeiro diff.
    let mut header_end: Option<usize> = None;
    let mut body_from: Option<usize> = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        let content = content.strip_suffix('\r').unwrap_or(content);
        if content == "---" {
            header_end = Some(offset);
            body_from = Some(offset + line.len());
            break;
        }
        if content.starts_with("diff --git ") {
            body_from = Some(offset);
            break;
        }
        offset += line.len();
    }
    if header_end.is_none() {
        failures.push(fail(
            "patch has no '---' terminator before the diff (DEP-3)".to_string(),
        ));
    }
    let header = match header_end {
        Some(end) => &text[..end],
        None => match body_from {
            Some(start) => &text[..start],
            None => text,
        },
    };
    let body = match body_from {
        Some(start) => &text[start..],
        None => "",
    };
    if !body.lines().any(|line| line.starts_with("diff --git ")) {
        failures.push(fail("patch has no git diff body".to_string()));
    }

    for field in REQUIRED_FIELDS {
        let bare = field.trim_end_matches(':');
        if dep3_header_value(header, bare).is_none() {
            failures.push(fail(format!(
                "patch {name} is missing the '{field}' header field"
            )));
        }
    }
    let has_signoff = dep3_header_value(header, "Signed-off-by").is_some()
        || dep3_header_value(header, "Reviewed-by").is_some();
    if !has_signoff {
        failures.push(fail(format!(
            "patch {name} is missing the 'Signed-off-by:' field"
        )));
    }

    let subjects = header
        .lines()
        .filter(|line| line.starts_with("Subject:"))
        .count();
    if subjects > 1 {
        failures.push(fail(format!("patch {name} has more than one Subject line")));
    }
    if subjects == 1
        && dep3_header_value(header, "Subject").is_some_and(|value| value.trim().is_empty())
    {
        failures.push(fail(format!("patch {name} has an empty Subject")));
    }
    if dep3::collect_description(header).trim().is_empty() {
        failures.push(fail(format!("patch {name} has an empty Description")));
    }
    if let Some(value) = dep3_header_value(header, "Origin") {
        if value.parse::<dep3::Origin>().is_err() {
            failures.push(fail(format!(
                "patch {name} has an invalid Origin value: {value}"
            )));
        }
    }
    if let Some(value) = dep3_header_value(header, "Forwarded") {
        if value.parse::<dep3::Forwarded>().is_err() {
            failures.push(fail(format!(
                "patch {name} has an invalid Forwarded value: {value}"
            )));
        }
    }
    if let Some(value) = dep3_header_value(header, "Applied-Upstream") {
        if value.parse::<dep3::AppliedUpstream>().is_err() {
            failures.push(fail(format!(
                "patch {name} has an invalid Applied-Upstream value: {value}"
            )));
        }
    }
    if let Some(value) = dep3_header_value(header, "Last-Update") {
        if value.parse::<dep3::IsoDate>().is_err() {
            failures.push(fail(format!(
                "patch {name} has an invalid Last-Update value (expected YYYY-MM-DD): {value}"
            )));
        }
    }

    for touch in forbidden_touches(body) {
        failures.push(fail(format!(
            "patch {name} touches a forbidden path: {}",
            touch.path
        )));
    }
    failures
}

/// Primeira ocorrencia de `Campo: valor` (duplicada aqui para acumular falhas).
fn dep3_header_value(header: &str, field: &str) -> Option<String> {
    let prefix = format!("{field}:");
    header
        .lines()
        .find_map(|line| line.strip_prefix(prefix.as_str()))
        .map(|rest| rest.trim().to_string())
}

/// Valida a serie completa com clone isolado.
pub fn run_lint<G: GitBackend, FS: FileSystem>(
    git: &G,
    fs: &mut FS,
    paths: &LintPaths,
    keep_cache: bool,
) -> LintReport {
    let mut report = LintReport::default();
    let series_text = match fs.read_to_string(&paths.series_file) {
        Ok(text) => text,
        Err(_) => {
            push_failure(
                &mut report,
                None,
                format!("series file not found: {}", paths.series_file.display()),
            );
            return report;
        }
    };
    let entries = parse_series(&series_text);
    if entries.is_empty() && !series_text.contains("allow-empty-series") {
        push_failure(
            &mut report,
            None,
            "the series is empty; add the marker 'allow-empty-series' to accept it".to_string(),
        );
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut previous: Option<PatchNumber> = None;
    let mut ordered_entries: Vec<String> = Vec::new();
    for entry in &entries {
        let text = entry.as_str();
        let patch_file = paths.root.join(text);
        if !fs.exists(&patch_file) {
            push_failure(
                &mut report,
                Some(text.to_string()),
                format!("patch listed in the series does not exist: {text}"),
            );
            continue;
        }
        if !seen.insert(text.to_string()) {
            push_failure(
                &mut report,
                Some(text.to_string()),
                format!("duplicate entry in the series: {text}"),
            );
            continue;
        }
        ordered_entries.push(text.to_string());

        let name = text.rsplit('/').next().unwrap_or(text);
        let valid_name = PatchFileName::from_path(name).is_ok();
        if !valid_name {
            push_failure(
                &mut report,
                Some(text.to_string()),
                format!("patch name does not follow NNNN-kebab-case.patch: {name}"),
            );
        }
        match PatchNumber::from_file_name(text) {
            Ok(number) => {
                if previous.is_some_and(|prev| number <= prev) {
                    push_failure(
                        &mut report,
                        Some(text.to_string()),
                        format!("series is not in ascending numeric order at {text}"),
                    );
                }
                previous = Some(number);
            }
            Err(_) if valid_name => {
                push_failure(
                    &mut report,
                    Some(text.to_string()),
                    format!("patch name does not follow NNNN-kebab-case.patch: {name}"),
                );
            }
            Err(_) => {}
        }
        match fs.read_to_string(&patch_file) {
            Ok(content) => {
                for failure in lint_patch_content(name, &content) {
                    report.failures.push(LintFailure {
                        target: Some(text.to_string()),
                        message: failure.message,
                    });
                }
            }
            Err(e) => push_failure(
                &mut report,
                Some(text.to_string()),
                format!("cannot read patch {text}: {e}"),
            ),
        }
    }

    // Arquivos orfaos: *.patch sob patches/ sem registro no series.
    // O registro e relativo a raiz (ex.: `patches/0001-a.patch`).
    let prefix = paths
        .patches_dir
        .strip_prefix(&paths.root)
        .ok()
        .map(PathBuf::from);
    match fs.list_files_recursive(&paths.patches_dir) {
        Ok(files) => {
            for rel in files {
                let name = rel.to_string_lossy().into_owned();
                if !name.ends_with(".patch") {
                    continue;
                }
                let root_rel = match &prefix {
                    Some(dir) => dir.join(rel).to_string_lossy().into_owned(),
                    None => name.clone(),
                };
                if !seen.contains(&root_rel) {
                    push_failure(
                        &mut report,
                        None,
                        format!("patch file is not registered in the series: {root_rel}"),
                    );
                }
            }
        }
        Err(e) => push_failure(&mut report, None, format!("cannot list patches dir: {e}")),
    }

    // Clone isolado sobre a base fixada.
    if fs.exists(&paths.cache_dir) && fs.remove_dir_all(&paths.cache_dir).is_err() {
        push_failure(
            &mut report,
            None,
            "cannot clean the lint cache dir".to_string(),
        );
        return report;
    }
    let cloned = if fs.exists(&paths.workspace_dir.join(".git")) {
        git.clone_shared_no_checkout(&paths.workspace_dir, &paths.cache_dir)
    } else {
        git.clone_no_checkout(&paths.upstream_url, &paths.cache_dir)
    };
    if let Err(e) = cloned {
        push_failure(
            &mut report,
            None,
            format!("cannot clone for lint validation: {e}"),
        );
        return report;
    }
    let base = paths.base_sha.to_string();
    if git.rev_parse(&paths.cache_dir, base.as_str()).is_err() {
        let _ = git.fetch_tags(&paths.cache_dir);
        if git.rev_parse(&paths.cache_dir, base.as_str()).is_err() {
            push_failure(
                &mut report,
                None,
                format!("pinned commit {base} is not reachable from the upstream remote"),
            );
            return report;
        }
    }
    if git
        .checkout_detach(&paths.cache_dir, base.as_str())
        .is_err()
    {
        push_failure(
            &mut report,
            None,
            format!("cannot checkout {base} in the lint clone"),
        );
        return report;
    }

    // Overlays nao podem colidir com o upstream.
    let overlay_list = fs
        .list_files_recursive(&paths.overlays_dir)
        .unwrap_or_default();
    report.overlays = overlay_list.len();
    for rel in overlay_list {
        let rel_text = rel.to_string_lossy().into_owned();
        match git.cat_file_exists(&paths.cache_dir, base.as_str(), rel_text.as_str()) {
            Ok(true) => push_failure(
                &mut report,
                None,
                format!("overlay {rel_text} already exists in the upstream; use a patch instead"),
            ),
            Ok(false) => {}
            Err(e) => push_failure(
                &mut report,
                None,
                format!("overlay check failed for {rel_text}: {e}"),
            ),
        }
    }

    // Aplicacao sequencial no clone.
    for entry in &ordered_entries {
        let patch = paths.root.join(entry.as_str());
        if let Err(err) = git.apply_check(&paths.cache_dir, &patch, false) {
            let detail = match &err {
                jx_git::GitError::CommandFailed { stderr, .. } if !stderr.trim().is_empty() => {
                    format!(" ({})", stderr.lines().collect::<Vec<_>>().join("; "))
                }
                _ => String::new(),
            };
            push_failure(
                &mut report,
                Some(entry.clone()),
                format!("patch does not apply cleanly on {base}: {entry}{detail}"),
            );
            break;
        }
        if let Err(err) = git.apply(&paths.cache_dir, &patch) {
            let detail = match &err {
                jx_git::GitError::CommandFailed { stderr, .. } if !stderr.trim().is_empty() => {
                    format!(" ({})", stderr.lines().collect::<Vec<_>>().join("; "))
                }
                _ => String::new(),
            };
            push_failure(
                &mut report,
                Some(entry.clone()),
                format!("patch failed while being applied: {entry}{detail}"),
            );
            break;
        }
        report.applied += 1;
    }

    if report.applied == ordered_entries.len() && !ordered_entries.is_empty() {
        let first = paths.root.join(&ordered_entries[0]);
        if git.apply_check(&paths.cache_dir, &first, false).is_ok() {
            push_failure(
                &mut report,
                None,
                "idempotency check failed, the series would be applied twice".to_string(),
            );
        }
        let mut reversible = true;
        for entry in ordered_entries.iter().rev() {
            let patch = paths.root.join(entry.as_str());
            if git.apply_reverse(&paths.cache_dir, &patch).is_err() {
                push_failure(
                    &mut report,
                    Some(entry.clone()),
                    format!("patch is not reversible: {entry}"),
                );
                reversible = false;
                break;
            }
        }
        if reversible {
            match git.status(&paths.cache_dir) {
                Ok(entries) if entries.is_empty() => {}
                Ok(_) => push_failure(
                    &mut report,
                    None,
                    "reversing the series did not restore the pinned base".to_string(),
                ),
                Err(e) => push_failure(
                    &mut report,
                    None,
                    format!("cannot check lint clone status: {e}"),
                ),
            }
        }
    }

    if !keep_cache {
        let _ = fs.remove_dir_all(&paths.cache_dir);
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::ScriptGit;
    use jx_core::MemFs;
    use std::path::Path;

    const BASE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn lint_paths() -> LintPaths {
        LintPaths {
            root: PathBuf::from("/repo"),
            patches_dir: PathBuf::from("/repo/patches"),
            series_file: PathBuf::from("/repo/patches/series"),
            overlays_dir: PathBuf::from("/repo/overlays"),
            workspace_dir: PathBuf::from("/ws"),
            upstream_url: "https://exemplo.test/painel.git".to_string(),
            base_sha: Sha40::parse(BASE).unwrap(),
            cache_dir: PathBuf::from("/repo/.br-cache/lint"),
        }
    }

    const GOOD_PATCH: &str = "From: A <a@exemplo.test>\nDate: seg, 1 jan 2026\nSubject: teste: algo\n\nDescription:\nLinha um.\n\nOrigin: vendor\nForwarded: not-needed\nApplied-Upstream: no\nLast-Update: 2026-10-26\nSigned-off-by: A <a@exemplo.test>\n\n---\n\ndiff --git a/x b/x\n+ola\n";

    fn seed(fs: &mut MemFs) {
        fs.write_string(
            Path::new("/repo/patches/series"),
            "patches/0001-a.patch\npatches/0002-b.patch\n",
        )
        .unwrap();
        fs.write_string(Path::new("/repo/patches/0001-a.patch"), GOOD_PATCH)
            .unwrap();
        fs.write_string(Path::new("/repo/patches/0002-b.patch"), GOOD_PATCH)
            .unwrap();
        fs.write_string(Path::new("/ws/.git/HEAD"), "ref\n")
            .unwrap();
    }

    #[test]
    fn accumulates_header_failures_without_failing_fast() {
        let failures = lint_patch_content(
            "0001-x.patch",
            "From: a\nSubject: s\nSubject: t\ndiff --git a/.env b/.env\n",
        );
        let messages: Vec<&str> = failures
            .iter()
            .map(|failure| failure.message.as_str())
            .collect();
        assert!(messages
            .iter()
            .any(|message| message.contains("no '---' terminator")));
        assert!(messages
            .iter()
            .any(|message| message.contains("more than one Subject")));
        assert!(messages
            .iter()
            .any(|message| message.contains("empty Description")));
        assert!(messages
            .iter()
            .any(|message| message.contains("missing the 'Date:'")));
        assert!(messages
            .iter()
            .any(|message| message.contains("forbidden path: .env")));
        assert!(messages
            .iter()
            .any(|message| message.contains("Signed-off-by")));
    }

    #[test]
    fn valid_series_passes_with_idempotency_and_reversibility() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        let git = ScriptGit::default();
        let report = run_lint(&git, &mut fs, &lint_paths(), true);
        assert!(report.failures.is_empty(), "{:?}", report.failures);
        assert_eq!(report.applied, 2);
    }

    #[test]
    fn reports_missing_duplicate_order_and_orphan() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        fs.write_string(
            Path::new("/repo/patches/series"),
            "patches/0002-b.patch\npatches/0001-a.patch\npatches/0001-a.patch\npatches/0009-falta.patch\n",
        )
        .unwrap();
        fs.write_string(Path::new("/repo/patches/0009-orfao.patch"), GOOD_PATCH)
            .unwrap();
        let git = ScriptGit::default();
        let report = run_lint(&git, &mut fs, &lint_paths(), true);
        let messages: Vec<&str> = report
            .failures
            .iter()
            .map(|failure| failure.message.as_str())
            .collect();
        assert!(messages
            .iter()
            .any(|message| message.contains("ascending numeric order")));
        assert!(messages
            .iter()
            .any(|message| message.contains("duplicate entry")));
        assert!(messages
            .iter()
            .any(|message| message.contains("does not exist")));
        assert!(messages
            .iter()
            .any(|message| message.contains("not registered in the series")));
    }

    #[test]
    fn stops_apply_loop_on_first_dirty_patch() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        let mut git = ScriptGit::default();
        git.check_ok
            .insert("/repo/patches/0002-b.patch".to_string(), false);
        let report = run_lint(&git, &mut fs, &lint_paths(), true);
        assert_eq!(report.applied, 1);
        assert!(report
            .failures
            .iter()
            .any(|failure| failure.message.contains("does not apply cleanly")));
    }
}
