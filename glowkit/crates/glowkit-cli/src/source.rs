//! The embedded glowkit sources: per-crate file trees, a light module model and a path resolver.

use crate::lex::{self, Item, Kind, Tok, Vis};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};

static BUNDLE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sources.bundle"));

/// Crate metadata: package name, module name under `glowkit::`, cargo feature of the umbrella crate.
pub const CRATES: &[(&str, &str, &str)] = &[
    ("glowkit-text", "text", "text"),
    ("glowkit-backgrounds", "backgrounds", "backgrounds"),
    ("glowkit-effects", "effects", "effects"),
    ("glowkit-components", "components", "components"),
    ("glowkit-charts", "charts", "charts"),
    ("glowkit-ai", "ai", "ai"),
    ("glowkit-blocks", "blocks", "blocks"),
    ("glowkit-motion", "motion", "motion"),
    ("glowkit-3d", "three_d", "three-d"),
    ("glowkit-canvas", "canvas", "canvas"),
    ("glowkit-templates", "templates", "templates"),
    ("glowkit-interactions", "interactions", "interactions"),
    ("glowkit-agents", "agents", "agents"),
];

/// `glowkit-text` -> `text`.
pub fn short_name(krate: &str) -> &'static str {
    CRATES.iter().find(|c| c.0 == krate).map(|c| c.1).unwrap_or("unknown")
}

/// `glowkit-text` -> `text` (cargo feature).
pub fn feature_name(krate: &str) -> &'static str {
    CRATES.iter().find(|c| c.0 == krate).map(|c| c.2).unwrap_or("unknown")
}

/// `glowkit_text` (Rust identifier) -> `glowkit-text`.
pub fn crate_from_ident(id: &str) -> Option<&'static str> {
    CRATES.iter().find(|c| c.0.replace('-', "_") == id).map(|c| c.0)
}

/// A file id: (crate index, path relative to `src/`).
pub type FileId = (usize, String);

/// What a path resolves to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Res {
    /// An item defined in this file.
    Item(usize, String),
    /// A module (crate index, module path).
    Module(usize, Vec<String>),
}

/// A parsed file.
pub struct File {
    pub src: String,
    pub toks: Vec<Tok>,
    pub items: Vec<Item>,
    /// Indices into `items` that are compiled out for users (`cfg(feature = "demos")`, `cfg(test)`).
    pub stripped: HashSet<usize>,
    /// Module path of this file.
    pub module: Vec<String>,
}

impl File {
    fn parse(rel: &str, src: String) -> File {
        let toks = lex::lex(&src);
        let items = lex::items(&toks, &src, 0, toks.len());
        let mut stripped = HashSet::new();
        for (i, it) in items.iter().enumerate() {
            for &(a, b) in &it.attrs {
                let text = &src[toks[a].start..toks[b - 1].end];
                let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
                let demos = compact.contains("feature=\"demos\"") && !compact.contains("not(");
                let test = compact == "#[cfg(test)]" || compact.starts_with("#[cfg(all(test");
                if (demos || test) && compact.starts_with("#[cfg(") {
                    stripped.insert(i);
                }
            }
        }
        File { module: module_of(rel), src, toks, items, stripped }
    }

    pub fn tok(&self, i: usize) -> &str {
        self.toks.get(i).map(|t| t.text(&self.src)).unwrap_or("")
    }

    /// Live (not stripped) items.
    pub fn live(&self) -> impl Iterator<Item = (usize, &Item)> {
        self.items.iter().enumerate().filter(|(i, _)| !self.stripped.contains(i))
    }

    /// `mod name;` declarations (file modules) that are live.
    pub fn child_mods(&self) -> Vec<(usize, String)> {
        self.live()
            .filter(|(_, it)| it.kind == "mod" && self.tok(it.last - 1) == ";")
            .map(|(i, it)| (i, it.name.clone().unwrap_or_default()))
            .collect()
    }

    /// Use paths of a `use` item: (segments, alias).
    pub fn use_paths(&self, it: &Item) -> Vec<lex::UsePath> {
        let mut j = it.head;
        while j < it.last && self.tok(j) != "use" {
            j += 1;
        }
        let end = if self.tok(it.last - 1) == ";" { it.last - 1 } else { it.last };
        lex::use_paths(&self.toks, &self.src, j + 1, end)
    }
}

/// `a/b.rs` -> [a, b]; `a/mod.rs` -> [a]; `lib.rs` -> [].
pub fn module_of(rel: &str) -> Vec<String> {
    let p = rel.trim_end_matches(".rs");
    let mut segs: Vec<String> = p.split('/').map(String::from).collect();
    if segs.last().map(|s| s == "mod" || s == "lib").unwrap_or(false) {
        segs.pop();
    }
    if segs.len() == 1 && segs[0] == "lib" {
        segs.clear();
    }
    segs
}

pub struct Crate {
    pub name: String,
    pub files: BTreeMap<String, File>,
    /// Raw non-Rust files (assets).
    pub assets: BTreeMap<String, Vec<u8>>,
    /// module path -> file
    pub mods: HashMap<Vec<String>, String>,
    /// self type name -> files with `impl ... for Name` / `impl Name`
    pub impls: HashMap<String, Vec<String>>,
    /// macro_rules name -> files
    pub macros: HashMap<String, Vec<String>>,
    /// public item name -> files (live files only)
    pub pub_items: HashMap<String, Vec<String>>,
}

/// Cache key for [`Sources::find_ns`]: (crate, module, name, prefer module).
type FindKey = (usize, Vec<String>, String, bool);

pub struct Sources {
    pub crates: Vec<Crate>,
    cache: RefCell<HashMap<FindKey, Option<Res>>>,
}

fn take<'a>(b: &mut &'a [u8]) -> Option<&'a [u8]> {
    if b.len() < 4 {
        return None;
    }
    let n = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
    let (x, rest) = b[4..].split_at(n.min(b.len() - 4));
    *b = rest;
    Some(x)
}

impl Sources {
    pub fn load() -> Sources {
        let raw = miniz_oxide::inflate::decompress_to_vec(BUNDLE).unwrap_or_default();
        let mut by_crate: BTreeMap<String, Vec<(String, Vec<u8>)>> = BTreeMap::new();
        let mut b: &[u8] = &raw;
        while let (Some(p), Some(d)) = (take(&mut b), take(&mut b)) {
            let p = String::from_utf8_lossy(p).to_string();
            let Some((krate, rest)) = p.split_once("/src/") else { continue };
            by_crate.entry(krate.to_string()).or_default().push((rest.to_string(), d.to_vec()));
        }
        let mut crates = Vec::new();
        for (name, _, _) in CRATES {
            let entries = by_crate.remove(*name).unwrap_or_default();
            let mut files = BTreeMap::new();
            let mut assets = BTreeMap::new();
            for (rel, data) in entries {
                if rel.ends_with(".rs") {
                    files.insert(rel.clone(), File::parse(&rel, String::from_utf8_lossy(&data).to_string()));
                } else {
                    assets.insert(rel, data);
                }
            }
            let mut c = Crate {
                name: name.to_string(),
                files,
                assets,
                mods: HashMap::new(),
                impls: HashMap::new(),
                macros: HashMap::new(),
                pub_items: HashMap::new(),
            };
            c.index();
            crates.push(c);
        }
        Sources { crates, cache: RefCell::new(HashMap::new()) }
    }

    pub fn is_empty(&self) -> bool {
        self.crates.iter().all(|c| c.files.is_empty())
    }

    pub fn crate_index(&self, name: &str) -> Option<usize> {
        self.crates.iter().position(|c| c.name == name)
    }

    pub fn file(&self, id: &FileId) -> Option<&File> {
        self.crates.get(id.0)?.files.get(&id.1)
    }

    /// Find `name` as seen from inside module `m` of crate `c`: own items, child modules, imports
    /// and glob imports (followed recursively).
    pub fn find(&self, c: usize, m: &[String], name: &str) -> Option<Res> {
        self.find_ns(c, m, name, false)
    }

    /// Like [`Sources::find`]; `module` prefers a module over a same-named item (the type
    /// namespace in the middle of a path).
    pub fn find_ns(&self, c: usize, m: &[String], name: &str, module: bool) -> Option<Res> {
        let key = (c, m.to_vec(), name.to_string(), module);
        if let Some(r) = self.cache.borrow().get(&key) {
            return r.clone();
        }
        // guard against cycles (glob re-exports between modules)
        self.cache.borrow_mut().insert(key.clone(), None);
        let r = self.find_uncached(c, m, name, module);
        self.cache.borrow_mut().insert(key, r.clone());
        r
    }

    fn find_uncached(&self, c: usize, m: &[String], name: &str, module: bool) -> Option<Res> {
        let krate = &self.crates[c];
        let rel = krate.mods.get(m)?;
        let f = &krate.files[rel];
        let mut globs = Vec::new();
        let mut as_module = None;
        for (_, it) in f.live() {
            match it.kind {
                "mod" if it.name.as_deref() == Some(name) => {
                    let mut p = m.to_vec();
                    p.push(name.to_string());
                    let r = if f.tok(it.last - 1) == ";" {
                        Res::Module(c, p)
                    } else {
                        // inline module: treat as part of this file
                        Res::Item(c, rel.clone())
                    };
                    if module {
                        return Some(r);
                    }
                    as_module = Some(r);
                }
                "use" => {
                    for (path, alias) in f.use_paths(it) {
                        let last = path.last().map(String::as_str).unwrap_or("");
                        if last == "*" {
                            globs.push(path);
                            continue;
                        }
                        let eff = if let Some(a) = &alias {
                            a.as_str()
                        } else if last == "self" && path.len() >= 2 {
                            path[path.len() - 2].as_str()
                        } else {
                            last
                        };
                        if eff == name {
                            let p = if last == "self" { path[..path.len() - 1].to_vec() } else { path.clone() };
                            if let Some(r) = self.resolve_ns(c, m, &p, module) {
                                return Some(r);
                            }
                        }
                    }
                }
                "impl" | "inner_attr" | "other" | "extern" => {}
                _ => {
                    if it.name.as_deref() == Some(name) {
                        return Some(Res::Item(c, rel.clone()));
                    }
                }
            }
        }
        for g in globs {
            if let Some(Res::Module(gc, gm)) = self.resolve_ns(c, m, &g[..g.len() - 1], true) {
                if let Some(r) = self.find_ns(gc, &gm, name, module) {
                    return Some(r);
                }
            }
        }
        as_module
    }

    /// Resolve a path (as written inside module `m` of crate `c`). External paths give `None`.
    pub fn resolve(&self, c: usize, m: &[String], path: &[String]) -> Option<Res> {
        self.resolve_ns(c, m, path, false)
    }

    /// Resolve a path; `module` says whether its last segment should prefer a module.
    pub fn resolve_ns(&self, c: usize, m: &[String], path: &[String], module: bool) -> Option<Res> {
        let n = path.len();
        let mut segs = path.iter().map(String::as_str).enumerate().map(|(i, s)| (i + 1 == n, s)).peekable();
        let (only, first) = segs.next()?;
        let mut cur = match first {
            "crate" => Res::Module(c, vec![]),
            "self" => Res::Module(c, m.to_vec()),
            "super" => {
                let mut p = m.to_vec();
                p.pop();
                while segs.peek().map(|s| s.1) == Some("super") {
                    segs.next();
                    p.pop();
                }
                Res::Module(c, p)
            }
            "*" => return Some(Res::Module(c, m.to_vec())),
            s => {
                if let Some(k) = crate_from_ident(s).and_then(|k| self.crate_index(k)) {
                    Res::Module(k, vec![])
                } else {
                    self.find_ns(c, m, s, !only || module)?
                }
            }
        };
        for (last, s) in segs {
            match &cur {
                Res::Item(..) => return Some(cur),
                Res::Module(mc, mm) => {
                    if s == "*" || s == "self" {
                        return Some(cur);
                    }
                    let (mc, mm) = (*mc, mm.clone());
                    cur = self.find_ns(mc, &mm, s, !last || module)?;
                }
            }
        }
        Some(cur)
    }

    /// The file a resolution lives in.
    pub fn res_file(&self, r: &Res) -> Option<FileId> {
        match r {
            Res::Item(c, f) => Some((*c, f.clone())),
            Res::Module(c, m) => self.crates[*c].mods.get(m).map(|f| (*c, f.clone())),
        }
    }
}

impl Crate {
    fn index(&mut self) {
        // module map + live files (reachable from lib.rs through live `mod x;` declarations)
        let mut all_mods: HashMap<Vec<String>, String> = HashMap::new();
        for rel in self.files.keys() {
            all_mods.insert(module_of(rel), rel.clone());
        }
        let mut live = Vec::new();
        let mut stack = vec![Vec::<String>::new()];
        while let Some(m) = stack.pop() {
            let Some(rel) = all_mods.get(&m) else { continue };
            live.push(rel.clone());
            self.mods.insert(m.clone(), rel.clone());
            for (_, name) in self.files[rel].child_mods() {
                let mut p = m.clone();
                p.push(name);
                stack.push(p);
            }
        }
        for rel in live {
            let f = &self.files[&rel];
            for (_, it) in f.live() {
                match it.kind {
                    "impl" => {
                        if let Some(t) = lex::impl_self_type(&f.toks, &f.src, it) {
                            self.impls.entry(t).or_default().push(rel.clone());
                        }
                    }
                    "macro_rules" => {
                        if let Some(n) = &it.name {
                            self.macros.entry(n.clone()).or_default().push(rel.clone());
                        }
                    }
                    "use" | "mod" | "inner_attr" | "other" => {}
                    _ => {
                        if it.vis == Vis::Pub {
                            if let Some(n) = &it.name {
                                self.pub_items.entry(n.clone()).or_default().push(rel.clone());
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Paths and bare identifiers appearing in a token range, for dependency scanning.
pub struct Refs {
    /// Multi-segment paths (`a::b::C`).
    pub paths: Vec<Vec<String>>,
    /// Single identifiers (not after `.` or `::`).
    pub bare: Vec<String>,
    /// Macro invocations `name!`.
    pub macros: Vec<String>,
}

pub fn refs(f: &File, from: usize, to: usize) -> Refs {
    let mut r = Refs { paths: vec![], bare: vec![], macros: vec![] };
    let mut i = from;
    while i < to {
        let t = f.toks[i];
        if t.kind != Kind::Ident {
            i += 1;
            continue;
        }
        let prev = if i > 0 { f.tok(i - 1) } else { "" };
        if prev == "." || prev == "::" || prev == "'" {
            i += 1;
            continue;
        }
        // `$crate` and macro metavariables
        if prev == "$" {
            if f.tok(i) == "crate" {
                // treat like `crate`
            } else {
                i += 1;
                continue;
            }
        }
        let mut segs = vec![t.text(&f.src).to_string()];
        let mut j = i + 1;
        while j + 1 < to && f.tok(j) == "::" {
            let n = f.toks[j + 1];
            if n.kind == Kind::Ident || f.tok(j + 1) == "*" {
                segs.push(n.text(&f.src).to_string());
                j += 2;
            } else {
                break; // `::<` turbofish or `::{` (use trees are handled separately)
            }
        }
        if segs.len() == 1 {
            if f.tok(i + 1) == "!" {
                r.macros.push(segs.pop().unwrap());
            } else {
                r.bare.push(segs.pop().unwrap());
            }
        } else {
            r.paths.push(segs);
        }
        i = j;
    }
    r
}
