//! Dependency direction: the app depends on the engine, never the other way round.
//! The engine (`analytics`, repo root) stays free of HTTP frameworks, HTML and app crates.
//! `docs/roadmap.md` P0.1; `.cursor/rules/analytics-core-scope.mdc`.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root")
}

/// Keys of `[dependencies]`, `[dev-dependencies]` and `[build-dependencies]`
/// in a Cargo manifest. Line-based on purpose: no TOML parser needed for this check.
fn dependency_names(manifest: &str) -> Vec<String> {
    let mut in_deps = false;
    let mut names = Vec::new();
    for raw in manifest.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_deps = matches!(
                line,
                "[dependencies]" | "[dev-dependencies]" | "[build-dependencies]"
            );
            continue;
        }
        if !in_deps || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = line.split_once('=') {
            names.push(key.trim().trim_matches('"').to_string());
        }
    }
    names
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

const FORBIDDEN_CORE_DEPS: &[&str] = &[
    "axum",
    "axum-core",
    "hyper",
    "tower",
    "tower-http",
    "http",
    "askama",
    "maud",
    "tera",
    "octa-app",
    "analytics-web",
];

const FORBIDDEN_CORE_TOKENS: &[&str] = &["axum", "octa_app", "analytics_web", "<html", "text/html"];

#[test]
fn core_manifest_lists_no_http_html_or_app_crates() {
    let manifest =
        std::fs::read_to_string(repo_root().join("Cargo.toml")).expect("core Cargo.toml");
    let deps = dependency_names(&manifest);
    assert!(
        deps.contains(&"serde".to_string()),
        "sanity: parsed {deps:?}"
    );
    for forbidden in FORBIDDEN_CORE_DEPS {
        assert!(
            !deps.iter().any(|d| d == forbidden),
            "analytics must not depend on `{forbidden}`; found {deps:?}"
        );
    }
}

#[test]
fn core_sources_never_reference_http_html_or_app_crates() {
    let mut files = Vec::new();
    rust_files(&repo_root().join("src"), &mut files);
    assert!(!files.is_empty(), "no core sources found");
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read source");
        for token in FORBIDDEN_CORE_TOKENS {
            assert!(
                !text.contains(token),
                "{} mentions `{token}`; the engine stays free of HTTP, HTML and app crates",
                file.display()
            );
        }
    }
}

#[test]
fn app_manifest_does_not_pull_in_the_poc_ui() {
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("app Cargo.toml");
    let deps = dependency_names(&manifest);
    assert!(
        !deps.iter().any(|d| d == "analytics-web"),
        "the product app must not build on the throwaway PoC UI; found {deps:?}"
    );
}
