//! Embeds the source of every glowkit component crate into the binary (deflate-compressed), so
//! `glowkit add` works offline, including when installed from crates.io.
//!
//! Inside the workspace the sources are read from `../glowkit-*/src`. A published crate has no
//! siblings, so it falls back to the `sources.bundle` shipped with it (written by
//! `BUNDLE=1 scripts/gen-cli-registry.sh`, which sets `GLOWKIT_CLI_WRITE_BUNDLE`).

use std::path::{Path, PathBuf};

/// Crates whose source is embedded (glowkit-core stays a normal dependency).
const CRATES: &[&str] = &[
    "glowkit-text",
    "glowkit-backgrounds",
    "glowkit-effects",
    "glowkit-components",
    "glowkit-charts",
    "glowkit-ai",
    "glowkit-blocks",
    "glowkit-motion",
    "glowkit-3d",
    "glowkit-canvas",
    "glowkit-templates",
    "glowkit-interactions",
    "glowkit-agents",
];

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn put(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(bytes);
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("sources.bundle");
    let crates_dir = manifest.parent().unwrap().to_path_buf();
    let shipped = manifest.join("sources.bundle");
    println!("cargo:rerun-if-env-changed=GLOWKIT_CLI_WRITE_BUNDLE");
    println!("cargo:rerun-if-changed=build.rs");

    if crates_dir.join("glowkit-core").join("Cargo.toml").exists() {
        // In the workspace: bundle `<crate>/src/**` (Rust files plus any assets they include).
        let mut raw = Vec::new();
        for c in CRATES {
            let src = crates_dir.join(c).join("src");
            println!("cargo:rerun-if-changed={}", src.display());
            let mut files = Vec::new();
            walk(&src, &mut files);
            for f in files {
                let rel = f.strip_prefix(&crates_dir).unwrap().to_string_lossy().replace('\\', "/");
                let data = std::fs::read(&f).unwrap();
                put(&mut raw, rel.as_bytes());
                put(&mut raw, &data);
            }
        }
        let packed = miniz_oxide::deflate::compress_to_vec(&raw, 9);
        std::fs::write(&out, &packed).unwrap();
        if std::env::var_os("GLOWKIT_CLI_WRITE_BUNDLE").is_some() {
            std::fs::write(&shipped, &packed).unwrap();
        }
    } else if shipped.exists() {
        println!("cargo:rerun-if-changed=sources.bundle");
        std::fs::copy(&shipped, &out).unwrap();
    } else {
        println!("cargo:warning=glowkit-cli: no component sources found; `glowkit add` will fall back to dependency instructions");
        std::fs::write(&out, miniz_oxide::deflate::compress_to_vec(&[], 1)).unwrap();
    }
}
