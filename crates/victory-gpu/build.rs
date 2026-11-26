//! Fingerprints the shader translator, so that cached translations
//! (`src/shader_cache.rs`) last until the translator itself changes and not
//! only until the next build of anything.

use std::path::{Path, PathBuf};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// "https://user:token@github.com/owner/repo.git" or "git@github.com:owner/repo.git" -> "owner/repo".
fn owner_repo(url: &str) -> Option<String> {
    let rest = url.trim().split_once("://").map_or(url.trim(), |(_, r)| r);
    // Drop "user:token@" (or "git@") before the host.
    let rest = match rest.split_once('@') {
        Some((who, after)) if !who.contains('/') => after,
        _ => rest,
    };
    let path = rest.replace(':', "/");
    let mut parts = path.trim_end_matches('/').trim_end_matches(".git").rsplit('/');
    let repo = parts.next().filter(|s| !s.is_empty())?;
    let owner = parts.next().filter(|s| !s.is_empty())?;
    Some(format!("{owner}/{repo}"))
}

fn main() {
    let here = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    let crates = here.join("..");
    let root = crates.join("..");
    // The translator, what it is built from, and the form entries are kept in.
    let mut all = vec![
        crates.join("victory-shader/Cargo.toml"),
        here.join("src/shader_cache.rs"),
        root.join("Cargo.lock"),
        root.join("rust-toolchain.toml"),
    ];
    let source = crates.join("victory-shader/src");
    println!("cargo:rerun-if-changed={}", source.display());
    files(&source, &mut all);
    all.sort();
    let mut hash = blake3::Hasher::new();
    for path in &all {
        println!("cargo:rerun-if-changed={}", path.display());
        // Names and line endings as they are in every checkout.
        let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let bytes: Vec<u8> = std::fs::read(path).unwrap_or_default().into_iter().filter(|&b| b != 13).collect();
        hash.update(name.as_bytes());
        hash.update(&(bytes.len() as u64).to_le_bytes());
        hash.update(&bytes);
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-env=VICTORY_TRANSLATOR={}", hash.finalize().to_hex());
    // KLBF Lab Bench card (PastiiJay/klbf-lab): owner/repo from the origin (credentials never
    // kept), else a name declared with KLBF_PROJECT_NAME. Without git (a `git archive` build),
    // KLBF_GIT_SHA / KLBF_GIT_REMOTE give the commit and the origin.
    let git = |args: &[&str]| {
        std::process::Command::new("git").args(args).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let env = |name: &str| std::env::var(name).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    let sha = git(&["rev-parse", "--short", "HEAD"]).or_else(|| env("KLBF_GIT_SHA"));
    if let Some(log) = git(&["rev-parse", "--git-path", "logs/HEAD"]) {
        println!("cargo:rerun-if-changed={log}");
    }
    let remote = git(&["remote", "get-url", "origin"]).or_else(|| env("KLBF_GIT_REMOTE"));
    let clean = |s: &str| s.chars().filter(|c| c.is_ascii_alphanumeric() || "-_./ ()".contains(*c)).take(80).collect::<String>();
    let (project, source) = match (remote.as_deref().and_then(owner_repo), env("KLBF_PROJECT_NAME")) {
        (Some(p), _) => (clean(&p), "git"),
        (None, Some(d)) => (clean(&d), "declared"),
        _ => ("unknown (local build)".to_string(), "none"),
    };
    println!("cargo:rustc-env=KLBF_PROJECT={project}");
    println!("cargo:rustc-env=KLBF_PROJECT_SOURCE={source}");
    println!("cargo:rustc-env=KLBF_SHA={}", sha.as_deref().map(clean).unwrap_or_else(|| "unknown".into()));
    for name in ["KLBF_GIT_SHA", "KLBF_GIT_REMOTE", "KLBF_PROJECT_NAME"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
}
