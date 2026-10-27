//! Aplicacao da serie de patches e overlays no workspace.
//!
//! Espelha `scripts/apply.sh`: checagem somente-leitura, reverso total,
//! fallback `--3way` com distincao conflito/base, snapshot por patch e
//! manifesto de overlays dentro de `.git/`.

use std::path::{Path, PathBuf};

use thiserror::Error;

use jx_core::{BrError, FileSystem};
use jx_git::{GitBackend, GitError, Sha40};
use jx_patch::parse_series;
use jx_patch::series::SeriesEntry;

/// Caminho do manifesto de overlays dentro do git do workspace.
pub const MANIFEST_REL: &str = ".git/br-overlays.txt";

/// Pedido de aplicacao (flags do CLI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyRequest {
    /// So valida, sem alterar nada.
    pub check_only: bool,
    /// Desfaz a serie inteira.
    pub reverse: bool,
    /// Entrada do series a pular.
    pub skip: Option<String>,
    /// Cria um commit por patch aplicado.
    pub snapshot: bool,
}

impl Default for ApplyRequest {
    /// Padrao do `apply.sh`: aplica com snapshots.
    fn default() -> Self {
        Self {
            check_only: false,
            reverse: false,
            skip: None,
            snapshot: true,
        }
    }
}

/// Caminhos e pin da aplicacao.
#[derive(Debug, Clone)]
pub struct ApplyPaths {
    /// Raiz do repo (entradas do series resolvem aqui).
    pub root: PathBuf,
    /// Workspace do upstream (alvo do `git apply`).
    pub target_dir: PathBuf,
    /// Arquivo `series`.
    pub series_file: PathBuf,
    /// Diretorio `overlays/`.
    pub overlays_dir: PathBuf,
    /// Pin do upstream.
    pub base_sha: Sha40,
}

/// Resultado da aplicacao.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplyReport {
    /// Entradas aplicadas (ou revertidas no reverso).
    pub applied: Vec<String>,
    /// Entradas puladas via `--skip`.
    pub skipped: Vec<String>,
    /// Avisos nao fatais (overlays locais preservados etc.).
    pub warnings: Vec<String>,
    /// Notas informativas (serie ja aplicada, nada a desfazer).
    pub notes: Vec<String>,
}

/// Erros da aplicacao.
#[derive(Debug, Error)]
pub enum ApplyError {
    /// Workspace ausente ou sem git.
    #[error("upstream workspace not found: {0}")]
    NoWorkspace(String),
    /// Arquivo `series` ausente.
    #[error("series file not found: {0}")]
    NoSeries(String),
    /// Worktree suja fora do modo `--check`.
    #[error("the workspace {0} has uncommitted changes; commit or stash them before applying (or use --check)")]
    DirtyWorkspace(String),
    /// Reverso recusado: commits fora da serie.
    #[error("refusing to reverse: the workspace has commits outside the overlay series")]
    ForeignCommits,
    /// Overlay colide com arquivo do upstream.
    #[error("overlay {0} already exists in the upstream; use a patch instead")]
    OverlayCollision(String),
    /// Overlay modificado no workspace.
    #[error("the overlay {0} was modified in the workspace; update overlays/{0} instead")]
    OverlayModified(String),
    /// Patch do series nao existe no disco.
    #[error("patch listed in the series does not exist: {0}")]
    MissingPatch(String),
    /// Patch nao aplica no modo `--check`.
    #[error("patch does not apply cleanly: {0}")]
    CheckFailed(String),
    /// Patch falhou no reverso (com rollback).
    #[error("failed to revert: {0}")]
    RevertFailed(String),
    /// Conflito com marcadores (workspace mantido no conflito, sem rollback).
    #[error("conflict while applying: {0}")]
    Conflict(String),
    /// Patch nao aplica na base (com rollback).
    #[error("the patch {0} does not apply to the current base; the workspace was restored")]
    BaseMismatch(String),
    /// Falha ao commitar o snapshot.
    #[error("failed to commit the patch in the workspace")]
    CommitFailed,
    /// Erro do git.
    #[error(transparent)]
    Git(#[from] GitError),
    /// Erro de arquivos.
    #[error(transparent)]
    Repo(#[from] BrError),
}

impl ApplyError {
    /// Informa se o workspace foi deixado em estado de conflito (sem rollback).
    pub fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict(_))
    }
}

/// Lista os overlays como caminhos relativos ordenados.
pub fn overlay_files<FS: FileSystem>(fs: &FS, overlays_dir: &Path) -> Result<Vec<String>, BrError> {
    Ok(fs
        .list_files_recursive(overlays_dir)?
        .iter()
        .map(|rel| rel.to_string_lossy().into_owned())
        .collect())
}

/// Aplica a serie conforme o pedido.
pub fn run_apply<G: GitBackend, FS: FileSystem>(
    git: &G,
    fs: &mut FS,
    paths: &ApplyPaths,
    req: &ApplyRequest,
) -> Result<ApplyReport, ApplyError> {
    if !fs.exists(&paths.target_dir) && !git.is_git_repo(&paths.target_dir) {
        return Err(ApplyError::NoWorkspace(
            paths.target_dir.display().to_string(),
        ));
    }
    if git.head_rev(&paths.target_dir).is_err() {
        return Err(ApplyError::NoWorkspace(
            paths.target_dir.display().to_string(),
        ));
    }
    if !fs.exists(&paths.series_file) {
        return Err(ApplyError::NoSeries(
            paths.series_file.display().to_string(),
        ));
    }
    let pre_run_head = git
        .head_rev(&paths.target_dir)
        .map_err(|_| ApplyError::NoWorkspace(paths.target_dir.display().to_string()))?;

    if !req.check_only && !git.status(&paths.target_dir)?.is_empty() {
        return Err(ApplyError::DirtyWorkspace(
            paths.target_dir.display().to_string(),
        ));
    }

    if req.reverse && pre_run_head != paths.base_sha.to_string() {
        let range = format!("{}..{}", paths.base_sha, pre_run_head);
        let foreign = git
            .log_subjects_range(&paths.target_dir, range.as_str())?
            .iter()
            .any(|subject| !subject.starts_with("br: "));
        if foreign {
            return Err(ApplyError::ForeignCommits);
        }
    }

    let series_text = fs
        .read_to_string(&paths.series_file)
        .map_err(|_| ApplyError::NoSeries(paths.series_file.display().to_string()))?;
    let entries = parse_series(&series_text);

    // Colisao de overlays antes de qualquer patch (somente leitura).
    for rel in overlay_files(fs, &paths.overlays_dir)? {
        if git.cat_file_exists(&paths.target_dir, &paths.base_sha.to_string(), rel.as_str())? {
            return Err(ApplyError::OverlayCollision(rel));
        }
    }

    let mut report = ApplyReport::default();
    if entries.is_empty() {
        return Ok(report);
    }
    let patch_path = |entry: &SeriesEntry| paths.root.join(entry.as_str());
    for entry in &entries {
        if !fs.exists(&patch_path(entry)) {
            return Err(ApplyError::MissingPatch(entry.as_str().to_string()));
        }
    }

    let first = &entries[0];
    let last = &entries[entries.len() - 1];
    let first_applied = git
        .apply_check(&paths.target_dir, &patch_path(first), true)
        .is_ok();
    let last_applied = git
        .apply_check(&paths.target_dir, &patch_path(last), true)
        .is_ok();
    if !req.reverse && first_applied && last_applied {
        report.notes.push(format!(
            "the series is already applied in {}",
            paths.target_dir.display()
        ));
        return Ok(report);
    }
    if req.reverse && !first_applied {
        report.notes.push(format!(
            "the series is not applied in {}",
            paths.target_dir.display()
        ));
        return Ok(report);
    }

    let mut order: Vec<&SeriesEntry> = entries.iter().collect();
    if req.reverse {
        order.reverse();
    }
    for entry in order {
        if req.skip.as_deref() == Some(entry.as_str()) {
            report.skipped.push(entry.as_str().to_string());
            continue;
        }
        let patch = patch_path(entry);
        if req.check_only {
            match git.apply_check(&paths.target_dir, &patch, req.reverse) {
                Ok(()) => report.applied.push(entry.as_str().to_string()),
                Err(check_err) => {
                    return Err(ApplyError::CheckFailed(format_failure_with_stderr(
                        entry.as_str(),
                        &check_err,
                    )))
                }
            }
            continue;
        }
        if req.reverse {
            match git.apply_reverse(&paths.target_dir, &patch) {
                Ok(()) => report.applied.push(entry.as_str().to_string()),
                Err(_) => {
                    rollback(git, &paths.target_dir, &pre_run_head)?;
                    return Err(ApplyError::RevertFailed(entry.as_str().to_string()));
                }
            }
            continue;
        }
        match git.apply(&paths.target_dir, &patch) {
            Ok(()) => report.applied.push(entry.as_str().to_string()),
            Err(apply_err) => match git.apply_3way(&paths.target_dir, &patch) {
                Ok(()) => {
                    report.applied.push(entry.as_str().to_string());
                    report
                        .warnings
                        .push(format!("applied with three-way merge: {}", entry.as_str()));
                }
                Err(three_way_err) => {
                    let unmerged = git.ls_unmerged(&paths.target_dir)?;
                    if !unmerged.is_empty() {
                        let details = format_conflict_diagnostics(
                            git,
                            fs,
                            &paths.target_dir,
                            &patch,
                            entry.as_str(),
                            &apply_err,
                            &three_way_err,
                            &unmerged,
                        );
                        return Err(ApplyError::Conflict(details));
                    }
                    rollback(git, &paths.target_dir, &pre_run_head)?;
                    return Err(ApplyError::BaseMismatch(format_failure_with_stderr(
                        entry.as_str(),
                        &apply_err,
                    )));
                }
            },
        }
        if req.snapshot && !git.status(&paths.target_dir)?.is_empty() {
            git.add_all(&paths.target_dir)?;
            let message = format!("br: patch {}", file_name(entry.as_str()));
            git.commit(&paths.target_dir, message.as_str())
                .map_err(|_| ApplyError::CommitFailed)?;
        }
    }

    if req.check_only {
        return Ok(report);
    }
    if req.reverse {
        remove_overlay_files(git, fs, &paths.target_dir, &mut report.warnings)?;
        git.reset_hard(&paths.target_dir, &paths.base_sha.to_string())?;
        git.clean(&paths.target_dir)?;
    } else {
        copy_overlays(fs, &paths.target_dir, &paths.overlays_dir)?;
        if req.snapshot && !git.status(&paths.target_dir)?.is_empty() {
            git.add_all(&paths.target_dir)?;
            git.commit(&paths.target_dir, "br: overlays")
                .map_err(|_| ApplyError::CommitFailed)?;
        }
    }
    Ok(report)
}

/// Restaura o workspace ao estado anterior a execucao.
fn rollback<G: GitBackend>(git: &G, target: &Path, pre_run_head: &str) -> Result<(), ApplyError> {
    git.reset_hard(target, pre_run_head)?;
    git.clean(target)?;
    Ok(())
}

/// Nome base da entrada do series.
fn file_name(entry: &str) -> &str {
    entry.rsplit('/').next().unwrap_or(entry)
}

/// Anexa `stderr` de um `GitError::CommandFailed` quando nao vazio.
fn format_failure_with_stderr(entry: &str, err: &GitError) -> String {
    match err {
        GitError::CommandFailed { stderr, .. } if !stderr.trim().is_empty() => {
            format!("{entry}\n{}", indent_block(stderr.trim(), "  "))
        }
        _ => entry.to_string(),
    }
}

/// Indenta todas as linhas de um bloco de texto.
fn indent_block(text: &str, prefix: &str) -> String {
    text.lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extrai caminhos unicos de `git ls-files -u` (ou entradas de teste).
fn parse_unmerged_paths(unmerged_raw: &[String]) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    for line in unmerged_raw {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let path = if let Some((_, after_tab)) = trimmed.split_once('\t') {
            after_tab.trim()
        } else {
            trimmed.split_whitespace().last().unwrap_or(trimmed)
        };
        if !path.is_empty() && !paths.iter().any(|existing| existing == path) {
            paths.push(path.to_string());
        }
    }
    paths
}

/// Extrai blocos `<<<<<<<` ... `=======` ... `>>>>>>>` com numeros de linha (1-based).
pub fn extract_conflict_blocks(file_content: &str) -> Vec<(usize, usize, String)> {
    let mut blocks = Vec::new();
    let mut current_start: Option<usize> = None;
    let mut current_lines: Vec<String> = Vec::new();
    for (idx, line) in file_content.lines().enumerate() {
        let line_no = idx + 1;
        if line.starts_with("<<<<<<<") && current_start.is_none() {
            current_start = Some(line_no);
            current_lines.clear();
            current_lines.push(format!("{line_no:>5} | {line}"));
        } else if let Some(start) = current_start {
            current_lines.push(format!("{line_no:>5} | {line}"));
            if line.starts_with(">>>>>>>") {
                blocks.push((start, line_no, current_lines.join("\n")));
                current_start = None;
                current_lines.clear();
            }
        }
    }
    blocks
}

/// Extrai o SHA base (`index <old_blob>..<new_blob>`) e os cabecalhos `@@ ... @@` de um arquivo no patch.
pub fn extract_patch_file_meta(
    patch_content: &str,
    target_file: &str,
) -> (Option<String>, Vec<String>) {
    let mut in_target = false;
    let mut old_blob: Option<String> = None;
    let mut hunks: Vec<String> = Vec::new();
    let header_prefix = format!("diff --git a/{target_file} b/{target_file}");
    for line in patch_content.lines() {
        if line.starts_with("diff --git ") {
            in_target = line.trim() == header_prefix;
            continue;
        }
        if !in_target {
            continue;
        }
        if old_blob.is_none() {
            if let Some(rest) = line.strip_prefix("index ") {
                if let Some((old_sha, _)) = rest.split_once("..") {
                    let clean = old_sha.trim();
                    if !clean.is_empty() {
                        old_blob = Some(clean.to_string());
                    }
                }
            }
        }
        if line.starts_with("@@ ") {
            hunks.push(line.to_string());
        }
    }
    (old_blob, hunks)
}

/// Monta diagnostico detalhado de conflito para orientar o desenvolvedor/LLM.
#[allow(clippy::too_many_arguments)]
fn format_conflict_diagnostics<G: GitBackend, FS: FileSystem>(
    git: &G,
    fs: &FS,
    target_dir: &Path,
    patch_path: &Path,
    entry: &str,
    apply_err: &GitError,
    three_way_err: &GitError,
    unmerged_raw: &[String],
) -> String {
    let files = parse_unmerged_paths(unmerged_raw);
    let patch_content = fs.read_to_string(patch_path).unwrap_or_default();
    let mut out = format!("{entry} (arquivos em conflito: {})", files.join(", "));

    if let GitError::CommandFailed { stderr, .. } = apply_err {
        if !stderr.trim().is_empty() {
            out.push_str("\n  diagnostico git apply:\n");
            out.push_str(&indent_block(stderr.trim(), "    "));
        }
    }
    if let GitError::CommandFailed { stderr, .. } = three_way_err {
        if !stderr.trim().is_empty() {
            out.push_str("\n  diagnostico git apply --3way:\n");
            out.push_str(&indent_block(stderr.trim(), "    "));
        }
    }

    for file in &files {
        out.push_str(&format!("\n  arquivo: {file}"));
        let (old_blob, hunks) = extract_patch_file_meta(&patch_content, file);
        if !hunks.is_empty() {
            out.push_str(&format!("\n    hunks do patch: {}", hunks.join(" | ")));
        }
        let full_path = target_dir.join(file);
        if let Ok(content) = fs.read_to_string(&full_path) {
            let blocks = extract_conflict_blocks(&content);
            for (idx, (start, end, snippet)) in blocks.iter().enumerate() {
                out.push_str(&format!(
                    "\n    marcador de conflito #{} (linhas {start}..{end} em {file}):\n{}",
                    idx + 1,
                    indent_block(snippet, "      ")
                ));
            }
        }
        if let Some(blob) = old_blob {
            let head_spec = format!("HEAD:{file}");
            if let Ok(Some(diff)) = git.diff_objects(target_dir, &blob, &head_spec) {
                if !diff.trim().is_empty() {
                    out.push_str(&format!(
                        "\n    mudancas no upstream ({blob} -> {head_spec}):\n{}",
                        indent_block(diff.trim(), "      ")
                    ));
                }
            }
        }
    }
    out
}

/// Remove os overlays de uma execucao anterior, preservando edicoes locais.
fn remove_overlay_files<G: GitBackend, FS: FileSystem>(
    git: &G,
    fs: &mut FS,
    target: &Path,
    warnings: &mut Vec<String>,
) -> Result<(), ApplyError> {
    let manifest = target.join(MANIFEST_REL);
    if !fs.exists(&manifest) {
        return Ok(());
    }
    let listed = fs.read_to_string(&manifest)?;
    for rel in listed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let current = target.join(rel);
        if fs.exists(&current) && !git.diff_quiet_head(target, rel)? {
            warnings.push(format!(
                "the overlay {rel} was modified in the workspace, keeping the local version"
            ));
            continue;
        }
        fs.remove_file(&current)?;
    }
    fs.remove_file(&manifest)?;
    Ok(())
}

/// Copia os overlays sobre o workspace e registra o manifesto.
pub(crate) fn copy_overlays<FS: FileSystem>(
    fs: &mut FS,
    target: &Path,
    overlays_dir: &Path,
) -> Result<(), ApplyError> {
    let rels = overlay_files(fs, overlays_dir)?;
    let mut manifest = String::new();
    for rel in rels {
        let src = overlays_dir.join(rel.as_str());
        let dst = target.join(rel.as_str());
        if fs.exists(&dst) {
            let (current, wanted) = (fs.read_to_string(&dst)?, fs.read_to_string(&src)?);
            if current != wanted {
                return Err(ApplyError::OverlayModified(rel));
            }
        }
        fs.copy_file(&src, &dst)?;
        manifest.push_str(rel.as_str());
        manifest.push('\n');
    }
    if !manifest.is_empty() {
        fs.write_string(&target.join(MANIFEST_REL), manifest.as_str())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::ScriptGit;
    use jx_core::MemFs;
    use jx_git::status::parse_status_porcelain;

    const BASE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn paths() -> ApplyPaths {
        ApplyPaths {
            root: PathBuf::from("/repo"),
            target_dir: PathBuf::from("/ws"),
            series_file: PathBuf::from("/repo/patches/series"),
            overlays_dir: PathBuf::from("/repo/overlays"),
            base_sha: Sha40::parse(BASE).unwrap(),
        }
    }

    fn seed(fs: &mut MemFs) {
        fs.write_string(
            Path::new("/repo/patches/series"),
            "patches/0001-a.patch\npatches/0002-b.patch\n",
        )
        .unwrap();
        fs.write_string(Path::new("/repo/patches/0001-a.patch"), "diff-a\n")
            .unwrap();
        fs.write_string(Path::new("/repo/patches/0002-b.patch"), "diff-b\n")
            .unwrap();
        fs.write_string(Path::new("/ws/.git/HEAD"), "ref\n")
            .unwrap();
    }

    fn script_git() -> ScriptGit {
        let mut git = ScriptGit::default();
        git.head = BASE.to_string();
        // Serie nao aplicada: reverse-check falha em ambos.
        git.check_reverse_ok
            .insert("/repo/patches/0001-a.patch".to_string(), false);
        git.check_reverse_ok
            .insert("/repo/patches/0002-b.patch".to_string(), false);
        git
    }

    #[test]
    fn forward_applies_with_snapshots() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        let git = script_git();
        let report = run_apply(&git, &mut fs, &paths(), &ApplyRequest::default()).unwrap();
        assert_eq!(
            report.applied,
            vec!["patches/0001-a.patch", "patches/0002-b.patch"]
        );
        assert!(git
            .commits
            .borrow()
            .iter()
            .any(|m| m == "br: patch 0001-a.patch"));
        assert!(!git
            .calls
            .borrow()
            .iter()
            .any(|call| call.starts_with("reset-hard")));
    }

    #[test]
    fn already_applied_is_a_noop_note() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        let mut git = ScriptGit::default();
        git.head = BASE.to_string();
        git.check_reverse_ok
            .insert("/repo/patches/0001-a.patch".to_string(), true);
        git.check_reverse_ok
            .insert("/repo/patches/0002-b.patch".to_string(), true);
        let report = run_apply(&git, &mut fs, &paths(), &ApplyRequest::default()).unwrap();
        assert!(report.applied.is_empty());
        assert_eq!(report.notes.len(), 1);
        assert!(git.commits.borrow().is_empty());
        assert!(git
            .calls
            .borrow()
            .iter()
            .all(|call| call.starts_with("check")));
    }

    #[test]
    fn check_only_never_mutates() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        let mut git = ScriptGit::default();
        git.head = BASE.to_string();
        git.check_reverse_ok
            .insert("/repo/patches/0001-a.patch".to_string(), false);
        git.check_ok
            .insert("/repo/patches/0001-a.patch".to_string(), true);
        git.check_ok
            .insert("/repo/patches/0002-b.patch".to_string(), false);
        let req = ApplyRequest {
            check_only: true,
            ..ApplyRequest::default()
        };
        let err = run_apply(&git, &mut fs, &paths(), &req).unwrap_err();
        assert!(matches!(err, ApplyError::CheckFailed(_)));
        assert!(git
            .calls
            .borrow()
            .iter()
            .all(|call| call.starts_with("check")));
    }

    #[test]
    fn three_way_fallback_and_conflict_paths() {
        // Fallback com merge limpo.
        let mut fs = MemFs::default();
        seed(&mut fs);
        let mut git = script_git();
        git.apply_ok
            .insert("/repo/patches/0001-a.patch".to_string(), false);
        git.three_way_ok
            .insert("/repo/patches/0001-a.patch".to_string(), true);
        let report = run_apply(&git, &mut fs, &paths(), &ApplyRequest::default()).unwrap();
        assert_eq!(report.applied.len(), 2);
        assert_eq!(report.warnings.len(), 1);

        // Conflito: sem rollback, erro Conflict.
        let mut fs = MemFs::default();
        seed(&mut fs);
        let mut git = script_git();
        git.apply_ok
            .insert("/repo/patches/0001-a.patch".to_string(), false);
        git.three_way_ok
            .insert("/repo/patches/0001-a.patch".to_string(), false);
        git.unmerged = vec!["UU a.ts".to_string()];
        let err = run_apply(&git, &mut fs, &paths(), &ApplyRequest::default()).unwrap_err();
        assert!(err.is_conflict());
        assert!(!git
            .calls
            .borrow()
            .iter()
            .any(|call| call.starts_with("reset-hard")));

        // Base incompativel: rollback, erro BaseMismatch.
        let mut fs = MemFs::default();
        seed(&mut fs);
        let mut git = script_git();
        git.apply_ok
            .insert("/repo/patches/0001-a.patch".to_string(), false);
        git.three_way_ok
            .insert("/repo/patches/0001-a.patch".to_string(), false);
        let err = run_apply(&git, &mut fs, &paths(), &ApplyRequest::default()).unwrap_err();
        assert!(matches!(err, ApplyError::BaseMismatch(_)));
        assert!(git
            .calls
            .borrow()
            .iter()
            .any(|call| call == &format!("reset-hard {BASE}")));
    }

    #[test]
    fn dirty_workspace_and_missing_patch_fail_fast() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        let mut git = script_git();
        git.statuses.insert(
            PathBuf::from("/ws"),
            parse_status_porcelain(" M x\n").unwrap(),
        );
        let err = run_apply(&git, &mut fs, &paths(), &ApplyRequest::default()).unwrap_err();
        assert!(matches!(err, ApplyError::DirtyWorkspace(_)));

        let mut fs = MemFs::default();
        seed(&mut fs);
        fs.remove_file(Path::new("/repo/patches/0002-b.patch"))
            .unwrap();
        let git = script_git();
        let err = run_apply(&git, &mut fs, &paths(), &ApplyRequest::default()).unwrap_err();
        assert!(matches!(err, ApplyError::MissingPatch(_)));
    }

    #[test]
    fn reverse_resets_to_base_and_removes_overlays() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        fs.write_string(Path::new("/repo/overlays/app/x.txt"), "novo\n")
            .unwrap();
        let mut git = ScriptGit::default();
        git.head = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string();
        git.check_reverse_ok
            .insert("/repo/patches/0001-a.patch".to_string(), true);
        git.log_range_ok = true;
        // Aplica para gerar o manifesto, depois reverte.
        let applied = run_apply(&git, &mut fs, &paths(), &ApplyRequest::default()).unwrap();
        assert_eq!(applied.applied.len(), 2);
        assert!(fs.exists(Path::new("/ws/.git/br-overlays.txt")));
        let req = ApplyRequest {
            reverse: true,
            ..ApplyRequest::default()
        };
        let reverted = run_apply(&git, &mut fs, &paths(), &req).unwrap();
        assert_eq!(reverted.applied.len(), 2);
        assert!(!fs.exists(Path::new("/ws/.git/br-overlays.txt")));
        assert!(!fs.exists(Path::new("/ws/app/x.txt")));
        assert!(git
            .calls
            .borrow()
            .iter()
            .any(|call| call == &format!("reset-hard {BASE}")));
    }

    #[test]
    fn reverse_refuses_foreign_commits() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        let mut git = ScriptGit::default();
        git.head = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string();
        git.check_reverse_ok
            .insert("/repo/patches/0001-a.patch".to_string(), true);
        git.log_range_ok = false;
        let req = ApplyRequest {
            reverse: true,
            ..ApplyRequest::default()
        };
        let err = run_apply(&git, &mut fs, &paths(), &req).unwrap_err();
        assert!(matches!(err, ApplyError::ForeignCommits));
    }

    #[test]
    fn conflict_diagnostics_extracts_markers_and_patch_hunks() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        fs.write_string(
            Path::new("/repo/patches/0001-a.patch"),
            "diff --git a/a.ts b/a.ts\nindex 1111111..2222222 100644\n--- a/a.ts\n+++ b/a.ts\n@@ -10,3 +10,3 @@ const x = 1;\n-old\n+new\n",
        )
        .unwrap();
        fs.write_string(
            Path::new("/ws/a.ts"),
            "linha 1\n<<<<<<< ours\nconst novoUpstream = true;\n=======\nconst traduzido = true;\n>>>>>>> theirs\nlinha 7\n",
        )
        .unwrap();
        let mut git = script_git();
        git.apply_ok
            .insert("/repo/patches/0001-a.patch".to_string(), false);
        git.three_way_ok
            .insert("/repo/patches/0001-a.patch".to_string(), false);
        git.unmerged = vec!["100644 1111111 1\ta.ts".to_string()];
        let err = run_apply(&git, &mut fs, &paths(), &ApplyRequest::default()).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("arquivos em conflito: a.ts"), "{msg}");
        assert!(msg.contains("@@ -10,3 +10,3 @@ const x = 1;"), "{msg}");
        assert!(msg.contains("linhas 2..6 em a.ts"), "{msg}");
        assert!(msg.contains("<<<<<<< ours"), "{msg}");
        assert!(msg.contains(">>>>>>> theirs"), "{msg}");
    }
}
