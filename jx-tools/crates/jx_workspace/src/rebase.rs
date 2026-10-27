//! Troca da base fixada com replay da serie.
//!
//! Espelha `scripts/rebase.sh`: resolve a nova base (tag, branch remoto ou
//! `ls-remote`), ensaia com `--dry-run`, reaplica a serie e grava o pin so
//! apos sucesso total.

use std::path::PathBuf;

use thiserror::Error;

use jx_core::{BrError, FileSystem};
use jx_git::{GitBackend, GitError, Sha40};
use jx_ops::apply::{run_apply, ApplyPaths, ApplyRequest};

/// Pedido de rebase (flags do CLI).
#[derive(Debug, Clone, Default)]
pub struct RebaseRequest {
    /// Nova ref (tag ou branch).
    pub new_ref: Option<String>,
    /// Commit especifico.
    pub new_sha: Option<String>,
    /// Grava a nova ref/sha no config.toml.
    pub update_pin: bool,
    /// So resolve e mostra, sem alterar nada.
    pub dry_run: bool,
    /// Descarta mudancas nao commitadas do workspace.
    pub force: bool,
}

impl RebaseRequest {
    /// Padrao do `rebase.sh`: grava o pin, executa de verdade.
    pub fn pinned(new_ref: &str) -> Self {
        Self {
            new_ref: Some(new_ref.to_string()),
            update_pin: true,
            ..Self::default()
        }
    }
}

/// Caminhos e pin atual do rebase.
#[derive(Debug, Clone)]
pub struct RebasePaths {
    /// Raiz do repo.
    pub root: PathBuf,
    /// Workspace do upstream.
    pub target_dir: PathBuf,
    /// Arquivo `series`.
    pub series_file: PathBuf,
    /// Diretorio `overlays/`.
    pub overlays_dir: PathBuf,
    /// URL do upstream.
    pub upstream_url: String,
    /// Ref atual (rotulo).
    pub current_ref: String,
    /// SHA atual.
    pub current_sha: Sha40,
    /// Arquivo config.toml a gravar quando `update_pin`.
    pub config_file: PathBuf,
}

/// Relatorio do rebase.
#[derive(Debug, Clone)]
pub struct RebaseReport {
    /// Ref anterior.
    pub old_ref: String,
    /// SHA anterior.
    pub old_sha: Sha40,
    /// Ref nova (ou curta derivada do SHA).
    pub new_ref: String,
    /// SHA novo normalizado.
    pub new_sha: Sha40,
    /// Pin gravado no config.toml.
    pub pinned: bool,
    /// Notas informativas.
    pub notes: Vec<String>,
}

/// Erros do rebase.
#[derive(Debug, Error)]
pub enum RebaseError {
    /// Workspace ausente ou sem git.
    #[error("upstream workspace not found: {0} (run dev-setup first)")]
    NoWorkspace(String),
    /// Worktree suja sem `--force`.
    #[error("the workspace {0} has uncommitted changes; commit or stash them before rebasing (use --force to discard)")]
    DirtyWorkspace(String),
    /// Nem ref nem sha informados.
    #[error("missing new ref (or use --sha)")]
    MissingRef,
    /// Ref desconhecida no upstream.
    #[error("ref not found upstream: {0}")]
    RefNotFound(String),
    /// Commit desconhecido.
    #[error("commit not found upstream: {0}")]
    CommitNotFound(String),
    /// SHA vindo do remoto fora do formato.
    #[error("the resolved commit is not a 40 character hex sha: {0}")]
    InvalidRemoteSha(String),
    /// Conflito no replay: workspace mantido, pin intacto.
    #[error("conflict while replaying the series at {entry}; resolve the markers, commit and refresh the patch (pin unchanged)")]
    Conflict {
        /// Entrada que conflitou.
        entry: String,
    },
    /// Falha nao-conflito no replay (pin intacto).
    #[error("series replay failed: {0} (pin unchanged)")]
    ApplyFailed(String),
    /// Erro do git.
    #[error(transparent)]
    Git(#[from] GitError),
    /// Erro de arquivos/config.
    #[error(transparent)]
    Repo(#[from] BrError),
}

fn workspace_display(target: &std::path::Path) -> String {
    target.display().to_string()
}

/// Resolve a nova base: tag, branch remoto ou `ls-remote`.
///
/// Ordem do shell: `refs/tags/<ref>`, `refs/remotes/origin/<ref>`,
/// `ls-remote <url> refs/tags/<ref>`. Retorna `(rotulo, sha)`.
pub fn resolve_new_base<G: GitBackend>(
    git: &G,
    target: &std::path::Path,
    upstream_url: &str,
    req: &RebaseRequest,
) -> Result<(String, Sha40), RebaseError> {
    if let Some(raw_sha) = &req.new_sha {
        let sha = git
            .rev_parse_opt(target, raw_sha)?
            .ok_or_else(|| RebaseError::CommitNotFound(raw_sha.clone()))?;
        let label = match &req.new_ref {
            Some(label) => label.clone(),
            None => git
                .tag_points_at(target, &sha.to_string())?
                .into_iter()
                .next()
                .unwrap_or_else(|| sha.short()),
        };
        return Ok((label, sha));
    }
    let new_ref = req.new_ref.clone().ok_or(RebaseError::MissingRef)?;
    let _ = git.fetch_tags(target);
    if let Some(sha) = git.rev_parse_opt(target, &format!("refs/tags/{new_ref}"))? {
        return Ok((new_ref, sha));
    }
    if let Some(sha) = git.rev_parse_opt(target, &format!("refs/remotes/origin/{new_ref}"))? {
        return Ok((new_ref, sha));
    }
    match git.ls_remote(
        target,
        upstream_url,
        &format!("refs/tags/{new_ref}^{{commit}}"),
    )? {
        Some(raw) => {
            let first = raw.split_whitespace().next().unwrap_or("");
            let sha =
                Sha40::parse(first).map_err(|_| RebaseError::InvalidRemoteSha(raw.clone()))?;
            Ok((new_ref, sha))
        }
        None => Err(RebaseError::RefNotFound(new_ref)),
    }
}

/// Move a base fixada e reaplica a serie.
pub fn run_rebase<G: GitBackend, FS: FileSystem>(
    git: &G,
    fs: &mut FS,
    paths: &RebasePaths,
    req: &RebaseRequest,
) -> Result<RebaseReport, RebaseError> {
    if !git.is_git_repo(&paths.target_dir) {
        return Err(RebaseError::NoWorkspace(workspace_display(
            &paths.target_dir,
        )));
    }
    if req.force {
        git.reset_hard_head(&paths.target_dir)?;
        git.clean(&paths.target_dir)?;
    } else if !git.status(&paths.target_dir)?.is_empty() {
        return Err(RebaseError::DirtyWorkspace(workspace_display(
            &paths.target_dir,
        )));
    }

    let (new_ref, new_sha) = resolve_new_base(git, &paths.target_dir, &paths.upstream_url, req)?;
    let mut report = RebaseReport {
        old_ref: paths.current_ref.clone(),
        old_sha: paths.current_sha,
        new_ref: new_ref.clone(),
        new_sha,
        pinned: false,
        notes: vec![
            format!(
                "current base: {} ({})",
                paths.current_sha, paths.current_ref
            ),
            format!("new base:     {new_sha} ({new_ref})"),
        ],
    };
    if req.dry_run {
        report.notes.push("dry run, nothing changed".to_string());
        return Ok(report);
    }

    let sha_text = new_sha.to_string();
    git.checkout_detach(&paths.target_dir, &sha_text)?;
    git.reset_hard(&paths.target_dir, &sha_text)?;
    git.clean(&paths.target_dir)?;

    let apply_paths = ApplyPaths {
        root: paths.root.clone(),
        target_dir: paths.target_dir.clone(),
        series_file: paths.series_file.clone(),
        overlays_dir: paths.overlays_dir.clone(),
        base_sha: new_sha,
    };
    match run_apply(git, fs, &apply_paths, &ApplyRequest::default()) {
        Ok(applied) => {
            report.notes.extend(applied.notes);
        }
        Err(jx_ops::apply::ApplyError::Conflict(entry)) => {
            return Err(RebaseError::Conflict { entry })
        }
        Err(other) => return Err(RebaseError::ApplyFailed(other.to_string())),
    }

    // O pin so e gravado depois que a serie aplica sem conflito.
    if req.update_pin {
        use jx_core::ConfigKey;
        let key_ref = ConfigKey::parse("upstream.ref")
            .map_err(|e| RebaseError::ApplyFailed(e.to_string()))?;
        let key_sha = ConfigKey::parse("upstream.sha")
            .map_err(|e| RebaseError::ApplyFailed(e.to_string()))?;
        jx_core::config_set(fs, &paths.config_file, &key_ref, &new_ref)?;
        jx_core::config_set(fs, &paths.config_file, &key_sha, &sha_text)?;
        report.pinned = true;
        report
            .notes
            .push(format!("config.toml updated to {new_ref} ({new_sha})"));
    }
    report
        .notes
        .push(format!("series replayed cleanly on {new_sha}"));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jx_core::MemFs;
    use jx_ops::support::ScriptGit;

    const BASE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const NEXT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn rebase_paths() -> RebasePaths {
        RebasePaths {
            root: PathBuf::from("/repo"),
            target_dir: PathBuf::from("/ws"),
            series_file: PathBuf::from("/repo/patches/series"),
            overlays_dir: PathBuf::from("/repo/overlays"),
            upstream_url: "https://exemplo.test/painel.git".to_string(),
            current_ref: "v4.0.7".to_string(),
            current_sha: Sha40::parse(BASE).unwrap(),
            config_file: PathBuf::from("/repo/config.toml"),
        }
    }

    fn seed(fs: &mut MemFs) {
        fs.write_string(
            std::path::Path::new("/repo/patches/series"),
            "patches/0001-a.patch\n",
        )
        .unwrap();
        fs.write_string(std::path::Path::new("/repo/patches/0001-a.patch"), "diff\n")
            .unwrap();
        fs.write_string(std::path::Path::new("/ws/.git/HEAD"), "ref\n")
            .unwrap();
        fs.write_string(
            std::path::Path::new("/repo/config.toml"),
            "[upstream]\nref = \"v4.0.7\"\nsha = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n",
        )
        .unwrap();
    }

    #[test]
    fn resolves_tag_branch_and_ls_remote_in_order() {
        let git = ScriptGit::default();
        // Tag local vence.
        let mut git = git;
        git.rev_map
            .insert("refs/tags/v4.0.8".to_string(), Sha40::parse(NEXT).unwrap());
        let req = RebaseRequest::pinned("v4.0.8");
        assert_eq!(
            resolve_new_base(&git, PathBuf::from("/ws").as_path(), "https://x", &req)
                .unwrap()
                .1
                .to_string(),
            NEXT
        );

        // Branch remoto quando a tag falta; ls-remote por ultimo.
        let mut git = ScriptGit::default();
        git.rev_map.insert(
            "refs/remotes/origin/main".to_string(),
            Sha40::parse(NEXT).unwrap(),
        );
        let req = RebaseRequest::pinned("main");
        assert_eq!(
            resolve_new_base(&git, PathBuf::from("/ws").as_path(), "https://x", &req)
                .unwrap()
                .1
                .to_string(),
            NEXT
        );

        let mut git = ScriptGit::default();
        git.ls_remote_sha = Some(format!("{NEXT}\trefs/tags/v9\n"));
        let req = RebaseRequest::pinned("v9");
        let (label, sha) =
            resolve_new_base(&git, PathBuf::from("/ws").as_path(), "https://x", &req).unwrap();
        assert_eq!((label.as_str(), sha.to_string().as_str()), ("v9", NEXT));

        // Nada em nenhum tier -> RefNotFound.
        let git = ScriptGit::default();
        let req = RebaseRequest::pinned("v404");
        assert!(matches!(
            resolve_new_base(&git, PathBuf::from("/ws").as_path(), "https://x", &req),
            Err(RebaseError::RefNotFound(_))
        ));
    }

    #[test]
    fn sha_mode_normalizes_and_derives_label() {
        let mut git = ScriptGit::default();
        git.rev_map
            .insert("bbbb".to_string(), Sha40::parse(NEXT).unwrap());
        git.tags = vec!["v4.0.8".to_string()];
        let req = RebaseRequest {
            new_sha: Some("bbbb".to_string()),
            update_pin: true,
            ..RebaseRequest::default()
        };
        let (label, sha) =
            resolve_new_base(&git, PathBuf::from("/ws").as_path(), "https://x", &req).unwrap();
        assert_eq!(label, "v4.0.8");
        assert_eq!(sha.to_string(), NEXT);

        // Sem tag e sem rotulo: curta de 12.
        let git = ScriptGit::default();
        let req = RebaseRequest {
            new_sha: Some(NEXT.to_string()),
            ..RebaseRequest::default()
        };
        // rev_parse_opt cai no parse direto do SHA completo.
        let (label, _) =
            resolve_new_base(&git, PathBuf::from("/ws").as_path(), "https://x", &req).unwrap();
        assert_eq!(label, &NEXT[..12]);
    }

    #[test]
    fn dry_run_resolves_without_mutation_and_pins_only_on_success() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        let mut git = ScriptGit::default();
        git.rev_map
            .insert("refs/tags/v4.0.8".to_string(), Sha40::parse(NEXT).unwrap());
        let req = RebaseRequest {
            new_ref: Some("v4.0.8".to_string()),
            dry_run: true,
            ..RebaseRequest::default()
        };
        let report = run_rebase(&git, &mut fs, &rebase_paths(), &req).unwrap();
        assert!(!report.pinned);
        assert!(report.notes.iter().any(|note| note.contains("dry run")));
        assert!(!git
            .calls
            .borrow()
            .iter()
            .any(|call| call.starts_with("checkout")));

        let req = RebaseRequest::pinned("v4.0.8");
        let report = run_rebase(&git, &mut fs, &rebase_paths(), &req).unwrap();
        assert!(report.pinned);
        let config = fs
            .read_to_string(std::path::Path::new("/repo/config.toml"))
            .unwrap();
        assert!(config.contains("ref = \"v4.0.8\""));
        assert!(config.contains(&format!("sha = \"{NEXT}\"")));
    }

    #[test]
    fn conflict_keeps_pin_untouched() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        let mut git = ScriptGit::default();
        git.rev_map
            .insert("refs/tags/v4.0.8".to_string(), Sha40::parse(NEXT).unwrap());
        git.check_reverse_ok
            .insert("/repo/patches/0001-a.patch".to_string(), false);
        git.apply_ok
            .insert("/repo/patches/0001-a.patch".to_string(), false);
        git.unmerged = vec!["UU a.ts".to_string()];
        let req = RebaseRequest::pinned("v4.0.8");
        let err = run_rebase(&git, &mut fs, &rebase_paths(), &req).unwrap_err();
        assert!(matches!(err, RebaseError::Conflict { .. }));
        let config = fs
            .read_to_string(std::path::Path::new("/repo/config.toml"))
            .unwrap();
        assert!(config.contains("sha = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\""));
    }
}
