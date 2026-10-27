//! Bootstrap do workspace: clone, pin, serie e dependencias opcionais.
//!
//! Espelha `scripts/dev-setup.sh`. Delega a aplicacao para `jx_ops::apply`.

use std::path::{Path, PathBuf};

use thiserror::Error;

use jx_core::{require_inside_root, resolve_path, BrError, FileSystem};
use jx_git::{GitBackend, GitError, Sha40};
use jx_ops::apply::{run_apply, ApplyPaths, ApplyRequest};
use jx_patch::parse_series;

/// Resultado de uma ferramenta externa opcional (`composer`, `pnpm`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmdOutcome {
    /// Ferramenta ausente: pula com aviso.
    SkippedMissing,
    /// Execucao com sucesso.
    Success,
    /// Falha demovida para aviso (mensagem ja formatada).
    FailedWarning(String),
}

/// Execucao injetavel de ferramentas externas.
pub trait CmdRunner {
    /// Roda `tool args...` em `cwd`.
    fn run_tool(&self, cwd: &Path, tool: &str, args: &[&str]) -> CmdOutcome;
}

/// Pedido de bootstrap (flags do CLI).
#[derive(Debug, Clone, Default)]
pub struct SetupRequest {
    /// Roda `composer install` e `pnpm install` no final.
    pub with_deps: bool,
    /// Recria o workspace do zero, descartando mudancas.
    pub force: bool,
    /// Ref alternativa do upstream nesta execucao.
    pub ref_override: Option<String>,
    /// Commit alternativo do upstream nesta execucao.
    pub sha_override: Option<Sha40>,
    /// Mirror local para o clone inicial.
    pub local: Option<PathBuf>,
}

/// Caminhos e pin do bootstrap.
#[derive(Debug, Clone)]
pub struct SetupPaths {
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
    /// Ref fixada (rotulo do log e fallback de fetch).
    pub base_ref: String,
    /// Commit fixado.
    pub base_sha: Sha40,
}

/// Relatorio do bootstrap.
#[derive(Debug, Clone, Default)]
pub struct SetupReport {
    /// Workspace reutilizado (em vez de clonado).
    pub reused: bool,
    /// Avisos nao fatais (fetch, deps).
    pub warnings: Vec<String>,
    /// Notas informativas (mirror, prontidao).
    pub notes: Vec<String>,
    /// Patches listados no series.
    pub patch_count: usize,
    /// Arquivos de overlay.
    pub overlay_count: usize,
}

/// Erros do bootstrap.
#[derive(Debug, Error)]
pub enum SetupError {
    /// Diretorio existe mas nao e repositorio git.
    #[error("upstream workspace exists but is not a git repository: {0}; remove it and run dev-setup again")]
    NotAGitRepo(String),
    /// Worktree suja sem `--force`.
    #[error("the workspace {0} has uncommitted changes; commit or stash them before running dev-setup (use --force to discard)")]
    DirtyWorkspace(String),
    /// Remocao fora da raiz recusada.
    #[error(transparent)]
    OutsideRoot(#[from] BrError),
    /// Pin inacessivel mesmo apos fetch.
    #[error("pinned commit not available: {0} (check upstream.sha in config.toml)")]
    PinUnreachable(String),
    /// Falha ao aplicar a serie.
    #[error(transparent)]
    Apply(#[from] jx_ops::apply::ApplyError),
    /// Erro do git.
    #[error(transparent)]
    Git(#[from] GitError),
}

fn workspace_display(target: &Path) -> String {
    target.display().to_string()
}

/// Prepara o workspace: clone/checkout, pin, serie, deps opcionais.
pub fn run_setup<G: GitBackend, FS: FileSystem, C: CmdRunner>(
    git: &G,
    fs: &mut FS,
    cmd: &C,
    paths: &SetupPaths,
    req: &SetupRequest,
) -> Result<SetupReport, SetupError> {
    let mut report = SetupReport::default();
    let upstream_ref = req
        .ref_override
        .clone()
        .unwrap_or_else(|| paths.base_ref.clone());
    let upstream_sha = req.sha_override.unwrap_or(paths.base_sha);

    let mut local = req.local.clone();
    if let Some(mirror) = &local {
        if !fs.exists(&mirror.join(".git")) {
            report.warnings.push(format!(
                "local mirror not found, falling back to the remote URL: {}",
                mirror.display()
            ));
            local = None;
        }
    }

    if req.force && fs.exists(&paths.target_dir) {
        let canonical = require_inside_root(&paths.root, &paths.target_dir)?;
        let _ = canonical;
        report.warnings.push(format!(
            "removing workspace: {}",
            workspace_display(&paths.target_dir)
        ));
        fs.remove_dir_all(&paths.target_dir)
            .map_err(SetupError::OutsideRoot)?;
    }

    if fs.exists(&paths.target_dir) && !git.is_git_repo(&paths.target_dir) {
        return Err(SetupError::NotAGitRepo(workspace_display(
            &paths.target_dir,
        )));
    }

    if !git.is_git_repo(&paths.target_dir) {
        if let Some(parent) = paths.target_dir.parent() {
            if !parent.as_os_str().is_empty() {
                fs.create_dir_all(parent).map_err(SetupError::OutsideRoot)?;
            }
        }
        match &local {
            Some(mirror) => {
                report
                    .notes
                    .push(format!("using local mirror: {}", mirror.display()));
                git.clone_no_checkout(&mirror.to_string_lossy(), &paths.target_dir)?;
                git.remote_set_url(&paths.target_dir, "origin", &paths.upstream_url)?;
            }
            None => {
                report.notes.push(format!(
                    "cloning upstream into {}",
                    workspace_display(&paths.target_dir)
                ));
                git.clone_no_checkout(&paths.upstream_url, &paths.target_dir)?;
            }
        }
    } else {
        report.reused = true;
        report.notes.push(format!(
            "reusing existing workspace: {}",
            workspace_display(&paths.target_dir)
        ));
        if !req.force && !git.status(&paths.target_dir)?.is_empty() {
            return Err(SetupError::DirtyWorkspace(workspace_display(
                &paths.target_dir,
            )));
        }
        if git.fetch_tags(&paths.target_dir).is_err() {
            report
                .warnings
                .push("fetch failed, continuing with the local objects".to_string());
        }
    }

    let sha_text = upstream_sha.to_string();
    if git.rev_parse(&paths.target_dir, &sha_text).is_err() {
        report.warnings.push(format!(
            "pinned commit not present, fetching {upstream_ref}"
        ));
        if git.fetch_tags(&paths.target_dir).is_err() {
            report
                .warnings
                .push("fetch failed, the pinned commit may be missing".to_string());
        }
    }
    if git.rev_parse(&paths.target_dir, &sha_text).is_err() {
        return Err(SetupError::PinUnreachable(sha_text));
    }

    git.checkout_detach(&paths.target_dir, &sha_text)?;
    git.reset_hard(&paths.target_dir, &sha_text)?;
    git.clean(&paths.target_dir)?;

    // Paridade consciente: o checkout usa o SHA efetivo (override), mas a
    // validacao de overlays/reverso le o pin do config, como o shell.
    let apply_paths = ApplyPaths {
        root: paths.root.clone(),
        target_dir: paths.target_dir.clone(),
        series_file: paths.series_file.clone(),
        overlays_dir: paths.overlays_dir.clone(),
        base_sha: paths.base_sha,
    };
    let applied = run_apply(git, fs, &apply_paths, &ApplyRequest::default())?;
    report.notes.extend(applied.notes);
    report.warnings.extend(applied.warnings);

    if req.with_deps {
        install_deps(
            cmd,
            &paths.target_dir,
            "composer",
            &["install", "--no-interaction", "--no-progress"],
            "php",
            &mut report,
        );
        install_deps(
            cmd,
            &paths.target_dir,
            "pnpm",
            &["install", "--frozen-lockfile"],
            "node",
            &mut report,
        );
    }

    let series_text = fs
        .read_to_string(&paths.series_file)
        .map_err(SetupError::OutsideRoot)?;
    report.patch_count = parse_series(&series_text).len();
    report.overlay_count = fs
        .list_files_recursive(&paths.overlays_dir)
        .map_err(SetupError::OutsideRoot)?
        .len();
    report.notes.push(format!(
        "workspace ready: {}",
        workspace_display(&paths.target_dir)
    ));
    report
        .notes
        .push(format!("base commit: {upstream_sha} ({upstream_ref})"));
    report.notes.push(format!(
        "patches: {} | overlays: {}",
        report.patch_count, report.overlay_count
    ));
    Ok(report)
}

/// Roda uma ferramenta opcional, demovendo ausencia e falha para avisos.
fn install_deps<C: CmdRunner>(
    cmd: &C,
    target: &Path,
    tool: &str,
    args: &[&str],
    kind: &str,
    report: &mut SetupReport,
) {
    match cmd.run_tool(target, tool, args) {
        CmdOutcome::Success => report.notes.push(format!("installing {kind} dependencies")),
        CmdOutcome::SkippedMissing => report
            .warnings
            .push(format!("{tool} not found, skipping {kind} dependencies")),
        CmdOutcome::FailedWarning(_) => report.warnings.push(format!("{tool} install failed")),
    }
}

/// Resolve um caminho de CLI contra a raiz (para `--local`).
pub fn resolve_cli_path(root: &Path, raw: &str) -> Result<PathBuf, BrError> {
    resolve_path(root, raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jx_core::MemFs;
    use jx_ops::support::ScriptGit;
    use std::path::Path;

    const BASE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn setup_paths() -> SetupPaths {
        SetupPaths {
            root: PathBuf::from("/repo"),
            target_dir: PathBuf::from("/repo/.upstream"),
            series_file: PathBuf::from("/repo/patches/series"),
            overlays_dir: PathBuf::from("/repo/overlays"),
            upstream_url: "https://exemplo.test/painel.git".to_string(),
            base_ref: "v4.0.7".to_string(),
            base_sha: Sha40::parse(BASE).unwrap(),
        }
    }

    fn seed(fs: &mut MemFs) {
        for (path, content) in [
            ("/repo/patches/series", "patches/0001-a.patch\n"),
            ("/repo/patches/0001-a.patch", "diff\n"),
            ("/repo/config.toml", "[upstream]\nsha = \"x\"\n"),
        ] {
            fs.write_string(Path::new(path), content).unwrap();
        }
    }

    /// Runner scriptado: sucesso salvo quando listado.
    struct FakeCmd {
        available: Vec<String>,
        failing: Vec<String>,
    }

    impl CmdRunner for FakeCmd {
        fn run_tool(&self, _cwd: &Path, tool: &str, _args: &[&str]) -> CmdOutcome {
            if self.failing.contains(&tool.to_string()) {
                return CmdOutcome::FailedWarning(format!("{tool} quebrou"));
            }
            if self.available.contains(&tool.to_string()) {
                CmdOutcome::Success
            } else {
                CmdOutcome::SkippedMissing
            }
        }
    }

    #[test]
    fn fresh_clone_uses_remote_and_applies() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        let mut git = ScriptGit::default();
        git.not_a_repo = true;
        let cmd = FakeCmd {
            available: vec![],
            failing: vec![],
        };
        let report = run_setup(
            &git,
            &mut fs,
            &cmd,
            &setup_paths(),
            &SetupRequest::default(),
        )
        .unwrap();
        assert!(!report.reused);
        assert!(report
            .notes
            .iter()
            .any(|note| note.contains("cloning upstream")));
        assert!(git.calls.borrow().iter().any(|call| call == "clone-remote"));
        assert_eq!(report.patch_count, 1);
    }

    #[test]
    fn local_mirror_rewires_origin_and_missing_mirror_warns() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        fs.write_string(Path::new("/mirror/.git/HEAD"), "ref\n")
            .unwrap();
        let mut git = ScriptGit::default();
        git.not_a_repo = true;
        let cmd = FakeCmd {
            available: vec![],
            failing: vec![],
        };
        let req = SetupRequest {
            local: Some(PathBuf::from("/mirror")),
            ..SetupRequest::default()
        };
        let report = run_setup(&git, &mut fs, &cmd, &setup_paths(), &req).unwrap();
        assert!(git
            .calls
            .borrow()
            .iter()
            .any(|call| call.starts_with("remote-set-url origin")));
        assert!(report
            .notes
            .iter()
            .any(|note| note.contains("local mirror")));

        let mut fs = MemFs::default();
        seed(&mut fs);
        let req = SetupRequest {
            local: Some(PathBuf::from("/ausente")),
            ..SetupRequest::default()
        };
        let report = run_setup(&git, &mut fs, &cmd, &setup_paths(), &req).unwrap();
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("falling back")));
    }

    #[test]
    fn reuse_refuses_dirty_without_force_and_warns_on_fetch_failure() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        fs.write_string(Path::new("/repo/.upstream/.git/HEAD"), "ref\n")
            .unwrap();
        let mut git = ScriptGit::default();
        git.fetch_fail = true;
        let cmd = FakeCmd {
            available: vec![],
            failing: vec![],
        };
        let report = run_setup(
            &git,
            &mut fs,
            &cmd,
            &setup_paths(),
            &SetupRequest::default(),
        )
        .unwrap();
        assert!(report.reused);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("continuing with the local objects")));

        let git = ScriptGit::default();
        git.status_dirty.set(true);
        let err = run_setup(
            &git,
            &mut fs,
            &cmd,
            &setup_paths(),
            &SetupRequest::default(),
        )
        .unwrap_err();
        assert!(matches!(err, SetupError::DirtyWorkspace(_)));

        let req = SetupRequest {
            force: true,
            ..SetupRequest::default()
        };
        let report = run_setup(&git, &mut fs, &cmd, &setup_paths(), &req).unwrap();
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("removing workspace")));
    }

    #[test]
    fn with_deps_demotes_missing_and_failure_to_warnings() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        let git = ScriptGit::default();
        let cmd = FakeCmd {
            available: vec!["composer".to_string()],
            failing: vec!["pnpm".to_string()],
        };
        let req = SetupRequest {
            with_deps: true,
            ..SetupRequest::default()
        };
        let report = run_setup(&git, &mut fs, &cmd, &setup_paths(), &req).unwrap();
        assert!(report
            .notes
            .iter()
            .any(|note| note.contains("installing php")));
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("pnpm install failed")));
    }
}
