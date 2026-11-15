//! `glowkit add`: copy a component's source (plus the crate-private files it needs) into a project.

use crate::lex::Kind;
use crate::source::{self, refs, FileId, Res, Sources};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Cargo dependencies of each crate on sibling glowkit crates (used to find demo types that live
/// in a dependency).
fn crate_deps(krate: &str) -> &'static [&'static str] {
    match krate {
        "glowkit-agents" => &["glowkit-ai", "glowkit-effects", "glowkit-charts"],
        "glowkit-interactions" => &["glowkit-effects"],
        "glowkit-templates" => &[
            "glowkit-components",
            "glowkit-charts",
            "glowkit-text",
            "glowkit-ai",
            "glowkit-effects",
            "glowkit-backgrounds",
        ],
        _ => &[],
    }
}

/// A public item the component's usage code refers to.
#[derive(Clone, Debug)]
pub struct Root {
    pub krate: usize,
    pub file: String,
    /// Path below the crate module, e.g. `text::ShinyText`.
    pub path: String,
}

/// Find the public items a usage snippet refers to.
pub fn roots(src: &Sources, krate: &str, code: &str) -> Vec<Root> {
    let Some(k) = src.crate_index(krate) else { return vec![] };
    let toks = crate::lex::lex(code);
    let mut names: Vec<String> = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        if t.kind != Kind::Ident {
            continue;
        }
        let prev = if i > 0 { toks[i - 1].text(code) } else { "" };
        if prev == "." || prev == "::" {
            continue;
        }
        let s = t.text(code).to_string();
        if !names.contains(&s) {
            names.push(s);
        }
    }
    let mut out: Vec<Root> = Vec::new();
    for n in names {
        let mut found = None;
        if let Some(Res::Item(c, f)) = src.find(k, &[], &n) {
            if is_pub_item(src, c, &f, &n) {
                found = Some((c, f, format!("{}::{}", source::short_name(krate), n)));
            }
        }
        if found.is_none() {
            if let Some(files) = src.crates[k].pub_items.get(&n) {
                let f = files[0].clone();
                let m = source::module_of(&f);
                let mut p = vec![source::short_name(krate).to_string()];
                p.extend(m);
                p.push(n.clone());
                found = Some((k, f, p.join("::")));
            }
        }
        if found.is_none() && n.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
            for d in crate_deps(krate) {
                let Some(dk) = src.crate_index(d) else { continue };
                if let Some(Res::Item(c, f)) = src.find(dk, &[], &n) {
                    if is_pub_item(src, c, &f, &n) {
                        found = Some((c, f, format!("{}::{}", source::short_name(d), n)));
                        break;
                    }
                }
            }
        }
        if let Some((c, file, path)) = found {
            out.push(Root { krate: c, file, path });
        }
    }
    out
}

fn is_pub_item(src: &Sources, c: usize, f: &str, name: &str) -> bool {
    let Some(file) = src.crates[c].files.get(f) else { return false };
    file.live().any(|(_, it)| it.name.as_deref() == Some(name) && it.vis == crate::lex::Vis::Pub && it.kind != "mod")
}

/// The set of files to copy for some starting files (transitively, across glowkit crates).
pub struct Closure {
    pub files: BTreeSet<FileId>,
    pub unresolved: BTreeSet<String>,
}

fn ancestors(src: &Sources, id: &FileId) -> Vec<FileId> {
    let Some(f) = src.file(id) else { return vec![] };
    let c = &src.crates[id.0];
    let mut out = Vec::new();
    for n in 0..f.module.len() {
        if let Some(rel) = c.mods.get(&f.module[..n]) {
            out.push((id.0, rel.clone()));
        }
    }
    out
}

fn is_local_root(seg: &str) -> bool {
    matches!(seg, "crate" | "self" | "super") || source::crate_from_ident(seg).is_some()
}

/// Direct dependencies of one file.
fn deps(src: &Sources, id: &FileId, unresolved: &mut BTreeSet<String>) -> Vec<FileId> {
    let Some(f) = src.file(id) else { return vec![] };
    let c = id.0;
    let m = f.module.clone();
    let children: BTreeSet<String> = f.child_mods().into_iter().map(|(_, n)| n).collect();
    let mut out: Vec<FileId> = Vec::new();
    let push = |r: Option<Res>, out: &mut Vec<FileId>| {
        if let Some(r) = r {
            if let Some(fid) = src.res_file(&r) {
                if &fid != id {
                    out.push(fid);
                }
            }
        }
    };
    for (_, it) in f.live() {
        match it.kind {
            "inner_attr" => {}
            "mod" if f.tok(it.last - 1) == ";" => {}
            "use" => {
                for (path, _) in f.use_paths(it) {
                    let first = if path.first().map(String::as_str) == Some("self") { path.get(1) } else { path.first() };
                    if first.is_some_and(|s| children.contains(s)) {
                        continue; // re-export of a child module: kept only if that child is copied
                    }
                    let r = src.resolve(c, &m, &path);
                    if r.is_none() && path.first().is_some_and(|s| is_local_root(s)) {
                        unresolved.insert(format!("{}: use {}", src.crates[c].name, path.join("::")));
                    }
                    push(r, &mut out);
                }
            }
            _ => {
                let r = refs(f, it.first, it.last);
                for p in &r.paths {
                    if p[0] == "Self" {
                        continue;
                    }
                    let res = src.resolve(c, &m, p);
                    if res.is_none() && is_local_root(&p[0]) && p.len() > 1 {
                        // `crate::Foo::new` resolves to Foo; only a missing first item is an error
                        unresolved.insert(format!("{}: {}", src.crates[c].name, p.join("::")));
                    }
                    push(res, &mut out);
                }
                for b in &r.bare {
                    push(src.find(c, &m, b), &mut out);
                }
                for mac in &r.macros {
                    if let Some(res) = src.find(c, &m, mac) {
                        push(Some(res), &mut out);
                    } else if let Some(files) = src.crates[c].macros.get(mac) {
                        if !files.contains(&id.1) {
                            out.extend(files.iter().map(|f| (c, f.clone())));
                        }
                    }
                }
            }
        }
        // impls of types defined here that live in other files of the crate
        if matches!(it.kind, "struct" | "enum" | "union" | "type") {
            if let Some(n) = &it.name {
                if let Some(files) = src.crates[c].impls.get(n) {
                    out.extend(files.iter().filter(|f| **f != id.1).map(|f| (c, f.clone())));
                }
            }
        }
    }
    out
}

pub fn closure(src: &Sources, start: &[FileId]) -> Closure {
    let mut files = BTreeSet::new();
    let mut unresolved = BTreeSet::new();
    let mut work: Vec<FileId> = start.to_vec();
    while let Some(id) = work.pop() {
        if src.file(&id).is_none() || !files.insert(id.clone()) {
            continue;
        }
        work.extend(ancestors(src, &id));
        work.extend(deps(src, &id, &mut unresolved));
    }
    Closure { files, unresolved }
}

/// Where a copied file goes, relative to the glowkit dir: `text/mod.rs`, `effects/micro_a/common.rs`.
pub fn dest_rel(src: &Sources, id: &FileId) -> String {
    let short = source::short_name(&src.crates[id.0].name);
    let rel = if id.1 == "lib.rs" { "mod.rs".to_string() } else { id.1.clone() };
    format!("{short}/{rel}")
}

/// Render one copied file with paths rewritten so it lives at `<base>::<short>`.
pub fn render(src: &Sources, id: &FileId, copied: &BTreeSet<FileId>, base: &str) -> String {
    let f = src.file(id).expect("file");
    let c = id.0;
    let m = &f.module;
    let short = source::short_name(&src.crates[c].name);
    let children: BTreeMap<String, ()> = f.child_mods().into_iter().map(|(_, n)| (n, ())).collect();
    // (start, end, replacement)
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let span = |a: usize, b: usize| -> (usize, usize) {
        let s = f.toks[a].start;
        let mut e = f.toks[b - 1].end;
        // swallow the rest of the line
        let bytes = f.src.as_bytes();
        while e < bytes.len() && (bytes[e] == b' ' || bytes[e] == b'\t') {
            e += 1;
        }
        if e < bytes.len() && bytes[e] == b'\n' {
            e += 1;
        }
        (s, e)
    };
    let mut skip_ranges: Vec<(usize, usize)> = Vec::new();
    for (i, it) in f.items.iter().enumerate() {
        if it.last <= it.first {
            continue;
        }
        let drop = if f.stripped.contains(&i) {
            true
        } else if it.kind == "inner_attr" {
            let t: String = f.src[f.toks[it.first].start..f.toks[it.last - 1].end].chars().filter(|c| !c.is_whitespace()).collect();
            t.starts_with("#![warn(") || t.starts_with("#![deny(") || t.starts_with("#![forbid(")
        } else if it.kind == "mod" && f.tok(it.last - 1) == ";" {
            let mut p = m.clone();
            p.push(it.name.clone().unwrap_or_default());
            match src.crates[c].mods.get(&p) {
                Some(rel) => !copied.contains(&(c, rel.clone())),
                None => true,
            }
        } else {
            false
        };
        if drop {
            let (s, e) = span(it.first, it.last);
            edits.push((s, e, String::new()));
            skip_ranges.push((it.first, it.last));
            continue;
        }
        if it.kind == "use" {
            let paths = f.use_paths(it);
            let keep: Vec<bool> = paths
                .iter()
                .map(|(p, _)| {
                    let first = if p.first().map(String::as_str) == Some("self") { p.get(1) } else { p.first() };
                    let local = p.first().is_some_and(|s| is_local_root(s)) || first.is_some_and(|s| children.contains_key(s));
                    match src.resolve(c, m, p) {
                        Some(r) => match src.res_file(&r) {
                            Some(fid) => copied.contains(&fid) || src.file(&fid).is_none(),
                            None => !local,
                        },
                        None => !local,
                    }
                })
                .collect();
            if keep.iter().all(|k| *k) {
                continue;
            }
            let (s, e) = span(it.first, it.last);
            skip_ranges.push((it.first, it.last));
            let mut rep = String::new();
            if keep.iter().any(|k| *k) {
                // re-render the kept paths one per line
                let mut j = it.head;
                while j < it.last && f.tok(j) != "use" {
                    j += 1;
                }
                let pre = if it.head < j { &f.src[f.toks[it.head].start..f.toks[j - 1].end] } else { "" };
                let attrs: String =
                    it.attrs.iter().map(|&(a, b)| format!("{}\n", &f.src[f.toks[a].start..f.toks[b - 1].end])).collect();
                for ((p, alias), k) in paths.iter().zip(&keep) {
                    if !*k {
                        continue;
                    }
                    let path = p.iter().map(|s| rewrite_seg(s, base, short)).collect::<Vec<_>>().join("::");
                    let alias = alias.as_ref().map(|a| format!(" as {a}")).unwrap_or_default();
                    let vis = if pre.is_empty() { String::new() } else { format!("{pre} ") };
                    rep.push_str(&format!("{attrs}{vis}use {path}{alias};\n"));
                }
            }
            edits.push((s, e, rep));
        }
    }
    // token rewrites outside dropped items
    let mut fence_open = false;
    let mut k = 0;
    let mut si = 0;
    skip_ranges.sort();
    while k < f.toks.len() {
        while si < skip_ranges.len() && skip_ranges[si].1 <= k {
            si += 1;
        }
        if si < skip_ranges.len() && skip_ranges[si].0 <= k && k < skip_ranges[si].1 {
            k = skip_ranges[si].1;
            continue;
        }
        let t = f.toks[k];
        let text = t.text(&f.src);
        if text == "#" && (f.tok(k + 1) == "[" || (f.tok(k + 1) == "!" && f.tok(k + 2) == "[")) {
            let open = if f.tok(k + 1) == "[" { k + 1 } else { k + 2 };
            let end = crate::lex::skip_group(&f.toks, &f.src, open);
            let raw = &f.src[t.start..f.toks[end - 1].end];
            if let Some(rep) = demos_attr(raw) {
                edits.push((t.start, f.toks[end - 1].end, rep));
                k = end;
                continue;
            }
        }
        match t.kind {
            Kind::Ident => {
                if text == "crate" && f.tok(k + 1) == "::" {
                    edits.push((t.start, t.end, format!("{base}::{short}")));
                } else if let Some(other) = source::crate_from_ident(text) {
                    let target = format!("{base}::{}", source::short_name(other));
                    if k > 0 && f.tok(k - 1) == "::" && (k < 2 || f.toks[k - 2].kind != Kind::Ident) {
                        edits.push((f.toks[k - 1].start, t.end, target));
                    } else {
                        edits.push((t.start, t.end, target));
                    }
                }
            }
            Kind::OuterDoc | Kind::InnerDoc => {
                // doc examples reference the original crates; keep them out of doctests
                let body = text.trim_start_matches(['/', '!', '*']).trim_start();
                if let Some(lang) = body.strip_prefix("```") {
                    if fence_open {
                        fence_open = false;
                    } else {
                        fence_open = true;
                        let l = lang.trim();
                        if l.is_empty() || l.contains("rust") || l.contains("no_run") || l.contains("should_panic") || l.contains("compile_fail") || l.contains("edition") {
                            let off = t.start + text.find("```").unwrap() + 3;
                            edits.push((off, off + lang.len(), "ignore".to_string()));
                        }
                    }
                }
            }
            _ => {}
        }
        k += 1;
    }
    edits.sort_by_key(|e| (e.0, e.1));
    let mut out = String::with_capacity(f.src.len() + 256);
    let mut pos = 0;
    for (s, e, r) in edits {
        if s < pos {
            continue;
        }
        out.push_str(&f.src[pos..s]);
        out.push_str(&r);
        pos = e;
        if r.is_empty() && (out.ends_with("\n\n") || out.is_empty()) {
            // don't leave a run of blank lines where an item was removed
            while f.src[pos..].starts_with('\n') {
                pos += 1;
            }
        }
    }
    out.push_str(&f.src[pos..]);
    out
}

/// Rewrite attributes that depend on the gallery-only `demos` feature (never enabled in a user's
/// project): `cfg(not(demos))` goes away, `cfg_attr(not(demos), x)` becomes `x`,
/// `cfg_attr(demos, x)` goes away. Returns `None` for unrelated attributes.
fn demos_attr(raw: &str) -> Option<String> {
    let compact: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if !compact.contains("feature=\"demos\"") {
        return None;
    }
    let inner = compact.starts_with("#!");
    let bang = if inner { "#![" } else { "#[" };
    let body = compact.strip_prefix(bang)?.strip_suffix(']')?;
    if body == "cfg(not(feature=\"demos\"))" {
        return Some(String::new());
    }
    if let Some(rest) = body.strip_prefix("cfg_attr(not(feature=\"demos\"),") {
        let a = rest.strip_suffix(')')?;
        return Some(format!("{bang}{a}]"));
    }
    if body.starts_with("cfg_attr(feature=\"demos\",") {
        return Some(String::new());
    }
    None
}

fn rewrite_seg(seg: &str, base: &str, short: &str) -> String {
    if seg == "crate" {
        format!("{base}::{short}")
    } else if let Some(other) = source::crate_from_ident(seg) {
        format!("{base}::{}", source::short_name(other))
    } else {
        seg.to_string()
    }
}

/// Header prepended to every copied file.
fn header(src: &Sources, id: &FileId) -> String {
    format!(
        "// Copied from {} {} (src/{}) by `glowkit add`. You own this code now: edit freely.\n",
        src.crates[id.0].name,
        env!("CARGO_PKG_VERSION"),
        id.1
    )
}

/// One file to write.
pub struct Output {
    pub rel: String,
    pub content: String,
    /// Already on disk (and will be kept unless it is a module file).
    pub exists: bool,
    pub is_module_file: bool,
}

/// Plan the files for a set of copied files, merging with what already exists in `dir`.
pub fn plan_outputs(
    src: &Sources,
    dir: &Path,
    base: &str,
    new: &BTreeSet<FileId>,
    overwrite: bool,
) -> (BTreeSet<FileId>, Vec<Output>) {
    // what was copied before
    let mut all = new.clone();
    for (ci, c) in src.crates.iter().enumerate() {
        for rel in c.files.keys() {
            let id = (ci, rel.clone());
            if dir.join(dest_rel(src, &id)).exists() {
                all.insert(id);
            }
        }
    }
    let mut outs = Vec::new();
    for id in &all {
        let f = src.file(id).unwrap();
        let is_module_file = !f.child_mods().is_empty() || id.1 == "lib.rs";
        let rel = dest_rel(src, id);
        let exists = dir.join(&rel).exists();
        if exists && !is_module_file && !overwrite {
            outs.push(Output { rel, content: String::new(), exists, is_module_file });
            continue;
        }
        let mut content = header(src, id);
        if id.1 == "lib.rs" {
            content.push_str("#![allow(dead_code, unused_imports, unused_macros, unused_variables, unused_mut)]\n");
        }
        content.push_str(&render(src, id, &all, base));
        outs.push(Output { rel, content, exists, is_module_file });
    }
    // assets referenced with include_bytes!/include_str! inside copied files
    for id in &all {
        let c = &src.crates[id.0];
        let f = src.file(id).unwrap();
        for (k, t) in f.toks.iter().enumerate() {
            let s = t.text(&f.src);
            if (s == "include_bytes" || s == "include_str") && f.tok(k + 1) == "!" {
                let lit = f.tok(k + 3).trim_matches('"');
                let dirn = Path::new(&id.1).parent().unwrap_or(Path::new(""));
                let rel = dirn.join(lit).to_string_lossy().replace('\\', "/");
                if let Some(data) = c.assets.get(&rel) {
                    let short = source::short_name(&c.name);
                    let dest = if id.1 == "lib.rs" { format!("{short}/{lit}") } else { format!("{short}/{rel}") };
                    outs.push(Output {
                        exists: dir.join(&dest).exists(),
                        rel: dest,
                        content: String::from_utf8_lossy(data).to_string(),
                        is_module_file: false,
                    });
                }
            }
        }
    }
    (all, outs)
}

/// Ensure `<dir>/mod.rs` declares every copied crate module. Returns the new content (or None if unchanged).
pub fn root_mod(dir: &Path, shorts: &BTreeSet<String>) -> Option<String> {
    let path = dir.join("mod.rs");
    let existing = std::fs::read_to_string(&path).ok();
    let mut content = existing.clone().unwrap_or_else(|| {
        "//! glowkit components copied into this project by `glowkit add`.\n//!\n//! This is your code now: edit it freely. Copied files only depend on the `glowkit-core` and\n//! `egui` crates. Run `glowkit add <id>` again to bring in more components.\n\n".to_string()
    });
    let mut changed = existing.is_none();
    for s in shorts {
        let decl = format!("pub mod {s};");
        if !content.lines().any(|l| l.trim() == decl) {
            if !content.ends_with('\n') && !content.is_empty() {
                content.push('\n');
            }
            content.push_str(&decl);
            content.push('\n');
            changed = true;
        }
    }
    changed.then_some(content)
}

/// Module path (`crate::glowkit`) for a directory under the project's `src/`.
pub fn module_base(project: &Path, dir: &Path) -> Result<String, String> {
    let src = project.join("src");
    let abs = if dir.is_absolute() { dir.to_path_buf() } else { project.join(dir) };
    let rel = abs.strip_prefix(&src).map_err(|_| format!("--dir must be inside {}", src.display()))?;
    let segs: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
    if segs.is_empty() {
        return Err("--dir must be a subdirectory of src/, e.g. src/glowkit".into());
    }
    for s in &segs {
        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || s.starts_with(|c: char| c.is_ascii_digit()) {
            return Err(format!("`{s}` is not a valid Rust module name"));
        }
    }
    Ok(format!("crate::{}", segs.join("::")))
}

/// Find the enclosing cargo project (directory with Cargo.toml).
pub fn find_project(start: &Path) -> Option<PathBuf> {
    let mut d = Some(start);
    while let Some(p) = d {
        if p.join("Cargo.toml").exists() {
            return Some(p.to_path_buf());
        }
        d = p.parent();
    }
    None
}
