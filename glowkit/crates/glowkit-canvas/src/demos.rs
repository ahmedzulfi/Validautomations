use crate::*;
use egui::{pos2, vec2, Color32, CornerRadius, Rect, Ui};
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::demo::{Category::Canvas as C, Demo};
use glowkit_core::{noise, paint, Theme};
use std::sync::{Arc, Mutex};

/// Demo-local persistent state (the demos are plain `fn(&mut Ui)`).
fn persist<T: Send + 'static>(ui: &Ui, key: &str, init: impl FnOnce() -> T) -> Arc<Mutex<T>> {
    let id = ui.scope_id().with(key);
    ui.data_mut(|d| d.get_temp_mut_or_insert_with(id, || Arc::new(Mutex::new(init()))).clone())
}

const H: f32 = 480.0;

/// A row of selectable chips above a demo; returns the newly picked index.
fn chips(ui: &mut Ui, label: &str, items: &[&str], current: usize) -> usize {
    let t = Theme::of(ui);
    let mut out = current;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).size(12.0).color(t.muted_foreground));
        for (i, it) in items.iter().enumerate() {
            if ui.selectable_label(i == current, egui::RichText::new(*it).size(12.0)).clicked() {
                out = i;
            }
        }
    });
    out
}

// ------------------------------------------------------------------------------------------------
// Node editor graphs
// ------------------------------------------------------------------------------------------------

fn workflow_graph(t: &Theme) -> Graph {
    let mut g = Graph::new();
    let c = t.chart;
    let hook = g.add(Node::new("Webhook", pos2(0.0, 120.0)).subtitle("POST /orders").icon("⚡").color(c[3]).output(Port::new("body")).line("Last call 2s ago").status(NodeStatus::Done));
    let filter = g.add(Node::new("If", pos2(280.0, 120.0)).subtitle("amount > $500").icon("?").color(c[1]).input(Port::new("in")).output(Port::new("true")).output(Port::new("false")).status(NodeStatus::Done));
    let enrich = g.add(Node::new("HTTP Request", pos2(560.0, 0.0)).subtitle("GET /customers/:id").icon("↗").color(c[5]).input(Port::new("in")).output(Port::new("json")).line("Retry 3x · 10s timeout").status(NodeStatus::Running));
    let ai = g.add(Node::new("AI Summarize", pos2(840.0, 0.0)).subtitle("Model · temperature 0.2").icon("✦").color(c[0]).input(Port::new("order")).input(Port::new("customer")).output(Port::new("summary")).line("\"Write a 2-line brief…\""));
    let notify = g.add(Node::new("Notify #sales", pos2(1120.0, -40.0)).subtitle("Chat message").icon("#").color(c[2]).input(Port::new("text")).output(Port::new("ok")));
    let db = g.add(Node::new("Insert row", pos2(1120.0, 140.0)).subtitle("orders_large").icon("▤").color(c[4]).input(Port::new("row")).output(Port::new("id")));
    let mail = g.add(Node::new("Send receipt", pos2(560.0, 260.0)).subtitle("Email · template #12").icon("@").color(c[3]).input(Port::new("to")).output(Port::new("sent")));
    g.link((hook, 0), (filter, 0)).anim = EdgeAnim::Dashes;
    g.link((filter, 0), (enrich, 0)).label("true").anim = EdgeAnim::Dashes;
    g.link((filter, 1), (mail, 0)).label("false");
    g.link((enrich, 0), (ai, 1));
    g.link((filter, 0), (ai, 0));
    g.link((ai, 0), (notify, 0));
    g.link((ai, 0), (db, 0));
    g
}

const FLOAT: Color32 = hex("#84CC16");
const VEC2: Color32 = hex("#22D3EE");
const COLOR: Color32 = hex("#F472B6");

fn shader_graph() -> Graph {
    let mut g = Graph::new();
    let f = |l: &str| Port::typed(l, "float", FLOAT);
    let v2 = |l: &str| Port::typed(l, "vec2", VEC2);
    let col = |l: &str| Port::typed(l, "color", COLOR);
    let uv = g.add(Node::new("UV", pos2(0.0, 0.0)).subtitle("Input").icon("◫").color(VEC2).width(170.0).output(v2("uv")));
    let time = g.add(Node::new("Time", pos2(0.0, 150.0)).subtitle("Input").icon("◷").color(FLOAT).width(170.0).output(f("t")).output(f("sin t")));
    let noise = g.add(Node::new("Noise", pos2(240.0, -20.0)).subtitle("Fractal · 4 octaves").icon("≈").color(FLOAT).width(190.0).input(v2("uv")).input(f("scale")).input(f("offset")).output(f("value")).custom(84.0).tag("noise"));
    let ramp = g.add(Node::new("Color Ramp", pos2(500.0, 10.0)).subtitle("3 stops").icon("▥").color(COLOR).width(190.0).input(f("factor")).output(col("color")).custom(26.0).tag("ramp"));
    let fres = g.add(Node::new("Fresnel", pos2(500.0, 230.0)).subtitle("IOR 1.45").icon("◐").color(FLOAT).width(190.0).input(f("ior")).output(f("fac")));
    let mix_n = g.add(Node::new("Mix", pos2(760.0, 60.0)).subtitle("Multiply").icon("×").color(COLOR).width(180.0).input(col("a")).input(f("b")).output(col("result")));
    let out = g.add(Node::new("Material Output", pos2(1010.0, 40.0)).subtitle("Surface").icon("◉").color(hex("#A78BFA")).width(210.0).input(col("base color")).input(f("roughness")).input(col("emission")).custom(96.0).tag("output"));
    g.link((uv, 0), (noise, 0));
    g.link((time, 0), (noise, 2)).anim = EdgeAnim::Dots;
    g.link((noise, 0), (ramp, 0));
    g.link((ramp, 0), (mix_n, 0));
    g.link((fres, 0), (mix_n, 1));
    g.link((mix_n, 0), (out, 0));
    g.link((time, 1), (out, 1));
    g
}

fn shader_body(p: &egui::Painter, r: Rect, zoom: f32, n: &Node, time: f32) {
    let rr = CornerRadius::same((6.0 * zoom) as u8);
    let stops = [hex("#1E1B4B"), hex("#DB2777"), hex("#FDE68A")];
    let field = |uv: egui::Vec2| noise::fbm3(uv.x * 3.0, uv.y * 3.0, time * 0.25, 4) * 0.5 + 0.5;
    match n.tag.as_str() {
        "noise" => {
            paint::shader_grid(p, r, 28, 12, |uv| {
                let v = field(uv).clamp(0.0, 1.0);
                Color32::from_gray((v * 255.0) as u8)
            });
            p.rect_stroke(r, rr, egui::Stroke::new(1.0, with_alpha(Color32::WHITE, 0.08)), egui::StrokeKind::Inside);
        }
        "ramp" => {
            paint::gradient_rect(p, r, 5.0 * zoom, 0.0, &stops);
        }
        "output" => {
            // a lit sphere preview
            let c = r.center();
            let rad = r.height() * 0.46;
            paint::glow(p, c, rad * 1.5, with_alpha(stops[1], 0.25));
            let outline = paint::circle_points(c, rad, 48);
            paint::fill_convex_fn(p, &outline, 10, |q| {
                let d = (q - c) / rad;
                let z = (1.0 - d.length_sq()).max(0.0).sqrt();
                let v = (noise::fbm3(d.x * 2.0 + 4.0, d.y * 2.0, time * 0.25, 3) * 0.5 + 0.5).clamp(0.0, 1.0);
                let base = glowkit_core::color::gradient(&stops, v);
                let light = (0.35 + 0.65 * (d.x * -0.4 + d.y * -0.5 + z * 0.75).max(0.0)).min(1.0);
                let rim = (1.0 - z).powi(3) * 0.8;
                mix(Color32::BLACK, base, light).lerp_to_gamma(stops[2], rim)
            });
        }
        _ => {}
    }
}

fn agent_graph(t: &Theme) -> Graph {
    let mut g = Graph::new();
    let c = t.chart;
    let q = g.add(Node::new("User query", pos2(0.0, 80.0)).subtitle("chat input").icon("›").color(c[5]).width(180.0).output(Port::new("text")).line("\"Compare Q3 churn by region\""));
    let plan = g.add(Node::new("Planner", pos2(250.0, 80.0)).subtitle("LLM · tool calling").icon("✦").color(c[0]).input(Port::new("goal")).output(Port::new("search")).output(Port::new("sql")).custom(14.0).tag("agent"));
    let rag = g.add(Node::new("Retriever", pos2(530.0, -40.0)).subtitle("vector search · k=8").icon("⌕").color(c[1]).input(Port::new("query")).output(Port::new("chunks")).custom(14.0).tag("agent"));
    let sql = g.add(Node::new("SQL tool", pos2(530.0, 170.0)).subtitle("warehouse · read-only").icon("▤").color(c[4]).input(Port::new("query")).output(Port::new("rows")).custom(14.0).tag("agent"));
    let rerank = g.add(Node::new("Reranker", pos2(800.0, -40.0)).subtitle("cross-encoder").icon("⇅").color(c[1]).width(180.0).input(Port::new("chunks")).output(Port::new("top 3")).custom(14.0).tag("agent"));
    let writer = g.add(Node::new("Writer", pos2(1040.0, 70.0)).subtitle("LLM · streaming").icon("✎").color(c[0]).input(Port::new("context").multi()).output(Port::new("draft")).custom(14.0).tag("agent"));
    let guard = g.add(Node::new("Guardrails", pos2(1300.0, 70.0)).subtitle("PII · policy").icon("⛨").color(c[2]).width(180.0).input(Port::new("draft")).output(Port::new("answer")).custom(14.0).tag("agent"));
    for (a, ap, b, bp) in [(q, 0, plan, 0), (plan, 0, rag, 0), (plan, 1, sql, 0), (rag, 0, rerank, 0), (rerank, 0, writer, 0), (sql, 0, writer, 0), (writer, 0, guard, 0)] {
        g.link((a, ap), (b, bp)).anim = EdgeAnim::Dots;
    }
    g
}

/// Agent pipeline stages light up one after another.
fn agent_tick(g: &mut Graph, time: f32) {
    let stage = |title: &str| match title {
        "Planner" => Some(0.0),
        "Retriever" | "SQL tool" => Some(1.0),
        "Reranker" => Some(2.0),
        "Writer" => Some(3.0),
        "Guardrails" => Some(4.0),
        _ => None,
    };
    let k = (time / 1.3) % 7.0;
    for n in &mut g.nodes {
        if let Some(i) = stage(&n.title) {
            n.status = if k < i { NodeStatus::None } else if k < i + 1.0 { NodeStatus::Running } else { NodeStatus::Done };
        }
    }
}

fn agent_body(p: &egui::Painter, r: Rect, zoom: f32, n: &Node, time: f32, t: &Theme) {
    let col = n.color.unwrap_or(t.accent);
    let bar = Rect::from_min_size(r.min + vec2(0.0, r.height() * 0.5 - 2.5 * zoom), vec2(r.width(), 5.0 * zoom));
    p.rect_filled(bar, CornerRadius::same(3), with_alpha(t.foreground, 0.08));
    let k = match n.status {
        NodeStatus::Done => 1.0,
        NodeStatus::Running => (time * 0.9).fract(),
        _ => 0.0,
    };
    if k > 0.0 {
        let mut fill = bar;
        fill.set_width(bar.width() * k);
        p.rect_filled(fill, CornerRadius::same(3), col);
    }
}

fn flow_palette(t: &Theme) -> Vec<PaletteItem> {
    let c = t.chart;
    vec![
        PaletteItem { template: Node::shape(NodeShape::Terminator, "Start / End", Pos2::ZERO).color(c[4]) },
        PaletteItem { template: Node::shape(NodeShape::Process, "Process", Pos2::ZERO).color(c[0]) },
        PaletteItem { template: { let mut n = Node::shape(NodeShape::Decision, "Decision", Pos2::ZERO).color(c[3]); n.outputs = vec![Port::new("yes"), Port::new("no")]; n } },
        PaletteItem { template: Node::shape(NodeShape::Data, "Input / Output", Pos2::ZERO).color(c[1]) },
        PaletteItem { template: Node::shape(NodeShape::Document, "Document", Pos2::ZERO).color(c[5]) },
        PaletteItem { template: Node::shape(NodeShape::Circle, "Event", Pos2::ZERO).color(c[2]) },
    ]
}

use egui::Pos2;

fn flowchart_graph(t: &Theme) -> Graph {
    let pal = flow_palette(t);
    let mk = |i: usize, title: &str, x: f32, y: f32| {
        let mut n = pal[i].template.clone();
        n.title = title.into();
        n.pos = pos2(x - n.width / 2.0, y);
        n
    };
    let mut g = Graph::new();
    let start = g.add(mk(0, "Ticket opened", 300.0, 0.0));
    let read = g.add(mk(3, "Read ticket", 300.0, 110.0));
    let dec = g.add(mk(2, "Known issue?", 300.0, 220.0));
    let kb = g.add(mk(4, "Send KB article", 120.0, 380.0));
    let esc = g.add(mk(1, "Escalate to L2", 480.0, 380.0));
    let end = g.add(mk(0, "Resolved", 300.0, 520.0));
    g.link((start, 0), (read, 0));
    g.link((read, 0), (dec, 0));
    g.link((dec, 0), (kb, 0));
    g.link((dec, 1), (esc, 0)).anim = EdgeAnim::Dashes;
    g.link((kb, 0), (end, 0));
    g.link((esc, 0), (end, 0));
    g
}

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "node-editor",
            name: "Node Editor",
            category: C,
            description: "A workflow-automation graph: drag nodes, connect handles with a live bezier preview, box-select, minimap, controls, undo/redo, copy/paste and auto-layout.",
            covers: &["React Flow: Node-based UI", "xyflow: Workflow editor", "React Flow: MiniMap", "React Flow: Controls", "React Flow: Node Toolbar", "React Flow: Edge types", "React Flow: Animated edges", "n8n: Workflow canvas"],
            code: r##"let mut graph = Graph::new();
let hook = graph.add(Node::new("Webhook", pos2(0.0, 0.0)).subtitle("POST /orders").output(Port::new("body")));
let filter = graph.add(Node::new("If", pos2(280.0, 0.0)).input(Port::new("in")).output(Port::new("true")).output(Port::new("false")));
graph.link((hook, 0), (filter, 0)).anim = EdgeAnim::Dashes;

// every frame (state + graph are yours):
NodeEditor::new(&mut graph, &mut state)
    .background(Background::Dots)
    .height(480.0)
    .show(ui);"##,
            show: |ui| {
                let t = Theme::of(ui);
                let s = persist(ui, "wf", || (workflow_graph(&t), NodeEditorState::default()));
                let mut s = s.lock().unwrap();
                let (g, st) = &mut *s;
                let cur = EdgeKind::ALL.iter().position(|k| *k == st.edge_kind).unwrap_or(0);
                let pick = chips(ui, "Edges", &["Bezier", "Smooth step", "Step", "Straight"], cur);
                st.edge_kind = EdgeKind::ALL[pick];
                NodeEditor::new(g, st).background(Background::Dots).height(H).show(ui);
            },
        },
        Demo {
            id: "shader-graph",
            name: "Shader Graph",
            category: C,
            description: "A material node graph with typed, color-coded ports: only compatible types connect, valid handles glow while you drag, and nodes render live previews.",
            covers: &["React Flow: Custom nodes", "React Flow: Connection validation", "Blender: Shader editor", "Unreal: Material editor"],
            code: r##"let f = |l: &str| Port::typed(l, "float", FLOAT);
let noise = graph.add(Node::new("Noise", pos2(0.0, 0.0)).input(Port::typed("uv", "vec2", VEC2)).output(f("value")).custom(84.0).tag("noise"));

NodeEditor::new(&mut graph, &mut state)
    .background(Background::Lines)
    .edge_kind(EdgeKind::Bezier)
    .node_body(|painter, rect, zoom, node| {
        if node.tag == "noise" { /* paint a preview into rect */ }
    })
    .show(ui);"##,
            show: |ui| {
                let s = persist(ui, "shader", || (shader_graph(), NodeEditorState::default()));
                let mut s = s.lock().unwrap();
                let (g, st) = &mut *s;
                let time = glowkit_core::time(ui);
                glowkit_core::animate(ui);
                NodeEditor::new(g, st).background(Background::Lines).height(H + 36.0).node_body(move |p, r, z, n| shader_body(p, r, z, n, time)).show(ui);
            },
        },
        Demo {
            id: "agent-pipeline",
            name: "AI Agent Pipeline",
            category: C,
            description: "An AI agent graph whose stages light up as a run flows through it: running spinners, progress bars, and particles travelling along the edges.",
            covers: &["React Flow: Animated edges", "LangGraph Studio: Graph view", "Flowise: Chatflow canvas", "xyflow: AI workflow"],
            code: r##"graph.link((planner, 0), (retriever, 0)).anim = EdgeAnim::Dots;
node.status = NodeStatus::Running; // spinner badge

NodeEditor::new(&mut graph, &mut state)
    .background(Background::Cross)
    .node_body(|painter, rect, zoom, node| { /* progress bar */ })
    .show(ui);"##,
            show: |ui| {
                let t = Theme::of(ui);
                let s = persist(ui, "agent", || (agent_graph(&t), { let mut s = NodeEditorState::default(); s.edge_kind = EdgeKind::SmoothStep; s }));
                let mut s = s.lock().unwrap();
                let (g, st) = &mut *s;
                let time = glowkit_core::time(ui);
                agent_tick(g, time);
                glowkit_core::animate(ui);
                let t2 = t.clone();
                NodeEditor::new(g, st).background(Background::Cross).height(H + 36.0).node_body(move |p, r, z, n| agent_body(p, r, z, n, time, &t2)).show(ui);
            },
        },
        Demo {
            id: "flowchart",
            name: "Flowchart",
            category: C,
            description: "Drag flowchart shapes (process, decision, terminator, data, document, event) from the palette onto the canvas and wire them top-to-bottom with arrowed smooth-step edges.",
            covers: &["React Flow: Drag and drop", "draw.io: Flowchart shapes", "Excalidraw: Library", "tldraw: Geo shapes"],
            code: r##"let palette = vec![
    PaletteItem { template: Node::shape(NodeShape::Decision, "Decision", Pos2::ZERO) },
    PaletteItem { template: Node::shape(NodeShape::Process, "Process", Pos2::ZERO) },
];
NodeEditor::new(&mut graph, &mut state)
    .palette(&palette)
    .arrows(true)
    .layout_direction(LayoutDirection::TopToBottom)
    .show(ui);"##,
            show: |ui| {
                let t = Theme::of(ui);
                let s = persist(ui, "flow", || (flowchart_graph(&t), { let mut s = NodeEditorState::default(); s.edge_kind = EdgeKind::SmoothStep; s }, flow_palette(&t)));
                let mut s = s.lock().unwrap();
                let (g, st, pal) = &mut *s;
                NodeEditor::new(g, st).palette(pal).arrows(true).layout_direction(LayoutDirection::TopToBottom).background(Background::Dots).height(H + 36.0).show(ui);
            },
        },
    ]
}
