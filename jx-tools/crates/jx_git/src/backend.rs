//! Backend git via processo, atras de trait para testes com fake.

use std::path::Path;
use std::process::Command;

use thiserror::Error;

use crate::sha::{InvalidSha, Sha40};
use crate::status::{parse_status_porcelain, InvalidStatus, StatusEntry};

/// Erros das operacoes git.
#[derive(Debug, Error)]
pub enum GitError {
    /// Falha ao gerar ou ler o processo `git`.
    #[error("git I/O failure for {0}: {1}")]
    Io(String, #[source] std::io::Error),
    /// Comando `git` saiu com status de erro.
    #[error("git {args} failed with status {status}: {stderr}")]
    CommandFailed {
        /// Argumentos invocados.
        args: String,
        /// Codigo de saida.
        status: String,
        /// Stderr aparado do comando.
        stderr: String,
    },
    /// Saida do git fora do formato esperado.
    #[error("invalid git output: {0}")]
    InvalidOutput(String),
    /// SHA invalido vindo do git.
    #[error(transparent)]
    InvalidSha(#[from] InvalidSha),
    /// Status porcelain malformado.
    #[error(transparent)]
    InvalidStatus(#[from] InvalidStatus),
}

impl GitError {
    /// Informa se e falha de comando (apply/check negativo) em vez de I/O.
    pub fn is_command_failure(&self) -> bool {
        matches!(self, Self::CommandFailed { .. })
    }
}

/// Operacoes git necessarias as ferramentas, injetaveis em testes.
///
/// Cada metodo espelha um comando usado pelos scripts; o tipo de retorno
/// tipado (`Sha40`, `StatusEntry`, `bool`) impede confundir saidas.
pub trait GitBackend {
    /// `git status --porcelain=v1` parseado.
    fn status(&self, dir: &Path) -> Result<Vec<StatusEntry>, GitError>;
    /// `git rev-parse --verify <rev>^{commit}` como SHA validado.
    fn rev_parse(&self, dir: &Path, rev: &str) -> Result<Sha40, GitError>;
    /// Informa se `<rev>:<path>` existe (`git cat-file -e`).
    fn cat_file_exists(&self, dir: &Path, rev: &str, path: &str) -> Result<bool, GitError>;
    /// `git apply --check [--reverse] <patch>`; erro = nao aplica.
    fn apply_check(&self, dir: &Path, patch: &Path, reverse: bool) -> Result<(), GitError>;
    /// `git apply <patch>`.
    fn apply(&self, dir: &Path, patch: &Path) -> Result<(), GitError>;
    /// `git apply --reverse <patch>`.
    fn apply_reverse(&self, dir: &Path, patch: &Path) -> Result<(), GitError>;
    /// `git apply --3way <patch>`; erro com `ls_unmerged` nao vazio = conflito.
    fn apply_3way(&self, dir: &Path, patch: &Path) -> Result<(), GitError>;
    /// Caminhos com conflito de merge (`git ls-files -u`).
    fn ls_unmerged(&self, dir: &Path) -> Result<Vec<String>, GitError>;
    /// `git add -A`.
    fn add_all(&self, dir: &Path) -> Result<(), GitError>;
    /// Commit com a identidade fixa do overlay.
    fn commit(&self, dir: &Path, message: &str) -> Result<(), GitError>;
    /// `git reset --hard <rev>`.
    fn reset_hard(&self, dir: &Path, rev: &str) -> Result<(), GitError>;
    /// `git reset --soft <rev>`.
    fn reset_soft(&self, dir: &Path, rev: &str) -> Result<(), GitError>;
    /// Desfaz o stage de um caminho (`git reset HEAD -- <path>`).
    fn reset_path(&self, dir: &Path, path: &str) -> Result<(), GitError>;
    /// `git clean -fdq`.
    fn clean(&self, dir: &Path) -> Result<(), GitError>;
    /// `git checkout --force --detach <rev>`.
    fn checkout_detach(&self, dir: &Path, rev: &str) -> Result<(), GitError>;
    /// SHA cheio do HEAD (`git rev-parse HEAD`).
    fn head_rev(&self, dir: &Path) -> Result<String, GitError>;
    /// Assunto do HEAD (`git log -1 --format=%s`).
    fn head_subject(&self, dir: &Path) -> Result<String, GitError>;
    /// Pai do HEAD (`HEAD^`); `None` quando nao existe.
    fn head_parent_rev(&self, dir: &Path) -> Result<Option<String>, GitError>;
    /// Assuntos de `git log --format=%s <range>`.
    fn log_subjects_range(&self, dir: &Path, range: &str) -> Result<Vec<String>, GitError>;
    /// SHA do primeiro commit cujo assunto casa com o padrao `--grep`.
    fn log_grep_commit(&self, dir: &Path, pattern: &str) -> Result<Option<String>, GitError>;
    /// `git diff --cached --binary --no-color HEAD`.
    fn diff_cached(&self, dir: &Path) -> Result<String, GitError>;
    /// `true` quando o caminho nao tem mudanca staged.
    fn diff_cached_quiet(&self, dir: &Path, path: &str) -> Result<bool, GitError>;
    /// `true` quando o caminho nao difere do HEAD.
    fn diff_quiet_head(&self, dir: &Path, path: &str) -> Result<bool, GitError>;
    /// Ignorados nao rastreados (`git ls-files --others --ignored`).
    fn ls_ignored_others(&self, dir: &Path) -> Result<Vec<String>, GitError>;
    /// `git clone --shared --no-checkout <src> <dst>`.
    fn clone_shared_no_checkout(&self, src: &Path, dst: &Path) -> Result<(), GitError>;
    /// `git clone --no-checkout <url> <dst>`.
    fn clone_no_checkout(&self, url: &str, dst: &Path) -> Result<(), GitError>;
    /// `git fetch --tags --force origin`.
    fn fetch_tags(&self, dir: &Path) -> Result<(), GitError>;
    /// `git config <chave>`; `None` quando ausente.
    fn git_config(&self, dir: &Path, key: &str) -> Result<Option<String>, GitError>;
    /// Reaponta um remote (`git remote set-url <nome> <url>`).
    fn remote_set_url(&self, dir: &Path, name: &str, url: &str) -> Result<(), GitError>;
    /// Primeira SHA de `git ls-remote <url> <padrao>`; `None` quando vazio.
    fn ls_remote(&self, dir: &Path, url: &str, pattern: &str) -> Result<Option<String>, GitError>;
    /// Tags que apontam para o commit (`git tag --points-at <sha>`).
    fn tag_points_at(&self, dir: &Path, sha: &str) -> Result<Vec<String>, GitError>;
    /// `rev-parse --verify <rev>` fallivel; `None` quando ausente.
    fn rev_parse_opt(&self, dir: &Path, rev: &str) -> Result<Option<Sha40>, GitError>;
    /// `git reset --hard` sem revisao (descarta, mantem o HEAD).
    fn reset_hard_head(&self, dir: &Path) -> Result<(), GitError>;
    /// Conteudo de `<rev>:<caminho>` (`git show`); `None` quando ausente.
    fn show_file(&self, dir: &Path, rev: &str, path: &str) -> Result<Option<String>, GitError>;
    /// Caminhos alterados contra a base (`git diff --name-only <rev>`).
    fn diff_name_only(&self, dir: &Path, rev: &str) -> Result<Vec<String>, GitError>;
    /// Informa se o diretorio e um repositorio git.
    fn is_git_repo(&self, dir: &Path) -> bool;
    /// Extrai `git archive <rev>` em `dest` (`git archive | tar -x`).
    fn archive_extract(&self, dir: &Path, rev: &str, dest: &Path) -> Result<(), GitError>;
    /// Diff entre dois objetos/especificacoes (`git diff --no-color -U2 <old_spec> <new_spec>`).
    fn diff_objects(
        &self,
        _dir: &Path,
        _old_spec: &str,
        _new_spec: &str,
    ) -> Result<Option<String>, GitError> {
        Ok(None)
    }
}

/// Identidade fixa dos commits de snapshot da serie.
pub const SNAPSHOT_AUTHOR_NAME: &str = "Jexactyl Overlay";
/// E-mail fixo dos commits de snapshot da serie.
pub const SNAPSHOT_AUTHOR_EMAIL: &str = "overlay@next-panel.local";

/// Backend real via processo `git`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessGit;

impl ProcessGit {
    /// Constroi o backend de processo.
    pub fn new() -> Self {
        Self
    }

    fn run(&self, dir: &Path, args: &[&str]) -> Result<String, GitError> {
        let label = args.join(" ");
        let output = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .map_err(|e| GitError::Io(label.clone(), e))?;
        if !output.status.success() {
            return Err(GitError::CommandFailed {
                args: label,
                status: output.status.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn run_tolerant(&self, dir: &Path, args: &[&str]) -> Result<Option<String>, GitError> {
        match self.run(dir, args) {
            Ok(out) => Ok(Some(out)),
            Err(GitError::CommandFailed { .. }) => Ok(None),
            Err(other) => Err(other),
        }
    }
}

impl GitBackend for ProcessGit {
    fn status(&self, dir: &Path) -> Result<Vec<StatusEntry>, GitError> {
        let out = self.run(dir, &["status", "--porcelain=v1"])?;
        parse_status_porcelain(&out).map_err(GitError::from)
    }

    fn rev_parse(&self, dir: &Path, rev: &str) -> Result<Sha40, GitError> {
        let spec = format!("{rev}^{{commit}}");
        let out = self.run(dir, &["rev-parse", "--verify", spec.as_str()])?;
        Sha40::parse(out.trim()).map_err(|_| {
            GitError::InvalidOutput(format!("rev-parse {rev} returned non-sha output"))
        })
    }

    fn cat_file_exists(&self, dir: &Path, rev: &str, path: &str) -> Result<bool, GitError> {
        let spec = format!("{rev}:{path}");
        Ok(self
            .run_tolerant(dir, &["cat-file", "-e", spec.as_str()])?
            .is_some())
    }

    fn apply_check(&self, dir: &Path, patch: &Path, reverse: bool) -> Result<(), GitError> {
        let path = patch.to_string_lossy().into_owned();
        if reverse {
            self.run(dir, &["apply", "--check", "--reverse", path.as_str()])?;
        } else {
            self.run(dir, &["apply", "--check", path.as_str()])?;
        }
        Ok(())
    }

    fn apply(&self, dir: &Path, patch: &Path) -> Result<(), GitError> {
        let path = patch.to_string_lossy().into_owned();
        self.run(dir, &["apply", "--verbose", path.as_str()])?;
        Ok(())
    }

    fn apply_reverse(&self, dir: &Path, patch: &Path) -> Result<(), GitError> {
        let path = patch.to_string_lossy().into_owned();
        self.run(dir, &["apply", "--verbose", "--reverse", path.as_str()])?;
        Ok(())
    }

    fn apply_3way(&self, dir: &Path, patch: &Path) -> Result<(), GitError> {
        let path = patch.to_string_lossy().into_owned();
        self.run(dir, &["apply", "--3way", path.as_str()])?;
        Ok(())
    }

    fn ls_unmerged(&self, dir: &Path) -> Result<Vec<String>, GitError> {
        let out = self.run(dir, &["ls-files", "-u"])?;
        Ok(out.lines().map(str::to_string).collect())
    }

    fn add_all(&self, dir: &Path) -> Result<(), GitError> {
        self.run(dir, &["add", "-A"])?;
        Ok(())
    }

    fn commit(&self, dir: &Path, message: &str) -> Result<(), GitError> {
        self.run(
            dir,
            &[
                "-c",
                &format!("user.name={SNAPSHOT_AUTHOR_NAME}"),
                "-c",
                &format!("user.email={SNAPSHOT_AUTHOR_EMAIL}"),
                "commit",
                "--quiet",
                "-m",
                message,
            ],
        )?;
        Ok(())
    }

    fn reset_hard(&self, dir: &Path, rev: &str) -> Result<(), GitError> {
        self.run(dir, &["reset", "--quiet", "--hard", rev])?;
        Ok(())
    }

    fn reset_soft(&self, dir: &Path, rev: &str) -> Result<(), GitError> {
        self.run(dir, &["reset", "--quiet", "--soft", rev])?;
        Ok(())
    }

    fn reset_path(&self, dir: &Path, path: &str) -> Result<(), GitError> {
        self.run(dir, &["reset", "--quiet", "HEAD", "--", path])?;
        Ok(())
    }

    fn clean(&self, dir: &Path) -> Result<(), GitError> {
        self.run(dir, &["clean", "-fdq"])?;
        Ok(())
    }

    fn checkout_detach(&self, dir: &Path, rev: &str) -> Result<(), GitError> {
        self.run(dir, &["checkout", "--quiet", "--force", "--detach", rev])?;
        Ok(())
    }

    fn head_rev(&self, dir: &Path) -> Result<String, GitError> {
        Ok(self.run(dir, &["rev-parse", "HEAD"])?.trim().to_string())
    }

    fn head_subject(&self, dir: &Path) -> Result<String, GitError> {
        Ok(self
            .run(dir, &["log", "-1", "--format=%s"])?
            .trim()
            .to_string())
    }

    fn head_parent_rev(&self, dir: &Path) -> Result<Option<String>, GitError> {
        Ok(self
            .run_tolerant(dir, &["rev-parse", "--verify", "HEAD^"])?
            .map(|out| out.trim().to_string()))
    }

    fn log_subjects_range(&self, dir: &Path, range: &str) -> Result<Vec<String>, GitError> {
        let out = self.run(dir, &["log", "--format=%s", range])?;
        Ok(out.lines().map(str::to_string).collect())
    }

    fn log_grep_commit(&self, dir: &Path, pattern: &str) -> Result<Option<String>, GitError> {
        let out = self.run(
            dir,
            &["log", "--format=%H", &format!("--grep={pattern}"), "-1"],
        )?;
        let sha = out.trim();
        Ok((!sha.is_empty()).then(|| sha.to_string()))
    }

    fn diff_cached(&self, dir: &Path) -> Result<String, GitError> {
        self.run(dir, &["diff", "--cached", "--binary", "--no-color", "HEAD"])
    }

    fn diff_cached_quiet(&self, dir: &Path, path: &str) -> Result<bool, GitError> {
        Ok(self
            .run_tolerant(dir, &["diff", "--cached", "--quiet", "HEAD", "--", path])?
            .is_some())
    }

    fn diff_quiet_head(&self, dir: &Path, path: &str) -> Result<bool, GitError> {
        Ok(self
            .run_tolerant(dir, &["diff", "--quiet", "HEAD", "--", path])?
            .is_some())
    }

    fn ls_ignored_others(&self, dir: &Path) -> Result<Vec<String>, GitError> {
        let out = self.run(
            dir,
            &["ls-files", "--others", "--ignored", "--exclude-standard"],
        )?;
        Ok(out.lines().map(str::to_string).collect())
    }

    fn clone_shared_no_checkout(&self, src: &Path, dst: &Path) -> Result<(), GitError> {
        let (src_text, dst_text) = (
            src.to_string_lossy().into_owned(),
            dst.to_string_lossy().into_owned(),
        );
        let output = Command::new("git")
            .args([
                "clone",
                "--quiet",
                "--shared",
                "--no-checkout",
                src_text.as_str(),
                dst_text.as_str(),
            ])
            .output()
            .map_err(|e| GitError::Io("clone --shared".to_string(), e))?;
        if !output.status.success() {
            return Err(GitError::CommandFailed {
                args: "clone --shared".to_string(),
                status: output.status.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        Ok(())
    }

    fn clone_no_checkout(&self, url: &str, dst: &Path) -> Result<(), GitError> {
        let dst_text = dst.to_string_lossy().into_owned();
        let output = Command::new("git")
            .args(["clone", "--quiet", "--no-checkout", url, dst_text.as_str()])
            .output()
            .map_err(|e| GitError::Io("clone".to_string(), e))?;
        if !output.status.success() {
            return Err(GitError::CommandFailed {
                args: "clone".to_string(),
                status: output.status.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        Ok(())
    }

    fn fetch_tags(&self, dir: &Path) -> Result<(), GitError> {
        self.run(dir, &["fetch", "--quiet", "--tags", "--force", "origin"])?;
        Ok(())
    }

    fn git_config(&self, dir: &Path, key: &str) -> Result<Option<String>, GitError> {
        Ok(self
            .run_tolerant(dir, &["config", key])?
            .map(|out| out.trim().to_string()))
    }

    fn remote_set_url(&self, dir: &Path, name: &str, url: &str) -> Result<(), GitError> {
        self.run(dir, &["remote", "set-url", name, url])?;
        Ok(())
    }

    fn ls_remote(&self, dir: &Path, url: &str, pattern: &str) -> Result<Option<String>, GitError> {
        let out = self.run(dir, &["ls-remote", url, pattern])?;
        Ok(out
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().next())
            .map(str::to_string))
    }

    fn tag_points_at(&self, dir: &Path, sha: &str) -> Result<Vec<String>, GitError> {
        let out = self.run(dir, &["tag", "--points-at", sha])?;
        Ok(out.lines().map(str::to_string).collect())
    }

    fn rev_parse_opt(&self, dir: &Path, rev: &str) -> Result<Option<Sha40>, GitError> {
        let spec = format!("{rev}^{{commit}}");
        match self.run_tolerant(dir, &["rev-parse", "--verify", spec.as_str()])? {
            Some(out) => Sha40::parse(out.trim()).map(Some).map_err(|_| {
                GitError::InvalidOutput(format!("rev-parse {rev} returned non-sha output"))
            }),
            None => Ok(None),
        }
    }

    fn reset_hard_head(&self, dir: &Path) -> Result<(), GitError> {
        self.run(dir, &["reset", "--quiet", "--hard"])?;
        Ok(())
    }

    fn is_git_repo(&self, dir: &Path) -> bool {
        self.run(dir, &["rev-parse", "--git-dir"]).is_ok()
    }

    fn diff_objects(
        &self,
        dir: &Path,
        old_spec: &str,
        new_spec: &str,
    ) -> Result<Option<String>, GitError> {
        self.run_tolerant(dir, &["diff", "--no-color", "-U2", old_spec, new_spec])
    }

    fn show_file(&self, dir: &Path, rev: &str, path: &str) -> Result<Option<String>, GitError> {
        let spec = format!("{rev}:{path}");
        match self.run(dir, &["show", spec.as_str()]) {
            Ok(out) => Ok(Some(out)),
            Err(GitError::CommandFailed { .. }) => Ok(None),
            Err(other) => Err(other),
        }
    }

    fn diff_name_only(&self, dir: &Path, rev: &str) -> Result<Vec<String>, GitError> {
        // Sem renames e com NUL como separador (igual ao Python:
        // `--name-only --no-renames -z`), para nao quebrar com espacos,
        // acentos ou quebras de linha em nomes de arquivo.
        let out = self.run(dir, &["diff", "--name-only", "--no-renames", "-z", rev])?;
        Ok(out
            .split('\0')
            .filter(|entry| !entry.is_empty())
            .map(str::to_string)
            .collect())
    }

    fn archive_extract(&self, dir: &Path, rev: &str, dest: &Path) -> Result<(), GitError> {
        use std::process::{Command, Stdio};
        std::fs::create_dir_all(dest).map_err(|e| GitError::Io(dest.display().to_string(), e))?;
        let mut producer = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["archive", "--format=tar", rev])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| GitError::Io("git archive".to_string(), e))?;
        let input = producer
            .stdout
            .take()
            .ok_or_else(|| GitError::InvalidOutput("git archive produced no stdout".to_string()))?;
        let consumer = Command::new("tar")
            .arg("-x")
            .arg("-C")
            .arg(dest)
            .stdin(input)
            .stderr(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .map_err(|e| GitError::Io("tar -x".to_string(), e))?;
        let consumer_out = consumer
            .wait_with_output()
            .map_err(|e| GitError::Io("tar -x".to_string(), e))?;
        let producer_out = producer
            .wait_with_output()
            .map_err(|e| GitError::Io("git archive".to_string(), e))?;
        if !consumer_out.status.success() {
            return Err(GitError::CommandFailed {
                args: format!("tar -x -C {}", dest.display()),
                status: consumer_out.status.to_string(),
                stderr: String::from_utf8_lossy(&consumer_out.stderr).into_owned(),
            });
        }
        if !producer_out.status.success() {
            return Err(GitError::CommandFailed {
                args: format!("git archive {rev}"),
                status: producer_out.status.to_string(),
                stderr: String::from_utf8_lossy(&producer_out.stderr).into_owned(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    /// Fake em memoria: prova que o trait e objetivamente testavel.
    #[derive(Debug, Default)]
    pub(crate) struct FakeGit {
        pub(crate) statuses: HashMap<PathBuf, Vec<StatusEntry>>,
    }

    impl GitBackend for FakeGit {
        fn status(&self, dir: &Path) -> Result<Vec<StatusEntry>, GitError> {
            Ok(self.statuses.get(dir).cloned().unwrap_or_default())
        }

        fn rev_parse(&self, _dir: &Path, rev: &str) -> Result<Sha40, GitError> {
            Sha40::parse(rev).map_err(|_| GitError::InvalidOutput(rev.to_string()))
        }

        fn cat_file_exists(&self, _dir: &Path, _rev: &str, path: &str) -> Result<bool, GitError> {
            Ok(path == "existe.ts")
        }

        fn apply_check(&self, _dir: &Path, _patch: &Path, _reverse: bool) -> Result<(), GitError> {
            Ok(())
        }

        fn apply(&self, _dir: &Path, _patch: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn apply_reverse(&self, _dir: &Path, _patch: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn apply_3way(&self, _dir: &Path, _patch: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn ls_unmerged(&self, _dir: &Path) -> Result<Vec<String>, GitError> {
            Ok(Vec::new())
        }

        fn add_all(&self, _dir: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn commit(&self, _dir: &Path, _message: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn reset_hard(&self, _dir: &Path, _rev: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn reset_soft(&self, _dir: &Path, _rev: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn reset_path(&self, _dir: &Path, _path: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn clean(&self, _dir: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn checkout_detach(&self, _dir: &Path, _rev: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn head_rev(&self, _dir: &Path) -> Result<String, GitError> {
            Ok("abc123".to_string())
        }

        fn head_subject(&self, _dir: &Path) -> Result<String, GitError> {
            Ok(String::new())
        }

        fn head_parent_rev(&self, _dir: &Path) -> Result<Option<String>, GitError> {
            Ok(None)
        }

        fn log_subjects_range(&self, _dir: &Path, _range: &str) -> Result<Vec<String>, GitError> {
            Ok(Vec::new())
        }

        fn log_grep_commit(&self, _dir: &Path, _pattern: &str) -> Result<Option<String>, GitError> {
            Ok(None)
        }

        fn diff_cached(&self, _dir: &Path) -> Result<String, GitError> {
            Ok(String::new())
        }

        fn diff_cached_quiet(&self, _dir: &Path, _path: &str) -> Result<bool, GitError> {
            Ok(true)
        }

        fn diff_quiet_head(&self, _dir: &Path, _path: &str) -> Result<bool, GitError> {
            Ok(true)
        }

        fn ls_ignored_others(&self, _dir: &Path) -> Result<Vec<String>, GitError> {
            Ok(Vec::new())
        }

        fn clone_shared_no_checkout(&self, _src: &Path, _dst: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn clone_no_checkout(&self, _url: &str, _dst: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn fetch_tags(&self, _dir: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn git_config(&self, _dir: &Path, _key: &str) -> Result<Option<String>, GitError> {
            Ok(None)
        }

        fn remote_set_url(&self, _dir: &Path, _name: &str, _url: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn ls_remote(
            &self,
            _dir: &Path,
            _url: &str,
            _pattern: &str,
        ) -> Result<Option<String>, GitError> {
            Ok(None)
        }

        fn tag_points_at(&self, _dir: &Path, _sha: &str) -> Result<Vec<String>, GitError> {
            Ok(Vec::new())
        }

        fn rev_parse_opt(&self, _dir: &Path, rev: &str) -> Result<Option<Sha40>, GitError> {
            Ok(Sha40::parse(rev).ok())
        }

        fn reset_hard_head(&self, _dir: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn is_git_repo(&self, _dir: &Path) -> bool {
            true
        }

        fn show_file(
            &self,
            _dir: &Path,
            _rev: &str,
            _path: &str,
        ) -> Result<Option<String>, GitError> {
            Ok(None)
        }

        fn diff_name_only(&self, _dir: &Path, _rev: &str) -> Result<Vec<String>, GitError> {
            Ok(Vec::new())
        }

        fn archive_extract(&self, _dir: &Path, _rev: &str, _dest: &Path) -> Result<(), GitError> {
            Ok(())
        }
    }

    fn backend_uses_dyn_dispatch(backend: &dyn GitBackend, dir: &Path) -> bool {
        StatusEntry::is_clean(&backend.status(dir).unwrap_or_default())
    }

    #[test]
    fn fake_git_drives_trait() {
        let mut fake = FakeGit::default();
        let dir = PathBuf::from("/ws");
        assert!(backend_uses_dyn_dispatch(&fake, &dir));
        fake.statuses
            .insert(dir.clone(), parse_status_porcelain(" M a.ts\n").unwrap());
        assert!(!backend_uses_dyn_dispatch(&fake, &dir));
        assert!(fake.cat_file_exists(&dir, "HEAD", "existe.ts").unwrap());
        assert!(!fake.cat_file_exists(&dir, "HEAD", "falta.ts").unwrap());
    }
}
