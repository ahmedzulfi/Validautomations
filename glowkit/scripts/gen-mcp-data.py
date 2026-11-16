#!/usr/bin/env python3
"""Generate the data the glowkit MCP server embeds (crates/glowkit-mcp/data/).

It combines the gallery's demo registry (`glowkit-gallery --dump-json`) with API information
parsed from the crate sources: for every component type a demo uses, its doc comment, its
constructors, its builder setters, its other public methods, its public fields and the traits
it implements. Every `use` path is then checked by the compiler (a throwaway crate that imports
each path from the local workspace), so the paths agents copy are known to resolve.

usage:
    python3 scripts/gen-mcp-data.py                 # runs the gallery dump itself
    python3 scripts/gen-mcp-data.py demos.json      # or reuse an existing dump
    python3 scripts/gen-mcp-data.py --no-verify     # skip the compile check of `use` paths

Re-run it whenever demos or public APIs change, and commit the result.
"""

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_DIR = os.path.join(ROOT, "crates", "glowkit-mcp", "data")

# crate directory -> (cargo feature of the `glowkit` umbrella crate, module under `glowkit::`)
CRATES = {
    "glowkit-text": ("text", "text"),
    "glowkit-backgrounds": ("backgrounds", "backgrounds"),
    "glowkit-effects": ("effects", "effects"),
    "glowkit-components": ("components", "components"),
    "glowkit-charts": ("charts", "charts"),
    "glowkit-ai": ("ai", "ai"),
    "glowkit-blocks": ("blocks", "blocks"),
    "glowkit-motion": ("motion", "motion"),
    "glowkit-3d": ("three-d", "three_d"),
    "glowkit-canvas": ("canvas", "canvas"),
    "glowkit-templates": ("templates", "templates"),
    "glowkit-interactions": ("interactions", "interactions"),
    "glowkit-agents": ("agents", "agents"),
    "glowkit-stream": ("stream", "stream"),
}

# Categories whose demos are whole pages / apps / sections: served by `get_template`.
TEMPLATE_CATEGORIES = {"Templates", "Dashboards & Shells", "Blocks"}

# Names that the prelude already brings in (or that are egui's), never imported per component.
PRELUDE = {"Theme", "Easing", "Spring", "Trigger", "Self", "Some", "None", "Ok", "Err", "Vec", "String",
           "Option", "Box"}


# ---------------------------------------------------------------------------------------------
# Rust source helpers
# ---------------------------------------------------------------------------------------------

def mask(src):
    """Return `src` with comments, string and char literals replaced by spaces (same length,
    newlines kept), so braces and keywords can be found without false hits."""
    out = list(src)
    i, n = 0, len(src)

    def blank(a, b):
        for k in range(a, b):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        c = src[i]
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            j = src.find("\n", i)
            j = n if j < 0 else j
            blank(i, j)
            i = j
        elif c == "/" and i + 1 < n and src[i + 1] == "*":
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif src.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(i, j)
            i = j
        elif c == "r" and re.match(r'r#*"', src[i:i + 40]) and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")):
            m = re.match(r'r(#*)"', src[i:])
            end = '"' + m.group(1)
            j = src.find(end, i + len(m.group(0)))
            j = n if j < 0 else j + len(end)
            blank(i, j)
            i = j
        elif c == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            blank(i, j + 1)
            i = j + 1
        elif c == "'":
            m = re.match(r"'(\\u\{[0-9a-fA-F]+\}|\\x[0-9a-fA-F]{2}|\\.|[^\\'\n])'", src[i:i + 12])
            if m:
                blank(i, i + len(m.group(0)))
                i += len(m.group(0))
            else:
                i += 1  # a lifetime
        else:
            i += 1
    return "".join(out)


def match_brace(masked, open_idx):
    """Index just past the `}` matching the `{` at `open_idx`."""
    depth = 0
    for k in range(open_idx, len(masked)):
        ch = masked[k]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return k + 1
    return len(masked)


def doc_above(lines, line_idx):
    """The `///` doc comment directly above line `line_idx` (attributes in between are skipped)."""
    docs = []
    k = line_idx - 1
    while k >= 0:
        s = lines[k].strip()
        if s.startswith("///"):
            docs.append(s[3:][1:] if s[3:].startswith(" ") else s[3:])
        elif s.startswith("#[") or s.startswith("#!["):
            pass
        elif s == "" and not docs:
            break
        else:
            break
        k -= 1
    return "\n".join(reversed(docs)).strip()


def first_paragraph(doc):
    """Doc text up to the first code fence, trimmed (keeps it short for agents)."""
    out = []
    for line in doc.splitlines():
        if line.strip().startswith("```"):
            break
        out.append(line)
    return "\n".join(out).strip()


def doc_example(doc):
    m = re.search(r"```[^\n]*\n(.*?)```", doc, re.S)
    if not m:
        return None
    lines = [l for l in m.group(1).splitlines() if not l.strip().startswith("# ") and l.strip() != "#"]
    return "\n".join(lines).strip() or None


def umbrella_paths(code):
    """`glowkit_text::X` -> `glowkit::text::X` (examples in crate docs use the sub-crate names)."""
    if code is None:
        return None
    for crate, (_, mod) in CRATES.items():
        code = code.replace(crate.replace("-", "_") + "::", "glowkit::%s::" % mod)
    return code.replace("glowkit_core::", "glowkit::core::")


def line_in(original, kind, name, fallback):
    m = re.search(r"(?m)^[ \t]*pub %s %s\b" % (kind, name), original)
    return line_of(original, m.start()) if m else fallback


def line_of(text, idx):
    return text.count("\n", 0, idx)


def strip_generics(s):
    """Remove `<...>` groups (nested) from a type header string."""
    out, depth = [], 0
    for ch in s:
        if ch == "<":
            depth += 1
        elif ch == ">" and depth:
            depth -= 1
        elif depth == 0:
            out.append(ch)
    return "".join(out)


def squash(s):
    s = re.sub(r"\s+", " ", s).strip()
    s = s.replace("( ", "(").replace(" )", ")").replace(", )", ")").replace(",)", ")")
    return s


def match_paren(masked, open_idx):
    """Index just past the bracket matching the `(`, `[` or `{` at `open_idx`."""
    pairs = {"(": ")", "[": "]", "{": "}"}
    o = masked[open_idx]
    c = pairs[o]
    depth = 0
    for k in range(open_idx, len(masked)):
        if masked[k] == o:
            depth += 1
        elif masked[k] == c:
            depth -= 1
            if depth == 0:
                return k + 1
    return len(masked)


def collect_macros(src):
    """Single-arm `macro_rules!` definitions: name -> (matcher, body)."""
    m_src = mask(src)
    out = {}
    for m in re.finditer(r"macro_rules!\s+(\w+)\s*\{", m_src):
        outer = m.end() - 1
        end = match_paren(m_src, outer)
        k = m_src.find("(", outer)
        if k < 0 or k > end:
            continue
        pat_end = match_paren(m_src, k)
        arrow = m_src.find("=>", pat_end)
        ob = arrow + 2
        while ob < end and m_src[ob] not in "{([":
            ob += 1
        body_end = match_paren(m_src, ob)
        out[m.group(1)] = (src[k + 1:pat_end - 1].strip(), src[ob + 1:body_end - 1])
    return out


def expand_macros(src, macros):
    """Textually expand invocations of the simple builder macros the crates use:
    `name!()` (a block of setters), `name!(Type)` (impls for a type) and
    `name!(/// doc\n field: Type)` (one documented setter)."""
    if not macros:
        return src
    names = "|".join(re.escape(n) for n in macros)
    for _ in range(3):  # macros that invoke macros
        m_src = mask(src)
        edits = []
        for m in re.finditer(r"(?<![\w$])(%s)!\s*([\(\{\[])" % names, m_src):
            name = m.group(1)
            open_idx = m.end() - 1
            close = match_paren(m_src, open_idx)
            end = close
            if end < len(m_src) and m_src[end] == ";":
                end += 1
            arg = src[open_idx + 1:close - 1]
            matcher, body = macros[name]
            if matcher == "":
                expanded = body
            elif re.fullmatch(r"\$(\w+):(ident|ty)", matcher):
                var = re.fullmatch(r"\$(\w+):(ident|ty)", matcher).group(1)
                expanded = re.sub(r"\$%s\b" % var, arg.strip(), body)
            elif "$name:ident" in matcher and "$ty:ty" in matcher:
                docs = [l.strip() for l in arg.splitlines() if l.strip().startswith("///")]
                rest = "\n".join(l for l in arg.splitlines() if not l.strip().startswith("///")).strip()
                fm = re.match(r"(\w+)\s*:\s*(.+)$", rest, re.S)
                if not fm:
                    continue
                expanded = re.sub(r"\$\(#\[\$\w+\]\)\*", "\n".join(docs), body)
                expanded = expanded.replace("$name", fm.group(1)).replace("$ty", fm.group(2).strip())
            else:
                continue
            edits.append((m.start(), end, expanded))
        if not edits:
            break
        for a, b, text in reversed(edits):
            src = src[:a] + text + src[b:]
    return src


class CrateIndex:
    """Public structs/enums, inherent impl methods, trait impls and top-level fns of one crate."""

    def __init__(self, crate_dir):
        self.crate_dir = crate_dir
        self.src_dir = os.path.join(ROOT, "crates", crate_dir, "src")
        self.types = {}  # name -> info
        self.impls = defaultdict(list)  # type name -> [method]
        self.traits = defaultdict(set)  # type name -> {trait}
        self.fns = {}  # top-level pub fn name -> info
        paths = []
        for dirpath, _, files in os.walk(self.src_dir):
            for f in sorted(files):
                if f.endswith(".rs"):
                    paths.append(os.path.join(dirpath, f))
        self.macros = {}
        for p in paths:
            self.macros.update(collect_macros(open(p, encoding="utf-8").read()))
        for p in paths:
            self._scan(p)

    def module_path(self, path):
        rel = os.path.relpath(path, self.src_dir)[:-3].split(os.sep)
        if rel[-1] in ("lib", "mod"):
            rel = rel[:-1]
        return rel

    def _scan(self, path):
        original = open(path, encoding="utf-8").read()
        src = expand_macros(original, self.macros)
        # drop the macro definitions themselves (their `$T` impls are not real items)
        for mm in reversed(list(re.finditer(r"macro_rules!\s+\w+\s*\{", mask(src)))):
            end = match_paren(mask(src), mm.end() - 1)
            src = src[:mm.start()] + "\n" * src.count("\n", mm.start(), end) + src[end:]
        m_src = mask(src)
        lines = src.splitlines()
        is_demo = os.path.basename(path).startswith("demo") or "demos" in path
        for m in re.finditer(r"(?m)^[ \t]*pub (struct|enum) (\w+)", m_src):
            kind, name = m.group(1), m.group(2)
            ln = line_of(src, m.start())
            info = {
                "kind": kind,
                "file": os.path.relpath(path, ROOT),
                "line": line_in(original, kind, name, ln) + 1,
                "doc": doc_above(lines, ln),
                "module": self.module_path(path),
                "fields": [],
                "variants": [],
                "demo_file": is_demo,
            }
            # body
            rest = m_src[m.end():]
            brace = re.match(r"[^{;(]*", rest)
            end_pos = m.end() + brace.end()
            if end_pos < len(m_src) and m_src[end_pos] == "{":
                close = match_brace(m_src, end_pos)
                body_masked = m_src[end_pos + 1:close - 1]
                body = src[end_pos + 1:close - 1]
                base_line = line_of(src, end_pos + 1)
                body_lines = body.splitlines()
                depth = 0
                for i, bl in enumerate(body_masked.splitlines()):
                    if depth == 0:
                        if kind == "struct":
                            fm = re.match(r"\s*pub (\w+)\s*:\s*(.+?),?\s*$", bl)
                            if fm:
                                info["fields"].append({
                                    "name": fm.group(1),
                                    "type": body_lines[i].split(":", 1)[1].strip().rstrip(","),
                                    "doc": doc_above(lines, base_line + i),
                                })
                        else:
                            vm = re.match(r"\s*([A-Z]\w*)\b", bl)
                            if vm:
                                info["variants"].append(squash(body_lines[i].strip().rstrip(",")))
                    depth += bl.count("{") + bl.count("(") - bl.count("}") - bl.count(")")
            prev = self.types.get(name)
            if prev is None or (prev["demo_file"] and not is_demo):
                self.types[name] = info
        for m in re.finditer(r"(?m)^pub (?:const )?fn (\w+)", m_src):
            ln = line_of(src, m.start())
            sig = self._signature(src, m_src, m.start())
            if not is_demo and m.group(1) != "demos":
                self.fns[m.group(1)] = {"sig": sig, "doc": first_paragraph(doc_above(lines, ln)),
                                       "module": self.module_path(path)}
        for m in re.finditer(r"(?m)^[ \t]*(?:unsafe )?impl\b", m_src):
            ob = m_src.find("{", m.end())
            semi = m_src.find(";", m.end())
            if ob < 0 or (0 <= semi < ob):
                continue
            header = strip_generics(m_src[m.end():ob])
            header = re.sub(r"\bwhere\b.*", "", header, flags=re.S).strip()
            trait = None
            if re.search(r"\sfor\s", " " + header + " "):
                trait, self_ty = re.split(r"\s+for\s+", header, maxsplit=1)
                trait = trait.strip().split("::")[-1]
            else:
                self_ty = header
            self_ty = self_ty.strip().lstrip("&").strip()
            self_ty = re.sub(r"^(mut|dyn)\s+", "", self_ty).split("::")[-1].strip()
            if not re.fullmatch(r"\w+", self_ty or ""):
                continue
            close = match_brace(m_src, ob)
            if trait:
                self.traits[self_ty].add(trait)
                continue
            self._methods(src, m_src, lines, ob, close, self_ty)

    def _signature(self, src, m_src, start):
        depth, k = 0, start
        while k < len(m_src):
            ch = m_src[k]
            if ch in "(<[":
                depth += 1
            elif ch in ")>]":
                # `->` is not a closing bracket
                if not (ch == ">" and k > 0 and m_src[k - 1] == "-"):
                    depth -= 1
            elif ch in "{;" and depth <= 0:
                break
            k += 1
        sig = squash(src[start:k])
        return re.sub(r"\s+where\s*$", "", sig)

    def _methods(self, src, m_src, lines, ob, close, self_ty):
        depth = 0
        k = ob + 1
        body = m_src[:close - 1]
        while k < close - 1:
            ch = body[k]
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
            elif depth == 0 and body.startswith("pub", k) and (k == 0 or not body[k - 1].isalnum()):
                mm = re.match(r"pub (?:const )?(?:async )?(?:unsafe )?fn (\w+)", body[k:])
                if mm:
                    sig = self._signature(src, m_src, k)
                    ln = line_of(src, k)
                    self.impls[self_ty].append({"name": mm.group(1), "sig": sig,
                                                "doc": first_paragraph(doc_above(lines, ln))})
                    k += len(mm.group(0))
                    continue
            k += 1


def classify(type_name, methods):
    ctors, setters, others = [], [], []
    for meth in methods:
        sig = meth["sig"]
        params = sig[sig.find("(") + 1:]
        ret = sig.split("->", 1)[1].strip() if "->" in sig else ""
        ret_base = strip_generics(ret).strip()
        returns_self = ret_base in ("Self", type_name)
        recv = re.match(r"\s*(&\s*'?\w*\s*mut\s+self|&\s*'?\w*\s*self|mut\s+self|self)\b", params)
        entry = {"name": meth["name"], "sig": sig, "doc": meth["doc"]}
        if not recv and returns_self:
            ctors.append(entry)
        elif recv and recv.group(1).replace(" ", "") in ("self", "mutself") and returns_self:
            setters.append(entry)
        else:
            others.append(entry)
    return ctors, setters, others


# ---------------------------------------------------------------------------------------------
# Demos -> crates
# ---------------------------------------------------------------------------------------------

def demo_locations():
    """demo id -> crate dir, by finding `id: "<id>"` in each crate's sources."""
    loc, fallback = {}, {}
    for crate in CRATES:
        src_dir = os.path.join(ROOT, "crates", crate, "src")
        for dirpath, _, files in os.walk(src_dir):
            for f in files:
                if not f.endswith(".rs"):
                    continue
                text = open(os.path.join(dirpath, f), encoding="utf-8").read()
                for m in re.finditer(r'\bid:\s*"([a-z0-9][a-z0-9-]*)"', text):
                    loc.setdefault(m.group(1), crate)
                if "demo" in f:
                    # ids passed positionally (e.g. to a demo-building helper)
                    for m in re.finditer(r'"([a-z0-9]+(?:-[a-z0-9]+)+)"', text):
                        fallback.setdefault(m.group(1), crate)
    for k, v in fallback.items():
        loc.setdefault(k, v)
    return loc


def idents_in(code):
    """CamelCase identifiers that look like paths/constructors in a snippet, in order."""
    masked = mask(code)
    seen, out = set(), []
    for m in re.finditer(r"(?<![\w:])([A-Z][A-Za-z0-9]+)\b", masked):
        name = m.group(1)
        if name not in seen:
            seen.add(name)
            out.append(name)
    return out


def split_params(sig):
    """Parameter list of a signature as [(name, type)], receiver excluded."""
    a = sig.find("(")
    if a < 0:
        return []
    close = match_paren(sig, a)
    inner = sig[a + 1:close - 1]
    parts, depth, cur = [], 0, ""
    for ch in inner:
        if ch in "(<[":
            depth += 1
        elif ch in ")>]":
            depth -= 1
        if ch == "," and depth == 0:
            parts.append(cur)
            cur = ""
        else:
            cur += ch
    if cur.strip():
        parts.append(cur)
    out = []
    for p in parts:
        p = p.strip()
        if re.fullmatch(r"&?\s*('\w+\s+)?(mut\s+)?self", p):
            continue
        if ":" in p:
            n, t = p.split(":", 1)
            out.append((n.strip(), t.strip()))
    return out


def owned_type(t):
    """`&'a mut Vec<X>` -> `Vec<X>`; `impl Into<String>` -> `String`; None when not nameable."""
    t = re.sub(r"^&\s*('\w+\s+)?mut\s+", "", t.strip())
    m = re.fullmatch(r"impl Into<(.+)>", t)
    if m:
        t = m.group(1)
    if t.startswith("&") or t.startswith("impl ") or "dyn " in t:
        return None
    return re.sub(r"<'\w+>", "", t).replace("'a ", "")


def infer_rhs_type(rhs):
    rhs = rhs.strip()
    m = re.match(r"([A-Z]\w*)::(new|default|with_\w+|from\w*)\b", rhs)
    if m and m.group(1) not in ("String", "Vec"):
        return m.group(1)
    if re.match(r"String::|\"[^\"]*\"\s*\.(into|to_string|to_owned)\(\)$", rhs):
        return "String"
    if rhs in ("true", "false"):
        return "bool"
    if re.fullmatch(r"-?\d+\.\d*(f32)?", rhs):
        return "f32"
    if re.fullmatch(r"\d+usize", rhs):
        return "usize"
    m = re.match(r"vec!\[\s*([A-Z]\w*)::", rhs)
    if m:
        return "Vec<%s>" % m.group(1)
    return None


def infer_state(code, apis):
    """Variables a snippet keeps across frames: `let mut x = ..;` declarations and variables
    passed as `&mut x` (typed from the receiving parameter's signature)."""
    masked = mask(code)
    state = {}
    order = []
    for m in re.finditer(r"(?m)^let mut (\w+)(\s*:\s*([^=]+))?\s*=\s*", masked):
        end = m.end()
        depth = 0
        while end < len(masked):
            ch = masked[end]
            if ch in "([{":
                depth += 1
            elif ch in ")]}":
                depth -= 1
            elif ch == ";" and depth == 0:
                break
            end += 1
        rhs = code[m.end():end]
        ty = m.group(3).strip() if m.group(3) else infer_rhs_type(rhs)
        state[m.group(1)] = {"name": m.group(1), "type": ty, "init": rhs.strip(), "decl": code[m.start():end + 1]}
        order.append(m.group(1))
    for m in re.finditer(r"&mut\s+([a-z_]\w*)\b(?!\s*[\.\[(])", masked):
        var = m.group(1)
        if var in ("self", "ui") or (var in state and state[var]["type"]):
            continue
        # innermost open paren before the argument, and the argument's index
        depth, k, idx = 0, m.start() - 1, 0
        while k >= 0:
            ch = masked[k]
            if ch in ")]}":
                depth += 1
            elif ch in "([{":
                if depth == 0:
                    break
                depth -= 1
            elif ch == "," and depth == 0:
                idx += 1
            k -= 1
        if k < 0 or masked[k] != "(":
            continue
        cm = re.search(r"(?:([A-Z]\w*)::|\.)\s*(\w+)\s*$", masked[:k])
        if not cm:
            continue
        ty_name, fn = cm.group(1), cm.group(2)
        sig = None
        for api in apis:
            if ty_name and api["name"] != ty_name:
                continue
            for meth in api["constructors"] + api["setters"] + api["methods"]:
                if meth["name"] == fn:
                    sig = meth["sig"]
                    break
            if sig:
                break
        params = split_params(sig) if sig else []
        if idx < len(params):
            ty = owned_type(params[idx][1])
            if ty:
                if var in state:
                    state[var]["type"] = ty
                else:
                    state[var] = {"name": var, "type": ty, "init": None, "decl": None}
                    order.append(var)
    return [state[v] for v in order]


def fn_calls_in(code):
    masked = mask(code)
    out = []
    for m in re.finditer(r"(?<![\.\w:])([a-z_][a-z0-9_]*)\s*\(", masked):
        if m.group(1) not in out:
            out.append(m.group(1))
    return out


TOKEN_RE = re.compile(r"[a-z0-9]+")


def tokens(s):
    return set(t for t in TOKEN_RE.findall(s.lower()) if len(t) > 2)


# ---------------------------------------------------------------------------------------------
# `use` path verification
# ---------------------------------------------------------------------------------------------

def check_paths(paths):
    """Compile a crate importing every path; return the set of paths that resolve."""
    if not paths:
        return set()
    tmp = tempfile.mkdtemp(prefix="glowkit-mcp-paths-")
    try:
        os.makedirs(os.path.join(tmp, "src"))
        with open(os.path.join(tmp, "Cargo.toml"), "w") as f:
            f.write('[package]\nname = "pathcheck"\nversion = "0.0.0"\nedition = "2021"\n\n[workspace]\n\n'
                    '[dependencies]\nglowkit = { path = "%s" }\n' % os.path.join(ROOT, "crates", "glowkit"))
        lines = ["#![allow(unused_imports)]"]
        line_to_path = {}
        for i, p in enumerate(paths):
            lines.append("mod m%d { use %s; }" % (i, p))
            line_to_path[len(lines)] = p
        lines.append("fn main() {}")
        with open(os.path.join(tmp, "src", "main.rs"), "w") as f:
            f.write("\n".join(lines) + "\n")
        env = dict(os.environ)
        env["CARGO_TARGET_DIR"] = os.path.join(ROOT, "target", "mcp-pathcheck")
        # reuse the workspace lockfile so no new versions get resolved
        shutil.copy(os.path.join(ROOT, "Cargo.lock"), os.path.join(tmp, "Cargo.lock"))
        res = subprocess.run(["cargo", "check", "--quiet", "--message-format=json"], cwd=tmp, env=env,
                             capture_output=True, text=True)
        bad = set()
        for line in res.stdout.splitlines():
            try:
                msg = json.loads(line)
            except json.JSONDecodeError:
                continue
            if msg.get("reason") != "compiler-message" or msg["message"]["level"] != "error":
                continue
            for span in msg["message"]["spans"]:
                if span["file_name"].endswith("main.rs") and span["line_start"] in line_to_path:
                    bad.add(line_to_path[span["line_start"]])
        if res.returncode != 0 and not bad:
            sys.stderr.write(res.stderr[-3000:])
            raise SystemExit("path check failed to compile for an unexpected reason")
        return set(paths) - bad
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


# ---------------------------------------------------------------------------------------------
# Theme
# ---------------------------------------------------------------------------------------------

def theme_data(core):
    path = os.path.join(ROOT, "crates", "glowkit-core", "src", "theme.rs")
    src = open(path, encoding="utf-8").read()
    info = core.types["Theme"]
    ctors, setters, others = classify("Theme", core.impls["Theme"])
    presets = []
    values = {}
    for m in re.finditer(r"pub fn (\w+)\(\) -> Self \{\s*Self \{(.*?)\n        \}", src, re.S):
        fn, body = m.group(1), m.group(2)
        vals = {}
        base = re.search(r"\.\.Self::(\w+)\(\)", body)
        if base and base.group(1) in values:
            vals.update(values[base.group(1)])
        for fm in re.finditer(r"(\w+):\s*(.+?),\n", body + ",\n"):
            v = fm.group(2).strip().rstrip(",")
            hexes = re.findall(r'hex\("(#[0-9A-Fa-f]{6,8})"\)', v)
            if hexes:
                vals[fm.group(1)] = hexes if v.startswith("[") else hexes[0]
            else:
                vals[fm.group(1)] = v.strip('"')
        values[fn] = vals
        ln = line_of(src, m.start())
        presets.append({"fn": fn, "doc": doc_above(src.splitlines(), ln), "values": vals})
    return {
        "doc": info["doc"],
        "fields": info["fields"],
        "presets": presets,
        "methods": [{"sig": c["sig"], "doc": c["doc"]} for c in ctors + setters + others],
    }


# ---------------------------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------------------------

def load_demos(argv):
    files = [a for a in argv if not a.startswith("--")]
    if files:
        return json.load(open(files[0], encoding="utf-8"))
    out = subprocess.run(["cargo", "run", "--quiet", "-p", "glowkit-gallery", "--", "--dump-json"],
                         cwd=ROOT, capture_output=True, text=True, check=True).stdout
    return json.loads(out)


def main(argv):
    demos = load_demos(argv)
    verify = "--no-verify" not in argv
    loc = demo_locations()
    indexes = {c: CrateIndex(c) for c in CRATES}
    core = CrateIndex("glowkit-core")

    def find_type(name, prefer):
        if name in indexes[prefer].types:
            return prefer
        for c, idx in indexes.items():
            if name in idx.types and not idx.types[name]["demo_file"]:
                return c
        return None

    types = {}  # key "crate::Name" -> api
    components = []
    path_candidates = {}  # item key -> [candidate use paths]

    def candidates(crate, module, name):
        mod = CRATES[crate][1]
        cands = ["glowkit::%s::%s" % (mod, name)]
        for k in range(len(module), 0, -1):
            cands.append("glowkit::%s::%s::%s" % (mod, "::".join(module[:k]), name))
        return cands

    missing = []
    for d in demos:
        crate = loc.get(d["id"])
        if crate is None:
            missing.append(d["id"])
            continue
        feature, mod = CRATES[crate]
        main_type, imports = None, []
        for name in idents_in(d["code"]):
            if name in PRELUDE:
                continue
            c = find_type(name, crate)
            if c is None:
                continue
            key = "%s::%s" % (c, name)
            imports.append(key)
            if main_type is None and indexes[c].types[name]["kind"] == "struct":
                main_type = key
            if key not in types:
                idx = indexes[c]
                info = idx.types[name]
                ctors, setters, others = classify(name, idx.impls.get(name, []))
                traits = sorted(idx.traits.get(name, set()))
                types[key] = {
                    "name": name,
                    "crate": c,
                    "kind": info["kind"],
                    "doc": first_paragraph(info["doc"]),
                    "example": umbrella_paths(doc_example(info["doc"])),
                    "source": "%s:%d" % (info["file"], info["line"]),
                    "constructors": ctors,
                    "setters": setters,
                    "methods": others,
                    "fields": info["fields"],
                    "variants": info["variants"],
                    "traits": traits,
                    "widget": "Widget" in traits,
                }
                path_candidates[key] = candidates(c, info["module"], name)
        for fn in fn_calls_in(d["code"]):
            if fn in indexes[crate].fns:
                key = "%s::fn::%s" % (crate, fn)
                imports.append(key)
                if key not in path_candidates:
                    path_candidates[key] = candidates(crate, indexes[crate].fns[fn]["module"], fn)
        state = infer_state(d["code"], [types[k] for k in imports if k in types])
        for st in state:
            for name in idents_in(st["type"] or ""):
                c = find_type(name, crate)
                key = "%s::%s" % (c, name)
                if c and key not in imports:
                    imports.append(key)
                    if key not in path_candidates:
                        info = indexes[c].types[name]
                        path_candidates[key] = candidates(c, info["module"], name)
        if main_type is None:
            for k in imports:
                if "::fn::" not in k:
                    main_type = k
                    break
        components.append({
            "id": d["id"],
            "name": d["name"],
            "category": d["category"],
            "description": d["description"],
            "covers": d["covers"],
            "code": d["code"],
            "showcase": d["showcase"],
            "crate": crate,
            "feature": feature,
            "module": "glowkit::" + mod,
            "type": main_type,
            "imports": imports,
            "template": d["category"] in TEMPLATE_CATEGORIES,
            "state": state,
        })

    if missing:
        sys.stderr.write("warning: no source location for %d demos: %s\n" % (len(missing), ", ".join(missing[:10])))

    # Resolve every `use` path with the compiler.
    resolved = {}
    if verify:
        pending = dict(path_candidates)
        level = 0
        while pending:
            trial = {k: c[level] for k, c in pending.items() if level < len(c)}
            if not trial:
                break
            ok = check_paths(sorted(set(trial.values())))
            for k, p in trial.items():
                if p in ok:
                    resolved[k] = p
            pending = {k: c for k, c in pending.items() if k not in resolved}
            level += 1
        if pending:
            sys.stderr.write("warning: %d items have no public path: %s\n" % (len(pending), ", ".join(sorted(pending)[:10])))
    else:
        resolved = {k: c[0] for k, c in path_candidates.items()}

    for key, t in types.items():
        t["path"] = resolved.get(key)
        t["path_verified"] = verify and key in resolved
    for comp in components:
        paths = []
        for k in comp["imports"]:
            p = resolved.get(k)
            if p and p not in paths:
                paths.append(p)
        comp["use"] = paths
        del comp["imports"]

    # Related components: same category, most shared words.
    toks = {c["id"]: tokens(c["name"] + " " + c["description"] + " " + " ".join(c["covers"])) for c in components}
    by_cat = defaultdict(list)
    for c in components:
        by_cat[c["category"]].append(c)
    for c in components:
        scored = []
        for o in by_cat[c["category"]]:
            if o["id"] == c["id"]:
                continue
            a, b = toks[c["id"]], toks[o["id"]]
            score = len(a & b) / (len(a | b) or 1)
            if o["type"] and o["type"] == c["type"]:
                score += 0.5
            scored.append((score, o["showcase"], o["id"]))
        scored.sort(key=lambda s: (-s[0], not s[1], s[2]))
        c["related"] = [s[2] for s in scored[:6]]

    categories = []
    seen = []
    for c in components:
        if c["category"] not in seen:
            seen.append(c["category"])
    for cat in seen:
        items = by_cat[cat]
        crates = sorted(set(i["crate"] for i in items))
        categories.append({
            "name": cat,
            "count": len(items),
            "crates": crates,
            "features": sorted(set(CRATES[c][0] for c in crates)),
            "examples": [i["name"] for i in items if i["showcase"]][:8],
        })

    data = {
        "glowkit_version": "0.1",
        "egui_version": "0.37",
        "features": {c: {"feature": f, "module": "glowkit::" + m} for c, (f, m) in CRATES.items()},
        "categories": categories,
        "components": components,
        "types": types,
        "theme": theme_data(core),
    }
    os.makedirs(OUT_DIR, exist_ok=True)
    with open(os.path.join(OUT_DIR, "components.json"), "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, separators=(",", ":"))
        f.write("\n")
    shutil.copy(os.path.join(ROOT, "crates", "glowkit", "docs", "CUSTOMIZING.md"), os.path.join(OUT_DIR, "CUSTOMIZING.md"))
    shutil.copy(os.path.join(ROOT, "CONTRIBUTING.md"), os.path.join(OUT_DIR, "CONVENTIONS.md"))
    n_typed = sum(1 for c in components if c["type"])
    n_verified = sum(1 for t in types.values() if t["path_verified"])
    print("wrote %d components (%d with a main type), %d types (%d use paths compiler-verified), %d categories"
          % (len(components), n_typed, len(types), n_verified, len(categories)))


if __name__ == "__main__":
    main(sys.argv[1:])
