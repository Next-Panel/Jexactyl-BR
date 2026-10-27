//! Caminhos tocados por um diff e lista de caminhos proibidos.
//!
//! Espelha `is_forbidden_path` e `check_forbidden_paths` de
//! `scripts/lint-patches.sh`.

/// Um toque `diff --git a/<old> b/<new>` com os dois lados separados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffTouch {
    /// Caminho do lado `a/`.
    pub old_path: String,
    /// Caminho do lado `b/`.
    pub new_path: String,
}

/// Toque em caminho proibido, com o lado e o caminho.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForbiddenTouch {
    /// Lado do diff (`a` ou `b`).
    pub side: char,
    /// Caminho proibido.
    pub path: String,
}

/// Extrai os pares `a/ b/` das linhas `diff --git` do corpo.
///
/// Regra herdada do shell: o lado `a` vai ate o ultimo ` b/`, e o lado `b`
/// comeca apos o primeiro ` b/` (quirk preservado para paridade).
pub fn touched_paths(body: &str) -> Vec<DiffTouch> {
    body.lines()
        .filter_map(|line| line.strip_prefix("diff --git a/"))
        .filter_map(|rest| {
            let split = rest.find(" b/")?;
            let new_path = rest[split + 3..].to_string();
            let old_path = rest[..rest.rfind(" b/").unwrap_or(split)].to_string();
            Some(DiffTouch { old_path, new_path })
        })
        .collect()
}

/// Informa se o caminho jamais pode aparecer num patch.
///
/// Nomes como `.env` e lockfiles valem sozinhos ou apos qualquer diretorio
/// (`*/.env`), como no `case` do shell; `public/build` e relativo a raiz.
pub fn is_forbidden_path(path: &str) -> bool {
    let file = path.rsplit('/').next().unwrap_or(path);
    if matches!(
        file,
        ".env" | "composer.lock" | "pnpm-lock.yaml" | "package-lock.json" | "yarn.lock"
    ) {
        return true;
    }
    if path == "public/build" || path.starts_with("public/build/") {
        return true;
    }
    let segments: Vec<&str> = path.split('/').collect();
    if segments.contains(&"node_modules") {
        return true;
    }
    if segments.contains(&"vendor") && !path.starts_with("resources/views/vendor/") {
        return true;
    }
    false
}

/// Lista os toques proibidos do corpo, nos dois lados do diff.
pub fn forbidden_touches(body: &str) -> Vec<ForbiddenTouch> {
    let mut hits = Vec::new();
    for touch in touched_paths(body) {
        if is_forbidden_path(&touch.old_path) {
            hits.push(ForbiddenTouch {
                side: 'a',
                path: touch.old_path.clone(),
            });
        }
        if is_forbidden_path(&touch.new_path) {
            hits.push(ForbiddenTouch {
                side: 'b',
                path: touch.new_path.clone(),
            });
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_both_sides() {
        let body = "diff --git a/a/x.ts b/a/x.ts\n+ola\ndiff --git a/velho b/novo\n";
        let touches = touched_paths(body);
        assert_eq!(touches.len(), 2);
        assert_eq!(touches[0].old_path, "a/x.ts");
        assert_eq!(touches[0].new_path, "a/x.ts");
        assert_eq!(touches[1].old_path, "velho");
    }

    #[test]
    fn forbids_lockfiles_env_build_and_vendors() {
        for bad in [
            ".env",
            "config/.env",
            "composer.lock",
            "pnpm-lock.yaml",
            "package-lock.json",
            "yarn.lock",
            "public/build",
            "public/build/x.js",
            "a/node_modules/b.js",
            "vendor/autoload.php",
            "app/vendor/x.php",
        ] {
            assert!(is_forbidden_path(bad), "liberou {bad}");
        }
        for good in [
            "resources/views/vendor/painel.blade.php",
            "app/Services/X.php",
            "resources/scripts/app.tsx",
            "public/x.js",
            "avendor/b.php",
        ] {
            assert!(!is_forbidden_path(good), "barrou {good}");
        }
    }

    #[test]
    fn reports_forbidden_touches() {
        let body = "diff --git a/.env b/.env\n+KEY=x\n";
        let hits = forbidden_touches(body);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].side, 'a');
    }
}
