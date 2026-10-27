//! Geracao de patches novos e refresh do ultimo patch.
//!
//! Espelha `scripts/dump-patch.sh`: modo novo (dump do delta pendente como
//! patch numerado + commit de baseline + registro no series) e modo refresh
//! (regenera o diff do ultimo patch preservando o cabecalho).

use std::path::PathBuf;

use thiserror::Error;

use jx_core::{format_rfc2822, format_ymd, Clock, FileSystem};
use jx_git::{GitBackend, GitError, Sha40};
use jx_patch::dep3::{AppliedUpstream, Forwarded, Origin};
use jx_patch::series::{
    domain_label, insert_into_text, next_after, parse_series, PatchNumber, SeriesEntry,
};

/// Autor padrao quando o git nao tem `user.name` configurado.
pub const FALLBACK_AUTHOR_NAME: &str = "Jexactyl";
/// E-mail padrao dos cabecalhos `From:`/`Signed-off-by:`.
pub const FALLBACK_AUTHOR_EMAIL: &str = "overlay@next-panel.local";
/// Descricao padrao do modo novo.
pub const DEFAULT_DESCRIPTION: &str =
    "Patch gerado a partir do workspace do upstream. Revise o conteudo antes de enviar.";

/// Pedido de patch novo.
#[derive(Debug, Clone)]
pub struct NewPatchRequest {
    /// Slug kebab-case (sem o prefixo de dominio).
    pub slug: String,
    /// Dominio de dois digitos (`01` i18n, `02` gateway, ...).
    pub domain: String,
    /// Assunto de uma linha (padrao: o slug).
    pub message: Option<String>,
    /// Descricao longa (padrao: texto gerado).
    pub description: Option<String>,
    /// Origem DEP-3.
    pub origin: Origin,
    /// Issue de acompanhamento (`Bug-Next-Panel`).
    pub bug: Option<String>,
    /// Linha `Signed-off-by` (padrao: autor do git).
    pub signoff: Option<String>,
    /// Estado no upstream.
    pub applied_upstream: AppliedUpstream,
    /// Registra no series (padrao do shell).
    pub add_to_series: bool,
}

impl Default for NewPatchRequest {
    /// Padroes do `dump-patch.sh`: origem `vendor`, sem serie extra.
    fn default() -> Self {
        Self {
            slug: String::new(),
            domain: "01".to_string(),
            message: None,
            description: None,
            origin: Origin::Vendor,
            bug: None,
            signoff: None,
            applied_upstream: AppliedUpstream::No,
            add_to_series: true,
        }
    }
}

/// Pedido de refresh de um patch da serie.
#[derive(Debug, Clone)]
pub struct RefreshRequest {
    /// Entrada do series a regenerar (ex.: `patches/0004-i18n-shared-runtime.patch`).
    pub entry: String,
    /// Pula a guarda append-only do pipeline i18n.
    pub force: bool,
    /// Categoria i18n ativa (guarda append-only quando `true` sem `--force`).
    pub pipeline_active: bool,
}

/// Caminhos e pin do dump.
#[derive(Debug, Clone)]
pub struct DumpPaths {
    /// Raiz do repo.
    pub root: PathBuf,
    /// Workspace do upstream.
    pub target_dir: PathBuf,
    /// Diretorio `patches/`.
    pub patches_dir: PathBuf,
    /// Diretorio `overlays/`.
    pub overlays_dir: PathBuf,
    /// Arquivo `series`.
    pub series_file: PathBuf,
    /// Pin do upstream.
    pub base_sha: Sha40,
}

/// Resultado do dump.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DumpOutcome {
    /// Nome do arquivo de patch.
    pub patch_name: String,
    /// Entrada registrada no series.
    pub series_entry: String,
    /// Avisos nao fatais (ignorados, arquivos novos).
    pub warnings: Vec<String>,
}

/// Erros do dump.
#[derive(Debug, Error)]
pub enum DumpError {
    /// Workspace ausente ou sem git.
    #[error("upstream workspace not found: {0}")]
    NoWorkspace(String),
    /// Workspace em estado desconhecido (nem base, nem serie).
    #[error("the workspace is in an unknown state (HEAD: {0}); run dev-setup before dumping")]
    UnknownState(String),
    /// Nada pendente para dumpar.
    #[error("no pending changes to dump")]
    NoChanges,
    /// Diff vazio apos o stage.
    #[error("nothing to dump (empty diff)")]
    EmptyDiff,
    /// Slug fora do kebab-case.
    #[error("slug must be kebab-case (a-z, 0-9 and dashes): {0}")]
    InvalidSlug(String),
    /// Dominio fora de `NN` (`00` reservado).
    #[error("domain must be two digits (01 = i18n, 02 = gateways, 03 = billing): {0}")]
    InvalidDomain(String),
    /// Refresh de entrada ausente no series.
    #[error("patch not found in the series ({0})")]
    NotLast(String),
    /// Refresh recusado pelo pipeline append-only.
    #[error("refusing to refresh {0}: the i18n pipeline is append-only; create a new correction patch instead (or pass --force to override)")]
    AppendOnly(String),
    /// Patch sem commit correspondente no workspace.
    #[error("no workspace commit found for {0}; run apply first")]
    NoCommit(String),
    /// Refresh fora do topo do historico.
    #[error("refusing to refresh {0}: it is not the last workspace commit")]
    NotAtTop(String),
    /// Patch ja registrado no series.
    #[error("patch already registered in the series")]
    AlreadyRegistered,
    /// Falha ao commitar o baseline.
    #[error("failed to commit the change in the workspace; the next dump would capture the entire series")]
    CommitFailed,
    /// Erro do git.
    #[error(transparent)]
    Git(#[from] GitError),
    /// Erro de arquivos.
    #[error(transparent)]
    Repo(#[from] jx_core::BrError),
}

/// Slug kebab-case (`^[a-z0-9]+(-[a-z0-9]+)*$`).
pub fn is_valid_slug(slug: &str) -> bool {
    if slug.is_empty() {
        return false;
    }
    slug.split('-').all(|part| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    })
}

/// Dominio valido: dois digitos, `00` reservado.
pub fn is_valid_domain(domain: &str) -> bool {
    domain.len() == 2 && domain.chars().all(|c| c.is_ascii_digit()) && domain != "00"
}

/// Quebra linhas em 79 colunas sem cortar palavras (como `fold -s -w 79`).
pub fn fold_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return text.lines().map(|line| line.trim().to_string()).collect();
    }
    let mut out = Vec::new();
    for line in text.lines() {
        let mut rest = line.trim();
        if rest.is_empty() {
            out.push(String::new());
            continue;
        }
        while rest.chars().count() > width {
            let max_byte = rest
                .char_indices()
                .nth(width)
                .map_or(rest.len(), |(idx, _)| idx);
            let search_byte = rest
                .char_indices()
                .nth(width + 1)
                .map_or(rest.len(), |(idx, _)| idx);
            let cut = rest[..search_byte].rfind(' ').unwrap_or(max_byte);
            let cut = if cut == 0 { max_byte } else { cut };
            out.push(rest[..cut].to_string());
            rest = rest[cut..].trim_start();
            if rest.is_empty() {
                break;
            }
        }
        if !rest.is_empty() {
            out.push(rest.to_string());
        }
    }
    out
}

/// Dados do cabecalho DEP-3 de um patch novo.
#[derive(Debug, Clone)]
pub struct NewHeader {
    /// Remetente (`From:` sem o e-mail).
    pub author_name: String,
    /// E-mail do remetente.
    pub author_email: String,
    /// Assunto de uma linha.
    pub subject: String,
    /// Descricao longa.
    pub description: String,
    /// Origem DEP-3.
    pub origin: Origin,
    /// Issue de acompanhamento (`Bug-Next-Panel`).
    pub bug: Option<String>,
    /// Estado no upstream.
    pub applied_upstream: AppliedUpstream,
    /// Data `YYYY-MM-DD`.
    pub last_update: String,
    /// Linha `Signed-off-by`.
    pub signoff: String,
    /// Data RFC 2822.
    pub date_rfc2822: String,
}

/// Monta o cabecalho DEP-3 do patch novo.
pub fn render_new_header(header: &NewHeader) -> String {
    let mut text = format!(
        "From: {} <{}>\nDate: {}\nSubject: {}\n\nDescription:\n",
        header.author_name, header.author_email, header.date_rfc2822, header.subject
    );
    for line in fold_text(&header.description, 79) {
        text.push_str(&line);
        text.push('\n');
    }
    text.push('\n');
    text.push_str(&format!(
        "Origin: {}\nForwarded: {}\n",
        header.origin,
        Forwarded::NotNeeded
    ));
    if let Some(url) = &header.bug {
        text.push_str(&format!("Bug-Next-Panel: {url}\n"));
    }
    text.push_str(&format!(
        "Applied-Upstream: {}\nLast-Update: {}\nSigned-off-by: {}\n",
        header.applied_upstream, header.last_update, header.signoff
    ));
    text
}

/// Guarda de estado: HEAD na base ou assunto `br: `.
fn head_state<G: GitBackend>(
    git: &G,
    target: &std::path::Path,
    base: &Sha40,
) -> Result<(), DumpError> {
    let head = git
        .head_rev(target)
        .map_err(|_| DumpError::NoWorkspace(target.display().to_string()))?;
    if head == base.to_string() {
        return Ok(());
    }
    let subject = git.head_subject(target).unwrap_or_default();
    if subject.starts_with("br: ") {
        Ok(())
    } else {
        Err(DumpError::UnknownState(subject))
    }
}

/// Proximo numero livre varrendo `*.patch` no diretorio.
fn next_patch_number_in<FS: FileSystem>(
    fs: &FS,
    patches_dir: &std::path::Path,
) -> Result<PatchNumber, DumpError> {
    let mut numbers: Vec<PatchNumber> = Vec::new();
    for rel in fs
        .list_files_recursive(patches_dir)
        .map_err(DumpError::Repo)?
    {
        let name = rel.to_string_lossy();
        if !name.ends_with(".patch") {
            continue;
        }
        if let Ok(number) = PatchNumber::from_file_name(&name) {
            numbers.push(number);
        }
    }
    next_after(&numbers).map_err(|_| DumpError::AlreadyRegistered)
}

/// Gera um patch novo a partir do delta pendente.
pub fn dump_new<G: GitBackend, FS: FileSystem, C: Clock>(
    git: &G,
    fs: &mut FS,
    clock: &C,
    paths: &DumpPaths,
    req: &NewPatchRequest,
) -> Result<DumpOutcome, DumpError> {
    head_state(git, &paths.target_dir, &paths.base_sha)?;
    if !is_valid_slug(&req.slug) {
        return Err(DumpError::InvalidSlug(req.slug.clone()));
    }
    if !is_valid_domain(&req.domain) {
        return Err(DumpError::InvalidDomain(req.domain.clone()));
    }
    if git.status(&paths.target_dir)?.is_empty() {
        return Err(DumpError::NoChanges);
    }

    let mut warnings = Vec::new();
    let ignored = git.ls_ignored_others(&paths.target_dir)?;
    if !ignored.is_empty() {
        warnings.push(format!(
            "{} ignored file(s) were not captured by the patch",
            ignored.len()
        ));
    }

    let subject = req.message.clone().unwrap_or_else(|| req.slug.clone());
    let description = req
        .description
        .clone()
        .unwrap_or_else(|| DEFAULT_DESCRIPTION.to_string());
    let label = domain_label(&req.domain);
    let slug = if req.slug.starts_with(&format!("{label}-")) {
        req.slug.clone()
    } else {
        format!("{label}-{}", req.slug)
    };
    let number = next_patch_number_in(fs, &paths.patches_dir)?;
    let patch_name = format!("{}-{slug}.patch", number.padded());
    let patch_path = paths.patches_dir.join(&patch_name);

    git.add_all(&paths.target_dir)?;
    exclude_overlay_paths(git, fs, paths, &mut warnings)?;

    let diff = git.diff_cached(&paths.target_dir)?;
    if diff.trim().is_empty() {
        return Err(DumpError::EmptyDiff);
    }
    if diff.contains("new file mode ") {
        warnings
            .push("the patch creates new files; brand new files belong in overlays/".to_string());
    }

    let author_name = git
        .git_config(&paths.target_dir, "user.name")?
        .unwrap_or_else(|| FALLBACK_AUTHOR_NAME.to_string());
    let author_email = git
        .git_config(&paths.target_dir, "user.email")?
        .unwrap_or_else(|| FALLBACK_AUTHOR_EMAIL.to_string());
    let signoff = req
        .signoff
        .clone()
        .unwrap_or_else(|| format!("{author_name} <{author_email}>"));
    let header = render_new_header(&NewHeader {
        author_name,
        author_email,
        subject,
        description,
        origin: req.origin,
        bug: req.bug.clone(),
        applied_upstream: req.applied_upstream.clone(),
        last_update: format_ymd(clock.now_millis()),
        signoff,
        date_rfc2822: format_rfc2822(clock.now_millis()),
    });
    fs.write_string(&patch_path, &format!("{header}\n---\n\n{diff}"))?;

    git.commit(&paths.target_dir, &format!("br: patch {patch_name}"))
        .map_err(|_| DumpError::CommitFailed)?;

    let series_entry = format!("{}/{patch_name}", patches_prefix(&paths.patches_dir));
    if req.add_to_series {
        let series_text = fs
            .read_to_string(&paths.series_file)
            .map_err(DumpError::Repo)?;
        if parse_series(&series_text)
            .iter()
            .any(|entry| entry.as_str() == series_entry)
        {
            warnings.push("patch already registered in the series".to_string());
        } else {
            let entry = SeriesEntry::new(&series_entry)
                .map_err(|_| DumpError::InvalidSlug(series_entry.clone()))?;
            let updated = insert_into_text(&series_text, &entry)
                .map_err(|_| DumpError::InvalidSlug(series_entry.clone()))?;
            fs.write_string(&paths.series_file, &updated)?;
        }
    }
    Ok(DumpOutcome {
        patch_name,
        series_entry,
        warnings,
    })
}

/// Prefixo do diretorio de patches relativo ao padrao `patches/`.
fn patches_prefix(patches_dir: &std::path::Path) -> String {
    patches_dir
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "patches".to_string())
}

/// Tira do stage os caminhos de overlay (nunca pertencem a um patch).
fn exclude_overlay_paths<G: GitBackend, FS: FileSystem>(
    git: &G,
    fs: &FS,
    paths: &DumpPaths,
    warnings: &mut Vec<String>,
) -> Result<(), DumpError> {
    for rel in fs
        .list_files_recursive(&paths.overlays_dir)
        .map_err(DumpError::Repo)?
    {
        let rel_text = rel.to_string_lossy().into_owned();
        if !git.diff_cached_quiet(&paths.target_dir, &rel_text)? {
            git.reset_path(&paths.target_dir, &rel_text)?;
            warnings.push(format!(
                "changes in the overlay {rel_text} are not captured by patches"
            ));
        }
    }
    Ok(())
}

/// Regenera o diff de um patch da serie in-place preservando o cabecalho e reaplicando os seguintes.
pub fn dump_refresh<G: GitBackend, FS: FileSystem>(
    git: &G,
    fs: &mut FS,
    paths: &DumpPaths,
    req: &RefreshRequest,
) -> Result<DumpOutcome, DumpError> {
    if req.pipeline_active && !req.force {
        return Err(DumpError::AppendOnly(req.entry.clone()));
    }
    let series_text = fs
        .read_to_string(&paths.series_file)
        .map_err(DumpError::Repo)?;
    let entries = parse_series(&series_text);
    let Some(target_idx) = entries.iter().position(|entry| entry.as_str() == req.entry) else {
        return Err(DumpError::NotLast(req.entry.clone()));
    };
    let target_entry = &entries[target_idx];
    let patch_path = paths.root.join(target_entry.as_str());
    if !fs.exists(&patch_path) {
        return Err(DumpError::NoChanges);
    }
    let patch_name = target_entry
        .as_str()
        .rsplit('/')
        .next()
        .unwrap_or(target_entry.as_str())
        .to_string();

    let old = fs.read_to_string(&patch_path).map_err(DumpError::Repo)?;
    let header_end = old
        .lines()
        .position(|line| line.trim_end_matches('\r') == "---")
        .ok_or(DumpError::EmptyDiff)?;
    let mut header: String = old
        .lines()
        .take(header_end + 1)
        .map(|line| line.trim_end_matches('\r'))
        .collect::<Vec<_>>()
        .join("\n");
    header.push('\n');

    let commit_opt = git.log_grep_commit(&paths.target_dir, &format!("br: patch {patch_name}"))?;
    let head = git
        .head_rev(&paths.target_dir)
        .map_err(|_| DumpError::NoWorkspace(paths.target_dir.display().to_string()))?;
    let head_subject = git.head_subject(&paths.target_dir).unwrap_or_default();
    let is_last_patch = target_idx + 1 == entries.len();
    let mut overlay_commit_on_top = false;
    let mut warnings = Vec::new();

    match commit_opt {
        Some(commit) => {
            head_state(git, &paths.target_dir, &paths.base_sha)?;
            let parent = git.head_parent_rev(&paths.target_dir)?.unwrap_or_default();
            if head == commit
                || (is_last_patch && parent == commit && head_subject == "br: overlays")
            {
                if head != commit {
                    overlay_commit_on_top = true;
                }
                git.reset_soft(&paths.target_dir, &format!("{commit}^"))?;
                git.add_all(&paths.target_dir)?;
                exclude_overlay_paths(git, fs, paths, &mut warnings)?;
            } else if !is_last_patch && head_subject.starts_with("br: ") {
                if head_subject == "br: overlays" {
                    overlay_commit_on_top = true;
                }
                git.add_all(&paths.target_dir)?;
                exclude_overlay_paths(git, fs, paths, &mut warnings)?;
                let pending_diff = git.diff_cached(&paths.target_dir)?;
                git.reset_hard(&paths.target_dir, &commit)?;
                git.clean(&paths.target_dir)?;
                if !pending_diff.trim().is_empty() {
                    let tmp_patch = paths.target_dir.join(".git/br-refresh-pending.patch");
                    fs.write_string(&tmp_patch, &pending_diff)?;
                    let apply_res = git
                        .apply(&paths.target_dir, &tmp_patch)
                        .or_else(|_| git.apply_3way(&paths.target_dir, &tmp_patch));
                    let _ = fs.remove_file(&tmp_patch);
                    apply_res?;
                }
                git.reset_soft(&paths.target_dir, &format!("{commit}^"))?;
                git.add_all(&paths.target_dir)?;
                exclude_overlay_paths(git, fs, paths, &mut warnings)?;
            } else {
                return Err(DumpError::NotAtTop(req.entry.clone()));
            }
        }
        None => {
            if target_idx > 0 {
                head_state(git, &paths.target_dir, &paths.base_sha)?;
                let prev_name = entries[target_idx - 1]
                    .as_str()
                    .rsplit('/')
                    .next()
                    .unwrap_or(entries[target_idx - 1].as_str());
                if head_subject != format!("br: patch {prev_name}") {
                    return Err(DumpError::NoCommit(req.entry.clone()));
                }
            }
            git.add_all(&paths.target_dir)?;
            exclude_overlay_paths(git, fs, paths, &mut warnings)?;
        }
    }

    let diff = git.diff_cached(&paths.target_dir)?;
    if diff.trim().is_empty() {
        return Err(DumpError::EmptyDiff);
    }
    fs.write_string(&patch_path, &format!("{header}\n{diff}"))?;
    git.commit(&paths.target_dir, &format!("br: patch {patch_name}"))
        .map_err(|_| DumpError::CommitFailed)?;

    if !is_last_patch {
        for rem in &entries[target_idx + 1..] {
            let rem_path = paths.root.join(rem.as_str());
            if git.apply(&paths.target_dir, &rem_path).is_err() {
                git.apply_3way(&paths.target_dir, &rem_path)?;
            }
            if !git.status(&paths.target_dir)?.is_empty() {
                git.add_all(&paths.target_dir)?;
                let rem_name = rem.as_str().rsplit('/').next().unwrap_or(rem.as_str());
                git.commit(&paths.target_dir, &format!("br: patch {rem_name}"))
                    .map_err(|_| DumpError::CommitFailed)?;
            }
        }
        let _ = crate::apply::copy_overlays(fs, &paths.target_dir, &paths.overlays_dir);
        if (overlay_commit_on_top
            || !fs
                .list_files_recursive(&paths.overlays_dir)
                .unwrap_or_default()
                .is_empty())
            && !git.status(&paths.target_dir)?.is_empty()
        {
            git.add_all(&paths.target_dir)?;
            git.commit(&paths.target_dir, "br: overlays")
                .map_err(|_| DumpError::CommitFailed)?;
        }
    } else if overlay_commit_on_top && !git.status(&paths.target_dir)?.is_empty() {
        git.add_all(&paths.target_dir)?;
        git.commit(&paths.target_dir, "br: overlays")
            .map_err(|_| DumpError::CommitFailed)?;
    }

    Ok(DumpOutcome {
        patch_name,
        series_entry: target_entry.as_str().to_string(),
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::ScriptGit;
    use jx_core::MemFs;

    const BASE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn dump_paths() -> DumpPaths {
        DumpPaths {
            root: PathBuf::from("/repo"),
            target_dir: PathBuf::from("/ws"),
            patches_dir: PathBuf::from("/repo/patches"),
            overlays_dir: PathBuf::from("/repo/overlays"),
            series_file: PathBuf::from("/repo/patches/series"),
            base_sha: Sha40::parse(BASE).unwrap(),
        }
    }

    /// Relogio fixo para cabecalhos deterministas.
    struct FixedClock(u64);

    impl Clock for FixedClock {
        fn now_millis(&self) -> u64 {
            self.0
        }
    }

    #[test]
    fn validates_slug_and_domain() {
        assert!(is_valid_slug("add-pt-br-locale"));
        assert!(!is_valid_slug("Add_PT_BR"));
        assert!(!is_valid_slug(""));
        assert!(is_valid_domain("01"));
        assert!(!is_valid_domain("00"));
        assert!(!is_valid_domain("1"));
        assert!(!is_valid_domain("0a"));
        assert_eq!(domain_label("01"), "i18n");
        assert_eq!(domain_label("02"), "gateway");
        assert_eq!(domain_label("09"), "core");
        assert_eq!(domain_label("10"), "ui");
    }

    #[test]
    fn folds_long_lines_without_cutting_words() {
        let folded = fold_text(
            "uma linha bem longa que precisa quebrar em algum espaco antes do limite",
            20,
        );
        assert!(folded.iter().all(|line| line.chars().count() <= 20));
        assert_eq!(
            folded.join(" ").replace("  ", " "),
            "uma linha bem longa que precisa quebrar em algum espaco antes do limite"
        );
        let utf8_folded = fold_text(
            "Tradução de configurações, exceções e notificações com acentuação gráfica completa",
            25,
        );
        assert!(utf8_folded.iter().all(|line| line.chars().count() <= 25));
        assert_eq!(fold_text("linha", 0), vec!["linha".to_string()]);
    }

    #[test]
    fn rejects_unknown_state_and_empty_tree() {
        let mut git = ScriptGit::default();
        git.head = "ffff".to_string();
        git.subject = Some("trabalho externo".to_string());
        let req = NewPatchRequest {
            slug: "x".to_string(),
            ..NewPatchRequest::default()
        };
        // HEAD desconhecido e sem assunto br: -> UnknownState.
        let err = dump_new(
            &git,
            &mut MemFs::default(),
            &FixedClock(0),
            &dump_paths(),
            &req,
        )
        .unwrap_err();
        assert!(matches!(err, DumpError::UnknownState(_)));

        let mut git = ScriptGit::default();
        git.head = BASE.to_string();
        let err = dump_new(
            &git,
            &mut MemFs::default(),
            &FixedClock(0),
            &dump_paths(),
            &req,
        )
        .unwrap_err();
        assert!(matches!(err, DumpError::NoChanges));
    }

    #[test]
    fn refresh_middle_patch_in_place_replays_subsequent_patches() {
        let mut fs = MemFs::default();
        fs.write_string(
            std::path::Path::new("/repo/patches/series"),
            "patches/0001-a.patch\npatches/0002-b.patch\n",
        )
        .unwrap();
        fs.write_string(
            std::path::Path::new("/repo/patches/0001-a.patch"),
            "From: A <a@b>\nSubject: a\n\n---\n\ndiff --git a/a b/a\n",
        )
        .unwrap();
        fs.write_string(
            std::path::Path::new("/repo/patches/0002-b.patch"),
            "From: A <a@b>\nSubject: b\n\n---\n\ndiff --git a/b b/b\n",
        )
        .unwrap();

        let mut git = ScriptGit::default();
        git.head = "2222222222222222222222222222222222222222".to_string();
        git.subject = Some("br: patch 0002-b.patch".to_string());
        git.status_dirty.set(true);
        git.grep_commits.insert(
            "br: patch 0001-a.patch".to_string(),
            "1111111111111111111111111111111111111111".to_string(),
        );

        let req = RefreshRequest {
            entry: "patches/0001-a.patch".to_string(),
            force: false,
            pipeline_active: false,
        };
        let outcome = dump_refresh(&git, &mut fs, &dump_paths(), &req).unwrap();
        assert_eq!(outcome.patch_name, "0001-a.patch");
        assert_eq!(outcome.series_entry, "patches/0001-a.patch");
        let commits = git.commits.borrow().clone();
        assert_eq!(
            commits,
            vec![
                "br: patch 0001-a.patch".to_string(),
                "br: patch 0002-b.patch".to_string()
            ]
        );
    }
}
