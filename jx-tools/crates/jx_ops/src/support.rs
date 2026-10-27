//! Git scriptado em memoria para os testes de `jx_ops`.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use jx_git::{GitBackend, GitError, Sha40, StatusEntry};

/// Fake com comportamento programavel por caminho de patch.
///
/// `applied` registra patches aplicados (para idempotencia), `dirty`
/// marca worktrees sujasapos mutacao; `calls` registra a ordem e
/// `commits` guarda as mensagens de snapshot.
#[derive(Debug, Default)]
pub struct ScriptGit {
    /// HEAD atual (texto livre hex nos testes).
    pub head: String,
    /// Assunto do HEAD (padrao: assunto `br: ` valido).
    pub subject: Option<String>,
    /// `apply --check` reverso por patch: `true` = ja aplicado.
    pub check_reverse_ok: HashMap<String, bool>,
    /// `apply --check` direto por patch.
    pub check_ok: HashMap<String, bool>,
    /// `apply` direto por patch.
    pub apply_ok: HashMap<String, bool>,
    /// `apply --3way` por patch.
    pub three_way_ok: HashMap<String, bool>,
    /// Saida de `ls-files -u`.
    pub unmerged: Vec<String>,
    /// Status por diretorio (vence o marcador `dirty`).
    pub statuses: HashMap<PathBuf, Vec<StatusEntry>>,
    /// `log A..B` so com `br: ` quando `true`.
    pub log_range_ok: bool,
    /// Retorno scriptado de `ls-remote`.
    pub ls_remote_sha: Option<String>,
    /// Retorno scriptado de `tag --points-at`.
    pub tags: Vec<String>,
    /// Mapa de `rev-parse --verify` (especificacao -> SHA).
    pub rev_map: HashMap<String, Sha40>,
    /// Mapa de `log --grep=<padrao>` (padrao -> SHA).
    pub grep_commits: HashMap<String, String>,
    /// Pai do HEAD (`rev-parse HEAD^`).
    pub head_parent: Option<String>,
    /// Conteudo de `<rev>:<path>` para `show_file` scriptado.
    pub show_files: std::collections::HashMap<String, String>,
    /// Saida scriptada de `diff --name-only`.
    pub diff_names: Vec<String>,
    /// Quando `true`, `is_git_repo` retorna `false`.
    pub not_a_repo: bool,
    /// Quando `true`, `fetch_tags` falha (aviso, sem aborto).
    pub fetch_fail: bool,
    /// Quando `true`, `status` sempre relata worktree suja.
    /// `reset_hard_head`/`clean` zeram (descarte do `--force`).
    pub status_dirty: std::cell::Cell<bool>,
    /// Patches considerados aplicados.
    applied: RefCell<HashSet<String>>,
    /// Worktrees sujas.
    dirty: RefCell<HashSet<PathBuf>>,
    /// Clone ja executado (vira repositorio).
    cloned: RefCell<bool>,
    /// Chamadas registradas em ordem.
    pub calls: RefCell<Vec<String>>,
    /// Mensagens de commit registradas.
    pub commits: RefCell<Vec<String>>,
}

impl ScriptGit {
    fn key(patch: &Path) -> String {
        patch.to_string_lossy().into_owned()
    }

    fn fail(&self, op: &str, patch: &Path) -> GitError {
        GitError::CommandFailed {
            args: format!("{op} {}", patch.display()),
            status: "exit status: 1".to_string(),
            stderr: String::new(),
        }
    }

    fn clean(&self, dir: &Path) -> bool {
        !self.statuses.contains_key(dir) && !self.dirty.borrow().contains(dir)
    }
}

impl GitBackend for ScriptGit {
    fn status(&self, dir: &Path) -> Result<Vec<StatusEntry>, GitError> {
        if let Some(entries) = self.statuses.get(dir) {
            return Ok(entries.clone());
        }
        if self.status_dirty.get() || self.dirty.borrow().contains(dir) {
            return Ok(vec![StatusEntry {
                index: ' ',
                worktree: 'M',
                path: "sujo.ts".to_string(),
                renamed_from: None,
            }]);
        }
        Ok(Vec::new())
    }

    fn rev_parse(&self, _dir: &Path, rev: &str) -> Result<Sha40, GitError> {
        Sha40::parse(rev).map_err(|_| GitError::InvalidOutput(rev.to_string()))
    }

    fn cat_file_exists(&self, _dir: &Path, _rev: &str, _path: &str) -> Result<bool, GitError> {
        Ok(false)
    }

    fn apply_check(&self, _dir: &Path, patch: &Path, reverse: bool) -> Result<(), GitError> {
        let key = Self::key(patch);
        let applied = self.applied.borrow().contains(&key);
        let ok = if reverse {
            self.check_reverse_ok.get(&key).copied().unwrap_or(applied)
        } else if applied {
            false
        } else {
            self.check_ok.get(&key).copied().unwrap_or(true)
        };
        self.calls.borrow_mut().push(format!(
            "check{} {key}",
            if reverse { "-reverse" } else { "" }
        ));
        if ok {
            Ok(())
        } else {
            Err(self.fail("apply --check", patch))
        }
    }

    fn apply(&self, dir: &Path, patch: &Path) -> Result<(), GitError> {
        let key = Self::key(patch);
        self.calls.borrow_mut().push(format!("apply {key}"));
        let allowed = self.check_ok.get(&key).copied().unwrap_or(true)
            && self.apply_ok.get(&key).copied().unwrap_or(true)
            && !self.applied.borrow().contains(&key);
        if allowed {
            self.applied.borrow_mut().insert(key);
            self.dirty.borrow_mut().insert(dir.to_path_buf());
            Ok(())
        } else {
            Err(self.fail("apply", patch))
        }
    }

    fn apply_reverse(&self, dir: &Path, patch: &Path) -> Result<(), GitError> {
        let key = Self::key(patch);
        self.calls.borrow_mut().push(format!("reverse {key}"));
        self.applied.borrow_mut().remove(&key);
        if self.applied.borrow().is_empty() {
            self.dirty.borrow_mut().remove(dir);
        }
        Ok(())
    }

    fn apply_3way(&self, dir: &Path, patch: &Path) -> Result<(), GitError> {
        let key = Self::key(patch);
        self.calls.borrow_mut().push(format!("3way {key}"));
        if self.three_way_ok.get(&key).copied().unwrap_or(false) {
            self.applied.borrow_mut().insert(key);
            self.dirty.borrow_mut().insert(dir.to_path_buf());
            Ok(())
        } else {
            Err(self.fail("apply --3way", patch))
        }
    }

    fn ls_unmerged(&self, _dir: &Path) -> Result<Vec<String>, GitError> {
        Ok(self.unmerged.clone())
    }

    fn add_all(&self, _dir: &Path) -> Result<(), GitError> {
        self.calls.borrow_mut().push("add".to_string());
        Ok(())
    }

    fn commit(&self, dir: &Path, message: &str) -> Result<(), GitError> {
        self.calls.borrow_mut().push(format!("commit {message}"));
        self.commits.borrow_mut().push(message.to_string());
        self.dirty.borrow_mut().remove(dir);
        Ok(())
    }

    fn reset_hard(&self, dir: &Path, rev: &str) -> Result<(), GitError> {
        self.calls.borrow_mut().push(format!("reset-hard {rev}"));
        self.dirty.borrow_mut().remove(dir);
        Ok(())
    }

    fn reset_soft(&self, dir: &Path, rev: &str) -> Result<(), GitError> {
        self.calls.borrow_mut().push(format!("reset-soft {rev}"));
        self.dirty.borrow_mut().insert(dir.to_path_buf());
        Ok(())
    }

    fn reset_path(&self, _dir: &Path, path: &str) -> Result<(), GitError> {
        self.calls.borrow_mut().push(format!("reset-path {path}"));
        Ok(())
    }

    fn clean(&self, dir: &Path) -> Result<(), GitError> {
        self.calls.borrow_mut().push("clean".to_string());
        self.dirty.borrow_mut().remove(dir);
        self.status_dirty.set(false);
        Ok(())
    }

    fn checkout_detach(&self, dir: &Path, rev: &str) -> Result<(), GitError> {
        self.calls.borrow_mut().push(format!("checkout {rev}"));
        self.dirty.borrow_mut().remove(dir);
        Ok(())
    }

    fn head_rev(&self, _dir: &Path) -> Result<String, GitError> {
        Ok(self.head.clone())
    }

    fn head_subject(&self, _dir: &Path) -> Result<String, GitError> {
        Ok(self
            .subject
            .clone()
            .unwrap_or_else(|| "br: patch 0001-a.patch".to_string()))
    }

    fn head_parent_rev(&self, _dir: &Path) -> Result<Option<String>, GitError> {
        Ok(self.head_parent.clone())
    }

    fn log_subjects_range(&self, _dir: &Path, _range: &str) -> Result<Vec<String>, GitError> {
        Ok(if self.log_range_ok {
            vec!["br: patch 0001-a.patch".to_string()]
        } else {
            vec!["trabalho externo".to_string()]
        })
    }

    fn log_grep_commit(&self, _dir: &Path, pattern: &str) -> Result<Option<String>, GitError> {
        Ok(self.grep_commits.get(pattern).cloned())
    }

    fn diff_cached(&self, _dir: &Path) -> Result<String, GitError> {
        Ok(if self.clean(_dir) {
            String::new()
        } else {
            "diff --git a/x b/x\n".to_string()
        })
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
        *self.cloned.borrow_mut() = true;
        self.calls.borrow_mut().push("clone-shared".to_string());
        Ok(())
    }

    fn clone_no_checkout(&self, _url: &str, _dst: &Path) -> Result<(), GitError> {
        *self.cloned.borrow_mut() = true;
        self.calls.borrow_mut().push("clone-remote".to_string());
        Ok(())
    }

    fn fetch_tags(&self, _dir: &Path) -> Result<(), GitError> {
        if self.fetch_fail {
            return Err(GitError::CommandFailed {
                args: "fetch".to_string(),
                status: "exit status: 128".to_string(),
                stderr: "could not read from remote".to_string(),
            });
        }
        self.calls.borrow_mut().push("fetch".to_string());
        Ok(())
    }

    fn git_config(&self, _dir: &Path, _key: &str) -> Result<Option<String>, GitError> {
        Ok(None)
    }

    fn remote_set_url(&self, _dir: &Path, name: &str, url: &str) -> Result<(), GitError> {
        self.calls
            .borrow_mut()
            .push(format!("remote-set-url {name} {url}"));
        Ok(())
    }

    fn ls_remote(
        &self,
        _dir: &Path,
        _url: &str,
        _pattern: &str,
    ) -> Result<Option<String>, GitError> {
        Ok(self.ls_remote_sha.clone())
    }

    fn tag_points_at(&self, _dir: &Path, _sha: &str) -> Result<Vec<String>, GitError> {
        Ok(self.tags.clone())
    }

    fn rev_parse_opt(&self, _dir: &Path, rev: &str) -> Result<Option<Sha40>, GitError> {
        if let Some(sha) = self.rev_map.get(rev) {
            return Ok(Some(*sha));
        }
        let bare = rev.strip_suffix("^{commit}").unwrap_or(rev);
        Ok(Sha40::parse(bare).ok())
    }

    fn reset_hard_head(&self, dir: &Path) -> Result<(), GitError> {
        self.calls.borrow_mut().push("reset-hard HEAD".to_string());
        self.dirty.borrow_mut().remove(dir);
        self.status_dirty.set(false);
        Ok(())
    }

    fn is_git_repo(&self, _dir: &Path) -> bool {
        *self.cloned.borrow() || !self.not_a_repo
    }

    fn show_file(&self, _dir: &Path, _rev: &str, path: &str) -> Result<Option<String>, GitError> {
        Ok(self.show_files.get(path).cloned())
    }

    fn diff_name_only(&self, _dir: &Path, _rev: &str) -> Result<Vec<String>, GitError> {
        Ok(self.diff_names.clone())
    }

    fn archive_extract(&self, _dir: &Path, rev: &str, dest: &Path) -> Result<(), GitError> {
        self.calls
            .borrow_mut()
            .push(format!("archive {rev} -> {}", dest.display()));
        Ok(())
    }
}
