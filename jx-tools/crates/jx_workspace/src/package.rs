//! Empacotamento reproduzivel do painel com a serie aplicada.
//!
//! Espelha `scripts/package.sh`: valida a serie via lint, exporta o pin com
//! `git archive`, aplica a serie na arvore exportada, embute `overlay/` e
//! gera o tarball com mtime/owner fixos.

use std::path::{Path, PathBuf};

use thiserror::Error;

use jx_core::{BrError, FileSystem};
use jx_git::{GitBackend, GitError, Sha40};
use jx_patch::parse_series;

/// Pedido de empacotamento (flags do CLI).
#[derive(Debug, Clone, Default)]
pub struct PackageRequest {
    /// Versao do pacote (padrao: upstream.ref sem o `v` inicial).
    pub version: Option<String>,
    /// Caminho do tarball (padrao: `<dist>/jexactyl-<versao>.tar.gz`).
    pub output: Option<PathBuf>,
    /// Epoch do tarball reproduzivel (`SOURCE_DATE_EPOCH`).
    pub source_date_epoch: u64,
}

/// Caminhos e pin do empacotamento.
#[derive(Debug, Clone)]
pub struct PackagePaths {
    /// Raiz do repo.
    pub root: PathBuf,
    /// Workspace do upstream.
    pub target_dir: PathBuf,
    /// Diretorio `patches/`.
    pub patches_dir: PathBuf,
    /// Arquivo `series`.
    pub series_file: PathBuf,
    /// Diretorio `overlays/`.
    pub overlays_dir: PathBuf,
    /// Diretorio de saida.
    pub dist_dir: PathBuf,
    /// URL do upstream (vai ao release.json).
    pub upstream_url: String,
    /// Ref do upstream (vai ao release.json e ao nome).
    pub upstream_ref: String,
    /// Pin do upstream.
    pub base_sha: Sha40,
    /// Arquivo config.toml (embutido em `overlay/`).
    pub config_file: PathBuf,
}

/// Relatorio do empacotamento.
#[derive(Debug, Clone)]
pub struct PackageReport {
    /// Nome do pacote (`jexactyl-<versao>`).
    pub package_name: String,
    /// Caminho do tarball gerado.
    pub archive: PathBuf,
    /// Patches aplicados.
    pub patch_count: usize,
}

/// Erros do empacotamento.
#[derive(Debug, Error)]
pub enum PackageError {
    /// Workspace ausente ou sem git.
    #[error("upstream workspace not found: {0} (run dev-setup first)")]
    NoWorkspace(String),
    /// Arquivo `series` ausente.
    #[error("series file not found: {0}")]
    NoSeries(String),
    /// Patch do series ausente no disco.
    #[error("patch listed in the series does not exist: {0}")]
    PatchMissing(String),
    /// Patch nao aplica na arvore exportada.
    #[error("patch does not apply on the exported tree: {0}")]
    CheckFailed(String),
    /// Falha ao aplicar na arvore exportada.
    #[error("patch failed while being applied: {0}")]
    ApplyFailed(String),
    /// Falha de I/O do tarball.
    #[error("tarball I/O failure at {0}: {1}")]
    TarIo(String, #[source] std::io::Error),
    /// Erro do git.
    #[error(transparent)]
    Git(#[from] GitError),
    /// Erro de arquivos.
    #[error(transparent)]
    Repo(#[from] BrError),
}

/// Nome do pacote a partir da ref ou da versao pedida.
///
/// Remove um unico `v` inicial, como `${VERSION#v}` do shell.
pub fn package_name_for(base_ref: &str, version: Option<&str>) -> String {
    let raw = version.unwrap_or(base_ref);
    format!("jexactyl-{}", raw.strip_prefix('v').unwrap_or(raw))
}

/// Escapa texto para string JSON (aspas, barras, controles).
pub fn json_escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 2);
    for c in raw.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Renderiza o `overlay/release.json` do pacote.
pub fn render_release_json(
    package: &str,
    url: &str,
    upstream_ref: &str,
    sha: &Sha40,
    entries: &[String],
) -> String {
    let mut text = format!(
        "{{\n    \"package\": \"{}\",\n    \"upstream\": {{\n        \"url\": \"{}\",\n        \"ref\": \"{}\",\n        \"sha\": \"{}\"\n    }},\n    \"patches\": [",
        json_escape(package),
        json_escape(url),
        json_escape(upstream_ref),
        sha
    );
    if entries.is_empty() {
        text.push(']');
    } else {
        for (index, entry) in entries.iter().enumerate() {
            text.push('\n');
            text.push_str(&format!("        \"{}\"", json_escape(entry)));
            if index + 1 < entries.len() {
                text.push(',');
            }
        }
        text.push('\n');
        text.push_str("    ]");
    }
    text.push_str("\n}\n");
    text
}

/// Copia o conteudo de um diretorio (arquivos, recursivo, ordenado).
fn copy_dir_contents<FS: FileSystem>(
    fs: &mut FS,
    src: &Path,
    dst: &Path,
) -> Result<usize, BrError> {
    let mut count = 0;
    for rel in fs.list_files_recursive(src)? {
        fs.copy_file(&src.join(&rel), &dst.join(&rel))?;
        count += 1;
    }
    Ok(count)
}

/// Estagia o pacote sob `stage_dir` (injetavel em testes via `MemFs`).
pub fn run_package_stage<G: GitBackend, FS: FileSystem>(
    git: &G,
    fs: &mut FS,
    paths: &PackagePaths,
    req: &PackageRequest,
    stage_dir: &Path,
) -> Result<PackageReport, PackageError> {
    if !git.is_git_repo(&paths.target_dir) {
        return Err(PackageError::NoWorkspace(
            paths.target_dir.display().to_string(),
        ));
    }
    let series_text = fs
        .read_to_string(&paths.series_file)
        .map_err(|_| PackageError::NoSeries(paths.series_file.display().to_string()))?;
    let entries = parse_series(&series_text);
    for entry in &entries {
        if !fs.exists(&paths.root.join(entry.as_str())) {
            return Err(PackageError::PatchMissing(entry.as_str().to_string()));
        }
    }

    let package_name = package_name_for(&paths.upstream_ref, req.version.as_deref());
    let package_root = stage_dir.join(&package_name);
    git.archive_extract(
        &paths.target_dir,
        &paths.base_sha.to_string(),
        &package_root,
    )?;

    for entry in &entries {
        let patch = paths.root.join(entry.as_str());
        if git.apply_check(&package_root, &patch, false).is_err() {
            return Err(PackageError::CheckFailed(entry.as_str().to_string()));
        }
        git.apply(&package_root, &patch)
            .map_err(|_| PackageError::ApplyFailed(entry.as_str().to_string()))?;
    }

    if fs.exists(&paths.overlays_dir) {
        copy_dir_contents(fs, &paths.overlays_dir, &package_root).map_err(PackageError::Repo)?;
    }

    let overlay_base = package_root.join("overlay");
    copy_dir_contents(fs, &paths.patches_dir, &overlay_base.join("patches"))
        .map_err(PackageError::Repo)?;
    let scripts_dir = paths.root.join("scripts");
    if fs.exists(&scripts_dir) {
        copy_dir_contents(fs, &scripts_dir, &overlay_base.join("scripts"))
            .map_err(PackageError::Repo)?;
    }
    fs.copy_file(&paths.config_file, &overlay_base.join("config.toml"))
        .map_err(PackageError::Repo)?;
    for extra in ["README.md", "LICENSE"] {
        let src = paths.root.join(extra);
        if fs.exists(&src) {
            fs.copy_file(&src, &overlay_base.join(extra))
                .map_err(PackageError::Repo)?;
        }
    }

    let entry_texts: Vec<String> = entries
        .iter()
        .map(|entry| entry.as_str().to_string())
        .collect();
    let release = render_release_json(
        &package_name,
        &paths.upstream_url,
        &paths.upstream_ref,
        &paths.base_sha,
        &entry_texts,
    );
    fs.write_string(&overlay_base.join("release.json"), &release)
        .map_err(PackageError::Repo)?;

    let archive = match &req.output {
        Some(out) => out.clone(),
        None => paths.dist_dir.join(format!("{package_name}.tar.gz")),
    };
    Ok(PackageReport {
        package_name,
        archive,
        patch_count: entries.len(),
    })
}

/// Monta o tarball reproduzivel: ordem de nome, mtime/uid/gid fixos, GNU.
///
/// Opera no disco real (mtime e dono nao cabem no `FileSystem` abstrato).
pub fn write_tarball(
    stage_dir: &Path,
    package_name: &str,
    archive: &Path,
    mtime: u64,
) -> Result<(), PackageError> {
    use std::fs::File;
    use std::io::Read;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let package_root = stage_dir.join(package_name);
    let mut rels: Vec<PathBuf> = Vec::new();
    collect_tree(&package_root, &package_root, &mut rels)
        .map_err(|e| PackageError::TarIo(package_root.display().to_string(), e))?;
    rels.sort();

    if let Some(parent) = archive.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| PackageError::TarIo(parent.display().to_string(), e))?;
        }
    }
    let file =
        File::create(archive).map_err(|e| PackageError::TarIo(archive.display().to_string(), e))?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut builder = tar::Builder::new(encoder);
    let prefix = Path::new(package_name);
    for rel in rels {
        let entry_path = prefix.join(&rel);
        let full = package_root.join(&rel);
        let meta = std::fs::symlink_metadata(&full)
            .map_err(|e| PackageError::TarIo(full.display().to_string(), e))?;
        let mut header = tar::Header::new_gnu();
        #[cfg(unix)]
        header.set_mode(meta.permissions().mode());
        #[cfg(not(unix))]
        header.set_mode(if meta.is_dir() { 0o755 } else { 0o644 });
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(mtime);
        if meta.is_dir() {
            header.set_entry_type(tar::EntryType::Directory);
            header.set_size(0);
            builder
                .append_data(&mut header, &entry_path, &[][..])
                .map_err(|e| {
                    PackageError::TarIo(rel.display().to_string(), std::io::Error::other(e))
                })?;
        } else if meta.file_type().is_symlink() {
            let target = std::fs::read_link(&full)
                .map_err(|e| PackageError::TarIo(full.display().to_string(), e))?;
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_size(0);
            header.set_link_name(&target).map_err(|e| {
                PackageError::TarIo(rel.display().to_string(), std::io::Error::other(e))
            })?;
            builder
                .append_data(&mut header, &entry_path, &[][..])
                .map_err(|e| {
                    PackageError::TarIo(rel.display().to_string(), std::io::Error::other(e))
                })?;
        } else {
            header.set_entry_type(tar::EntryType::Regular);
            header.set_size(meta.len());
            let mut handle = File::open(&full)
                .map_err(|e| PackageError::TarIo(full.display().to_string(), e))?;
            let mut chunk = Vec::new();
            handle
                .read_to_end(&mut chunk)
                .map_err(|e| PackageError::TarIo(full.display().to_string(), e))?;
            builder
                .append_data(&mut header, &entry_path, chunk.as_slice())
                .map_err(|e| {
                    PackageError::TarIo(rel.display().to_string(), std::io::Error::other(e))
                })?;
        }
    }
    let encoder = builder.into_inner().map_err(|e| {
        PackageError::TarIo(archive.display().to_string(), std::io::Error::other(e))
    })?;
    encoder
        .finish()
        .map_err(|e| PackageError::TarIo(archive.display().to_string(), e))?;
    Ok(())
}

/// Coleta dirs e arquivos sob `base`, com caminhos relativos a `base`.
fn collect_tree(base: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path
            .strip_prefix(base)
            .map(Path::to_path_buf)
            .map_err(|_| std::io::Error::other("prefixo fora da base"))?;
        out.push(rel);
        if entry.file_type()?.is_dir() {
            collect_tree(base, &path, out)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jx_core::MemFs;
    use jx_ops::support::ScriptGit;

    const BASE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn package_paths() -> PackagePaths {
        PackagePaths {
            root: PathBuf::from("/repo"),
            target_dir: PathBuf::from("/ws"),
            patches_dir: PathBuf::from("/repo/patches"),
            series_file: PathBuf::from("/repo/patches/series"),
            overlays_dir: PathBuf::from("/repo/overlays"),
            dist_dir: PathBuf::from("/repo/dist"),
            upstream_url: "https://exemplo.test/painel.git".to_string(),
            upstream_ref: "v4.0.7".to_string(),
            base_sha: Sha40::parse(BASE).unwrap(),
            config_file: PathBuf::from("/repo/config.toml"),
        }
    }

    fn seed(fs: &mut MemFs) {
        for (path, content) in [
            ("/repo/patches/series", "patches/0001-a.patch\n"),
            ("/repo/patches/0001-a.patch", "diff\n"),
            ("/repo/overlays/app/x.txt", "novo\n"),
            ("/repo/scripts/dump.sh", "#!/bin/sh\n"),
            ("/repo/config.toml", "[upstream]\nsha = \"x\"\n"),
            ("/repo/README.md", "oi\n"),
        ] {
            fs.write_string(Path::new(path), content).unwrap();
        }
        fs.write_string(Path::new("/ws/.git/HEAD"), "ref\n")
            .unwrap();
    }

    #[test]
    fn strips_single_leading_v_and_renders_release_json() {
        assert_eq!(package_name_for("v4.0.7", None), "jexactyl-4.0.7");
        assert_eq!(package_name_for("v4.0.7", Some("vv1")), "jexactyl-v1");
        assert_eq!(package_name_for("main", Some("1.0")), "jexactyl-1.0");
        let json = render_release_json(
            "jexactyl-4.0.7",
            "https://x/y",
            "v4.0.7",
            &Sha40::parse(BASE).unwrap(),
            &["patches/0001-a.patch".to_string()],
        );
        assert!(json.contains("\"package\": \"jexactyl-4.0.7\""));
        assert!(json.contains("\"sha\": \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\""));
        assert!(json.contains("\"patches/0001-a.patch\""));
        let empty = render_release_json(
            "jexactyl-4.0.7",
            "u",
            "r",
            &Sha40::parse(BASE).unwrap(),
            &[],
        );
        assert!(empty.contains("\"patches\": []"));
        assert_eq!(json_escape("a\"b\\c\nd"), "a\\\"b\\\\c\\nd");
    }

    #[test]
    fn stages_package_with_overlays_and_manifests() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        let git = ScriptGit::default();
        let report = run_package_stage(
            &git,
            &mut fs,
            &package_paths(),
            &PackageRequest::default(),
            Path::new("/stage"),
        )
        .unwrap();
        assert_eq!(report.package_name, "jexactyl-4.0.7");
        assert_eq!(
            report.archive,
            PathBuf::from("/repo/dist/jexactyl-4.0.7.tar.gz")
        );
        assert_eq!(report.patch_count, 1);
        assert!(git
            .calls
            .borrow()
            .iter()
            .any(|call| call.starts_with("archive ")));
        // Serie aplicada via git: check e apply registrados contra o repo.
        assert!(git
            .calls
            .borrow()
            .iter()
            .any(|call| call == "check /repo/patches/0001-a.patch"));
        assert!(git
            .calls
            .borrow()
            .iter()
            .any(|call| call == "apply /repo/patches/0001-a.patch"));
        assert_eq!(
            fs.get(Path::new("/stage/jexactyl-4.0.7/app/x.txt")),
            Some("novo\n")
        );
        assert_eq!(
            fs.get(Path::new(
                "/stage/jexactyl-4.0.7/overlay/patches/0001-a.patch"
            )),
            Some("diff\n")
        );
        assert_eq!(
            fs.get(Path::new("/stage/jexactyl-4.0.7/overlay/scripts/dump.sh")),
            Some("#!/bin/sh\n")
        );
        assert!(fs
            .get(Path::new("/stage/jexactyl-4.0.7/overlay/release.json"))
            .unwrap()
            .contains("\"package\": \"jexactyl-4.0.7\""));
    }

    #[test]
    fn fails_on_missing_patch_and_dirty_check() {
        let mut fs = MemFs::default();
        seed(&mut fs);
        fs.remove_file(Path::new("/repo/patches/0001-a.patch"))
            .unwrap();
        let git = ScriptGit::default();
        let err = run_package_stage(
            &git,
            &mut fs,
            &package_paths(),
            &PackageRequest::default(),
            Path::new("/stage"),
        )
        .unwrap_err();
        assert!(matches!(err, PackageError::PatchMissing(_)));

        let mut fs = MemFs::default();
        seed(&mut fs);
        let mut git = ScriptGit::default();
        // O check roda com o caminho do patch no repo.
        git.check_ok
            .insert("/repo/patches/0001-a.patch".to_string(), false);
        let err = run_package_stage(
            &git,
            &mut fs,
            &package_paths(),
            &PackageRequest::default(),
            Path::new("/stage"),
        )
        .unwrap_err();
        assert!(matches!(err, PackageError::CheckFailed(_)));
    }

    #[test]
    fn tarball_is_deterministic_and_sorted() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = dir.path().join("jexactyl-1.0");
        std::fs::create_dir_all(pkg.join("b")).unwrap();
        std::fs::write(pkg.join("b/z.txt"), "z").unwrap();
        std::fs::write(pkg.join("a.txt"), "a").unwrap();
        let first = dir.path().join("um.tar.gz");
        let second = dir.path().join("dois.tar.gz");
        write_tarball(dir.path(), "jexactyl-1.0", &first, 0).unwrap();
        write_tarball(dir.path(), "jexactyl-1.0", &second, 0).unwrap();
        assert_eq!(
            std::fs::read(&first).unwrap(),
            std::fs::read(&second).unwrap()
        );

        let data = std::fs::read(&first).unwrap();
        let decoder = flate2::read::GzDecoder::new(data.as_slice());
        let mut archive = tar::Archive::new(decoder);
        let names: Vec<String> = archive
            .entries()
            .unwrap()
            .map(|entry| {
                entry
                    .unwrap()
                    .path()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
        assert!(names.iter().any(|name| name == "jexactyl-1.0/a.txt"));
    }
}
