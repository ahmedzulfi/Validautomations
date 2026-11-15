//! `glowkit`: add glowkit components to your project as source you own (shadcn-style), and
//! scaffold new eframe apps.
//!
//! ```text
//! glowkit list [query]                         list / search components
//! glowkit info <id>                            crate, feature, use path and usage of a component
//! glowkit add <id>... [--dir src/glowkit]      copy components into your project
//! glowkit init <name> [--template blank]       scaffold a new eframe app
//! ```

mod copier;
mod lex;
mod registry;
mod source;
mod templates;

use registry::Component;
use source::Sources;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Copies above this many files are considered too deep to own and fall back to the cargo
/// dependency (unless `--force`). The largest component today needs about 60.
const MAX_FILES: usize = 80;

/// Names re-exported at the root of `glowkit-core` that usage snippets commonly need.
const CORE_NAMES: &[&str] =
    &["ease", "Clock", "Easing", "Spring", "Trigger", "hex", "hsv", "mix", "with_alpha", "Rng", "Theme"];

const HELP: &str = "glowkit: animated egui components you own

USAGE:
    glowkit list [query]                 List components (filter by words in id, name, category)
    glowkit info <id>                    Show crate, feature, use path and a usage snippet
    glowkit add <id>... [options]        Copy components' source into your project
        --dir <path>                     Destination module dir (default: src/glowkit)
        --dry-run                        Show what would be written, write nothing
        --overwrite                      Overwrite files that already exist
        --force                          Copy even components with deep cross-crate dependencies
    glowkit init <name> [--template <t>] Scaffold a new eframe app using glowkit
        templates: blank (default), dashboard, ai-chat, agent-sessions, trading, game
    glowkit help | --version

Copied components depend only on the `glowkit-core` and `egui` crates.";

fn main() -> ExitCode {
    // `glowkit list | head`: exit quietly when stdout is closed instead of panicking.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = info.payload().downcast_ref::<String>().map(String::as_str).unwrap_or("");
        if msg.contains("Broken pipe") {
            std::process::exit(0);
        }
        default_hook(info);
    }));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    let rest = &args[args.len().min(1)..];
    let r = match cmd {
        "list" | "ls" | "search" => cmd_list(rest),
        "info" | "show" => cmd_info(rest),
        "add" => cmd_add(rest),
        "init" | "new" => cmd_init(rest),
        "__plan" => cmd_plan(rest),
        "help" | "-h" | "--help" => {
            println!("{HELP}");
            Ok(())
        }
        "-V" | "--version" | "version" => {
            println!("glowkit {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        other => Err(format!("unknown command `{other}`\n\n{HELP}")),
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Split args into positionals and `--flag [value]` options.
struct Args {
    pos: Vec<String>,
    opts: Vec<(String, Option<String>)>,
}

fn parse(args: &[String], with_value: &[&str]) -> Result<Args, String> {
    let mut a = Args { pos: vec![], opts: vec![] };
    let mut i = 0;
    while i < args.len() {
        let s = &args[i];
        if let Some(flag) = s.strip_prefix("--") {
            if let Some((k, v)) = flag.split_once('=') {
                a.opts.push((k.to_string(), Some(v.to_string())));
            } else if with_value.contains(&flag) {
                let v = args.get(i + 1).ok_or(format!("--{flag} needs a value"))?;
                a.opts.push((flag.to_string(), Some(v.clone())));
                i += 1;
            } else {
                a.opts.push((flag.to_string(), None));
            }
        } else {
            a.pos.push(s.clone());
        }
        i += 1;
    }
    Ok(a)
}

impl Args {
    fn flag(&self, k: &str) -> bool {
        self.opts.iter().any(|(n, _)| n == k)
    }
    fn value(&self, k: &str) -> Option<&str> {
        self.opts.iter().rev().find(|(n, _)| n == k).and_then(|(_, v)| v.as_deref())
    }
    fn check(&self, known: &[&str]) -> Result<(), String> {
        for (n, _) in &self.opts {
            if !known.contains(&n.as_str()) {
                return Err(format!("unknown option --{n}"));
            }
        }
        Ok(())
    }
}

fn find<'a>(all: &'a [Component], id: &str) -> Result<&'a Component, String> {
    all.iter().find(|c| c.id == id).ok_or_else(|| {
        let s = registry::suggest(all, id);
        if s.is_empty() {
            format!("no component `{id}` (try `glowkit list {id}`)")
        } else {
            format!("no component `{id}`. Did you mean: {}?", s.join(", "))
        }
    })
}

fn cmd_list(args: &[String]) -> Result<(), String> {
    let a = parse(args, &[])?;
    a.check(&[])?;
    let all = registry::load();
    let q = a.pos.join(" ");
    let hits = registry::search(&all, &q);
    if hits.is_empty() {
        return Err(format!("no components match `{q}`"));
    }
    let w_id = hits.iter().map(|c| c.id.len()).max().unwrap_or(2).max(2);
    let w_name = hits.iter().map(|c| c.name.chars().count()).max().unwrap_or(4).max(4);
    println!("{:w_id$}  {:w_name$}  CATEGORY", "ID", "NAME");
    for c in &hits {
        println!("{:w_id$}  {:w_name$}  {}", c.id, c.name, c.category);
    }
    println!("\n{} component(s). `glowkit info <id>` for details, `glowkit add <id>` to copy one.", hits.len());
    Ok(())
}

/// `use` lines for the roots, grouped by module (`use glowkit::text::{A, B};`).
fn use_lines(roots: &[copier::Root], prefix: &str) -> Vec<String> {
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    for r in roots {
        let (module, name) = r.path.rsplit_once("::").unwrap_or(("", &r.path));
        match groups.iter_mut().find(|g| g.0 == module) {
            Some(g) => {
                if !g.1.iter().any(|n| n == name) {
                    g.1.push(name.to_string());
                }
            }
            None => groups.push((module.to_string(), vec![name.to_string()])),
        }
    }
    groups
        .into_iter()
        .map(|(m, names)| {
            if names.len() == 1 {
                format!("use {prefix}::{m}::{};", names[0])
            } else {
                format!("use {prefix}::{m}::{{{}}};", names.join(", "))
            }
        })
        .collect()
}

fn cmd_info(args: &[String]) -> Result<(), String> {
    let a = parse(args, &[])?;
    a.check(&[])?;
    let id = a.pos.first().ok_or("usage: glowkit info <id>")?;
    let all = registry::load();
    let c = find(&all, id)?;
    let src = Sources::load();
    let roots = copier::roots(&src, &c.krate, &c.code);
    let mut features: Vec<&str> = vec![source::feature_name(&c.krate)];
    for r in &roots {
        let f = source::feature_name(&src.crates[r.krate].name);
        if !features.contains(&f) {
            features.push(f);
        }
    }
    println!("{} ({})", c.name, c.id);
    println!("  {}\n", c.description);
    println!("category : {}", c.category);
    println!("crate    : {} (or the `glowkit` umbrella crate)", c.krate);
    println!("feature  : glowkit/{}", features.join(", glowkit/"));
    println!(
        "cargo    : glowkit = {{ version = \"0.1\", default-features = false, features = [{}] }}",
        features.iter().map(|f| format!("\"{f}\"")).collect::<Vec<_>>().join(", ")
    );
    if !roots.is_empty() {
        println!("use      : {}", use_lines(&roots, "glowkit").join("\n           "));
    }
    println!("\nusage:\n");
    for l in c.code.lines() {
        println!("    {l}");
    }
    println!("\nOwn the source instead: glowkit add {}", c.id);
    Ok(())
}

fn cmd_plan(args: &[String]) -> Result<(), String> {
    let all = registry::load();
    let src = Sources::load();
    let ids: Vec<&Component> = if args.is_empty() {
        all.iter().collect()
    } else {
        args.iter().map(|i| find(&all, i)).collect::<Result<_, _>>()?
    };
    for c in ids {
        let roots = copier::roots(&src, &c.krate, &c.code);
        let start: Vec<_> = roots.iter().map(|r| (r.krate, r.file.clone())).collect();
        let cl = copier::closure(&src, &start);
        let crates: BTreeSet<usize> = cl.files.iter().map(|f| f.0).collect();
        println!(
            "{}\troots={}\tfiles={}\tcrates={}\tunresolved={}\t{}",
            c.id,
            roots.iter().map(|r| r.path.as_str()).collect::<Vec<_>>().join(","),
            cl.files.len(),
            crates.len(),
            cl.unresolved.len(),
            cl.unresolved.iter().take(3).cloned().collect::<Vec<_>>().join(" | ")
        );
    }
    Ok(())
}

fn cmd_add(args: &[String]) -> Result<(), String> {
    let a = parse(args, &["dir"])?;
    a.check(&["dir", "dry-run", "overwrite", "force"])?;
    if a.pos.is_empty() {
        return Err("usage: glowkit add <id>... [--dir src/glowkit] [--dry-run]".into());
    }
    let dry = a.flag("dry-run");
    // printed only after every file is written, so a closed stdout (`| head`) cannot interrupt writing
    let mut log: Vec<String> = Vec::new();
    let all = registry::load();
    let comps: Vec<&Component> = a.pos.iter().map(|i| find(&all, i)).collect::<Result<_, _>>()?;
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let project = copier::find_project(&cwd).ok_or("no Cargo.toml found here or in a parent directory")?;
    let dir_arg = a.value("dir").unwrap_or("src/glowkit");
    let dir = if Path::new(dir_arg).is_absolute() { PathBuf::from(dir_arg) } else { cwd.join(dir_arg) };
    let dir = normalize(&dir);
    let base = copier::module_base(&project, &dir)?;
    let src = Sources::load();

    let mut wanted = BTreeSet::new();
    let mut copied: Vec<(&Component, Vec<copier::Root>)> = Vec::new();
    let mut fallback: Vec<(&Component, Vec<copier::Root>, String)> = Vec::new();
    for c in comps {
        if src.is_empty() {
            fallback.push((c, vec![], "this build of the CLI has no embedded sources".into()));
            continue;
        }
        let roots = copier::roots(&src, &c.krate, &c.code);
        if roots.is_empty() {
            fallback.push((c, roots, "its usage code names no public glowkit type".into()));
            continue;
        }
        let start: Vec<_> = roots.iter().map(|r| (r.krate, r.file.clone())).collect();
        let cl = copier::closure(&src, &start);
        let crates: BTreeSet<usize> = cl.files.iter().map(|f| f.0).collect();
        if !cl.unresolved.is_empty() {
            let why =
                format!("could not resolve: {}", cl.unresolved.iter().take(3).cloned().collect::<Vec<_>>().join(", "));
            fallback.push((c, roots, why));
            continue;
        }
        if !a.flag("force") && cl.files.len() > MAX_FILES {
            let why = format!(
                "it pulls in {} files from {} glowkit crates (use --force to copy anyway)",
                cl.files.len(),
                crates.len()
            );
            fallback.push((c, roots, why));
            continue;
        }
        let names: Vec<&str> = crates.iter().map(|k| source::short_name(&src.crates[*k].name)).collect();
        log.push(format!("{}: {} file(s) from {}", c.id, cl.files.len(), names.join(", ")));
        wanted.extend(cl.files);
        copied.push((c, roots));
    }

    let rel_dir = dir.strip_prefix(&project).unwrap_or(&dir).to_string_lossy().replace('\\', "/");
    if !wanted.is_empty() {
        let (all_files, outs) = copier::plan_outputs(&src, &dir, &base, &wanted, a.flag("overwrite"));
        let shorts: BTreeSet<String> =
            all_files.iter().map(|f| source::short_name(&src.crates[f.0].name).to_string()).collect();
        let mut wrote = 0;
        let mut kept = 0;
        log.push(format!("{} into {rel_dir}/", if dry { "Would write" } else { "Writing" }));
        for o in &outs {
            let path = dir.join(&o.rel);
            let (verb, write) = if !o.exists {
                ("create", true)
            } else if o.is_module_file {
                let same = std::fs::read_to_string(&path).map(|s| s == o.content).unwrap_or(false);
                if same {
                    ("same  ", false)
                } else {
                    ("update", true)
                }
            } else if a.flag("overwrite") {
                let same = std::fs::read_to_string(&path).map(|s| s == o.content).unwrap_or(false);
                if same {
                    ("same  ", false)
                } else {
                    ("over  ", true)
                }
            } else {
                ("keep  ", false)
            };
            if verb != "same  " && verb != "keep  " {
                log.push(format!("  {verb} {rel_dir}/{}", o.rel));
            }
            if write {
                wrote += 1;
            } else {
                kept += 1;
            }
            if write && !dry {
                if let Some(p) = path.parent() {
                    std::fs::create_dir_all(p).map_err(|e| format!("{}: {e}", p.display()))?;
                }
                std::fs::write(&path, &o.content).map_err(|e| format!("{}: {e}", path.display()))?;
            }
        }
        if let Some(root) = copier::root_mod(&dir, &shorts) {
            log.push(format!("  {} {rel_dir}/mod.rs", if dir.join("mod.rs").exists() { "update" } else { "create" }));
            if !dry {
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                std::fs::write(dir.join("mod.rs"), root).map_err(|e| e.to_string())?;
            }
        }
        log.push(format!(
            "  ({wrote} written, {kept} already present and kept{})",
            if a.flag("overwrite") { "" } else { "; --overwrite replaces them" }
        ));
    }

    for l in &log {
        println!("{l}");
    }
    if !copied.is_empty() {
        next_steps(&project, &dir, &base, &copied);
    }
    for (c, roots, why) in &fallback {
        println!("\n! {} was not copied: {why}.", c.id);
        println!("  Use it from the published crate instead:");
        println!("    cargo add glowkit --no-default-features --features {}", source::feature_name(&c.krate));
        for l in use_lines(roots, "glowkit") {
            println!("    {l}");
        }
        if roots.is_empty() {
            println!("    use glowkit::{}::*;", source::short_name(&c.krate));
        }
    }
    if copied.is_empty() && fallback.is_empty() {
        return Err("nothing to add".into());
    }
    Ok(())
}

fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

fn next_steps(project: &Path, dir: &Path, base: &str, copied: &[(&Component, Vec<copier::Root>)]) {
    let manifest = std::fs::read_to_string(project.join("Cargo.toml")).unwrap_or_default();
    let has_dep = |name: &str| {
        manifest.lines().any(|l| {
            let l = l.trim_start();
            l.starts_with(&format!("{name} "))
                || l.starts_with(&format!("{name}="))
                || l.starts_with(&format!("{name}."))
        })
    };
    let segs: Vec<&str> = base.trim_start_matches("crate::").split("::").collect();
    let top = segs[0];
    let top_decl = format!("mod {top};");
    let entry = ["src/main.rs", "src/lib.rs"]
        .iter()
        .map(|p| project.join(p))
        .find(|p| p.exists())
        .unwrap_or_else(|| project.join("src/main.rs"));
    let entry_src = std::fs::read_to_string(&entry).unwrap_or_default();
    let declared = entry_src.lines().any(|l| {
        let l = l.trim();
        l == top_decl || l == format!("pub {top_decl}") || l == format!("pub(crate) {top_decl}")
    });
    let entry_rel = entry.strip_prefix(project).unwrap_or(&entry).to_string_lossy().to_string();
    println!("\nNext steps:");
    let mut n = 1;
    if segs.len() == 1 {
        if declared {
            println!("  {n}. {entry_rel} already has `{top_decl}` ✓");
        } else {
            println!("  {n}. Add to {entry_rel}:\n\n         {top_decl}\n");
        }
    } else {
        let parent = segs[..segs.len() - 1].join("::");
        println!("  {n}. Declare the module: `mod {};` inside `crate::{parent}`", segs[segs.len() - 1]);
    }
    n += 1;
    let mut missing = Vec::new();
    if !has_dep("glowkit-core") {
        missing.push("glowkit-core = \"0.1\"");
    }
    if !has_dep("egui") {
        missing.push("egui = \"0.37\"");
    }
    if missing.is_empty() {
        println!("  {n}. Cargo.toml already has glowkit-core and egui ✓");
    } else {
        println!("  {n}. Add to Cargo.toml [dependencies]:\n");
        for m in &missing {
            println!("         {m}");
        }
        println!();
    }
    n += 1;
    println!("  {n}. Use it:\n");
    let mut lines: Vec<String> = Vec::new();
    for (_, roots) in copied {
        for l in use_lines(roots, base) {
            if !lines.contains(&l) {
                lines.push(l);
            }
        }
    }
    let core: BTreeSet<&str> = copied
        .iter()
        .flat_map(|(c, _)| {
            let toks = lex::lex(&c.code);
            toks.iter()
                .map(|t| t.text(&c.code))
                .filter(|t| CORE_NAMES.contains(t))
                .map(|t| CORE_NAMES[CORE_NAMES.iter().position(|n| n == &t).unwrap()])
                .collect::<Vec<_>>()
        })
        .collect();
    if !core.is_empty() {
        lines.push(format!("use glowkit_core::{{{}}};", core.into_iter().collect::<Vec<_>>().join(", ")));
    }
    for l in &lines {
        println!("         {l}");
    }
    for (c, _) in copied {
        println!("\n         // {}", c.name);
        for l in c.code.lines() {
            println!("         {l}");
        }
    }
    if has_dep("glowkit") {
        println!(
            "\n  Note: this project also depends on the `glowkit` crate. With a local `mod {top};`, write\n  `use ::glowkit::...` for the crate and `use crate::{top}::...` for your copies."
        );
    }
    let _ = dir;
}

fn cmd_init(args: &[String]) -> Result<(), String> {
    let a = parse(args, &["template"])?;
    a.check(&["template"])?;
    let name = a
        .pos
        .first()
        .ok_or("usage: glowkit init <name> [--template blank|dashboard|ai-chat|agent-sessions|trading|game]")?;
    let template = a.value("template").unwrap_or("blank");
    templates::init(Path::new(name), template)
}
