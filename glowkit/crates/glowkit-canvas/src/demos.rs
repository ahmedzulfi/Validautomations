use crate::*;
use egui::{pos2, vec2, Color32, CornerRadius, Pos2, Rect, Ui};
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
    let hook = g.add(
        Node::new("Webhook", pos2(0.0, 120.0))
            .subtitle("POST /orders")
            .icon("⚡")
            .color(c[3])
            .output(Port::new("body"))
            .line("Last call 2s ago")
            .status(NodeStatus::Done),
    );
    let filter = g.add(
        Node::new("If", pos2(280.0, 120.0))
            .subtitle("amount > $500")
            .icon("?")
            .color(c[1])
            .input(Port::new("in"))
            .output(Port::new("true"))
            .output(Port::new("false"))
            .status(NodeStatus::Done),
    );
    let enrich = g.add(
        Node::new("HTTP Request", pos2(560.0, -30.0))
            .subtitle("GET /customers/:id")
            .icon("↗")
            .color(c[5])
            .input(Port::new("in"))
            .output(Port::new("json"))
            .line("Retry 3x · 10s timeout")
            .status(NodeStatus::Running),
    );
    let ai = g.add(
        Node::new("AI Summarize", pos2(840.0, 60.0))
            .subtitle("Model · temperature 0.2")
            .icon("✦")
            .color(c[0])
            .input(Port::new("order"))
            .input(Port::new("customer"))
            .output(Port::new("summary"))
            .line("\"Write a 2-line brief…\""),
    );
    let notify = g.add(
        Node::new("Notify #sales", pos2(1120.0, 0.0))
            .subtitle("Chat message")
            .icon("#")
            .color(c[2])
            .input(Port::new("text"))
            .output(Port::new("ok")),
    );
    let db = g.add(
        Node::new("Insert row", pos2(1120.0, 170.0))
            .subtitle("orders_large")
            .icon("▤")
            .color(c[4])
            .input(Port::new("row"))
            .output(Port::new("id")),
    );
    let mail = g.add(
        Node::new("Send receipt", pos2(560.0, 260.0))
            .subtitle("Email · template #12")
            .icon("@")
            .color(c[3])
            .input(Port::new("to"))
            .output(Port::new("sent")),
    );
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
    let uv =
        g.add(Node::new("UV", pos2(0.0, 0.0)).subtitle("Input").icon("◫").color(VEC2).width(170.0).output(v2("uv")));
    let time = g.add(
        Node::new("Time", pos2(0.0, 150.0))
            .subtitle("Input")
            .icon("◷")
            .color(FLOAT)
            .width(170.0)
            .output(f("t"))
            .output(f("sin t")),
    );
    let noise = g.add(
        Node::new("Noise", pos2(240.0, -20.0))
            .subtitle("Fractal · 4 octaves")
            .icon("≈")
            .color(FLOAT)
            .width(190.0)
            .input(v2("uv"))
            .input(f("scale"))
            .input(f("offset"))
            .output(f("value"))
            .custom(84.0)
            .tag("noise"),
    );
    let ramp = g.add(
        Node::new("Color Ramp", pos2(500.0, 10.0))
            .subtitle("3 stops")
            .icon("▥")
            .color(COLOR)
            .width(190.0)
            .input(f("factor"))
            .output(col("color"))
            .custom(26.0)
            .tag("ramp"),
    );
    let fres = g.add(
        Node::new("Fresnel", pos2(500.0, 230.0))
            .subtitle("IOR 1.45")
            .icon("◐")
            .color(FLOAT)
            .width(190.0)
            .input(f("ior"))
            .output(f("fac")),
    );
    let mix_n = g.add(
        Node::new("Mix", pos2(760.0, 60.0))
            .subtitle("Multiply")
            .icon("×")
            .color(COLOR)
            .width(180.0)
            .input(col("a"))
            .input(f("b"))
            .output(col("result")),
    );
    let out = g.add(
        Node::new("Material Output", pos2(1010.0, 40.0))
            .subtitle("Surface")
            .icon("◉")
            .color(hex("#A78BFA"))
            .width(210.0)
            .input(col("base color"))
            .input(f("roughness"))
            .input(col("emission"))
            .custom(96.0)
            .tag("output"),
    );
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
    let q = g.add(
        Node::new("User query", pos2(0.0, 80.0))
            .subtitle("chat input")
            .icon("›")
            .color(c[5])
            .width(180.0)
            .output(Port::new("text"))
            .line("\"Compare Q3 churn by region\""),
    );
    let plan = g.add(
        Node::new("Planner", pos2(250.0, 80.0))
            .subtitle("LLM · tool calling")
            .icon("✦")
            .color(c[0])
            .input(Port::new("goal"))
            .output(Port::new("search"))
            .output(Port::new("sql"))
            .custom(14.0)
            .tag("agent"),
    );
    let rag = g.add(
        Node::new("Retriever", pos2(530.0, -40.0))
            .subtitle("vector search · k=8")
            .icon("⌕")
            .color(c[1])
            .input(Port::new("query"))
            .output(Port::new("chunks"))
            .custom(14.0)
            .tag("agent"),
    );
    let sql = g.add(
        Node::new("SQL tool", pos2(530.0, 170.0))
            .subtitle("warehouse · read-only")
            .icon("▤")
            .color(c[4])
            .input(Port::new("query"))
            .output(Port::new("rows"))
            .custom(14.0)
            .tag("agent"),
    );
    let rerank = g.add(
        Node::new("Reranker", pos2(800.0, -40.0))
            .subtitle("cross-encoder")
            .icon("⇅")
            .color(c[1])
            .width(180.0)
            .input(Port::new("chunks"))
            .output(Port::new("top 3"))
            .custom(14.0)
            .tag("agent"),
    );
    let writer = g.add(
        Node::new("Writer", pos2(1040.0, 70.0))
            .subtitle("LLM · streaming")
            .icon("✎")
            .color(c[0])
            .input(Port::new("context").multi())
            .output(Port::new("draft"))
            .custom(14.0)
            .tag("agent"),
    );
    let guard = g.add(
        Node::new("Guardrails", pos2(1300.0, 70.0))
            .subtitle("PII · policy")
            .icon("⛨")
            .color(c[2])
            .width(180.0)
            .input(Port::new("draft"))
            .output(Port::new("answer"))
            .custom(14.0)
            .tag("agent"),
    );
    for (a, ap, b, bp) in [
        (q, 0, plan, 0),
        (plan, 0, rag, 0),
        (plan, 1, sql, 0),
        (rag, 0, rerank, 0),
        (rerank, 0, writer, 0),
        (sql, 0, writer, 0),
        (writer, 0, guard, 0),
    ] {
        let col = g.node(a).and_then(|n| n.color);
        let e = g.link((a, ap), (b, bp));
        e.anim = EdgeAnim::Dots;
        e.color = col;
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
            n.status = if k < i {
                NodeStatus::None
            } else if k < i + 1.0 {
                NodeStatus::Running
            } else {
                NodeStatus::Done
            };
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
        PaletteItem {
            template: {
                let mut n = Node::shape(NodeShape::Decision, "Decision", Pos2::ZERO).color(c[3]);
                n.outputs = vec![Port::new("yes"), Port::new("no")];
                n
            },
        },
        PaletteItem { template: Node::shape(NodeShape::Data, "Input / Output", Pos2::ZERO).color(c[1]) },
        PaletteItem { template: Node::shape(NodeShape::Document, "Document", Pos2::ZERO).color(c[5]) },
        PaletteItem { template: Node::shape(NodeShape::Circle, "Event", Pos2::ZERO).color(c[2]) },
    ]
}

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
            covers: &["React Flow: Node-based UI", "React Flow: MiniMap", "React Flow: Controls", "React Flow: Node Toolbar", "xyflow: Workflow editor", "n8n: Workflow canvas"],
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
        Demo {
            id: "infinite-canvas",
            name: "Infinite Canvas",
            category: C,
            description: "A pannable, zoomable surface: drag / middle-mouse / space+drag to pan, ctrl+wheel or pinch to zoom smoothly around the cursor, dot / line / cross backgrounds, minimap and fit-to-content.",
            covers: &["React Flow: Background", "React Flow: Viewport", "tldraw: Infinite canvas", "Figma: Canvas navigation", "react-zoom-pan-pinch"],
            code: r##"let mut view = CanvasView::default(); // keep it in your app state
InfiniteCanvas::new(&mut view)
    .background(Background::Dots)
    .height(480.0)
    .show(ui, |cv| {
        let r = cv.rect_to_screen(Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 120.0)));
        cv.painter.rect_filled(r, 12.0 * cv.zoom(), Color32::DARK_GRAY);
    });"##,
            show: |ui| {
                let s = persist(ui, "inf", || (CanvasView::with_limits(0.1, 6.0), 0usize));
                let mut s = s.lock().unwrap();
                let (view, bgk) = &mut *s;
                *bgk = chips(ui, "Background", &["Dots", "Lines", "Cross", "None"], *bgk);
                let bg = [Background::Dots, Background::Lines, Background::Cross, Background::None][*bgk];
                infinite_demo(ui, view, bg);
            },
        },
        Demo {
            id: "whiteboard",
            name: "Whiteboard",
            category: C,
            description: "An infinite whiteboard: pen with pressure-like strokes, highlighter, shapes, arrows that stick to shapes, text, sticky notes, eraser, resize / rotate handles, a hand-drawn sketchy style, layers, undo and JSON export.",
            covers: &["tldraw: Editor", "Excalidraw: Canvas", "Miro: Board", "FigJam: Whiteboard", "perfect-freehand", "rough.js: Sketchy shapes"],
            code: r##"let mut doc = Document::default();
doc.add(WbShape::sticky(pos2(0.0, 0.0), "Ship it!", STICKY_COLORS[0]));
doc.add(WbShape::new(ShapeKind::Rect, Rect::from_min_size(pos2(220.0, 0.0), vec2(160.0, 90.0))).sketchy(true).label("API"));

Whiteboard::new(&mut doc, &mut state).height(480.0).show(ui);
let json = doc.to_json();                  // export
let doc2 = Document::from_json(&json)?;    // import"##,
            show: |ui| {
                let s = persist(ui, "wb", || (whiteboard_doc(), WhiteboardState::default()));
                let mut s = s.lock().unwrap();
                let (doc, st) = &mut *s;
                Whiteboard::new(doc, st).height(H + 36.0).show(ui);
            },
        },
        Demo {
            id: "kanban-board",
            name: "Kanban Board",
            category: C,
            description: "Drag cards within and between columns: other cards spring aside to open a gap and dropped cards glide into place. Inline add, WIP limits, labels, avatars, due dates and collapsible columns.",
            covers: &["Trello: Board", "Linear: Board view", "Jira: Board", "dnd-kit: Sortable multiple containers", "Kibo UI: Kanban"],
            code: r##"let mut board = Board { columns: vec![
    Column::new(1, "Todo", hex("#71717A")).cards(vec![
        Card::new(1, "Design onboarding").label("Design", hex("#8B5CF6")).assignees(&["AK"]).due("Nov 14"),
    ]),
    Column::new(2, "In progress", hex("#3B82F6")).wip(3),
] };
KanbanBoard::new(&mut board, &mut state).height(480.0).show(ui);"##,
            show: |ui| {
                let t = Theme::of(ui);
                let s = persist(ui, "kb", || (kanban_board(&t), KanbanState::default()));
                let mut s = s.lock().unwrap();
                let (b, st) = &mut *s;
                KanbanBoard::new(b, st).height(H + 36.0).show(ui);
            },
        },
        Demo {
            id: "gantt-chart",
            name: "Gantt Chart",
            category: C,
            description: "A project timeline: grouped rows, bars with progress fill, milestones, dependency arrows (red when violated), a today line and day / week / month zoom. Drag bars to move, edges to resize, the notch to set progress.",
            covers: &["Frappe Gantt", "dhtmlxGantt", "Linear: Timeline", "Notion: Timeline view", "SVAR: React Gantt"],
            code: r##"let mut g = Gantt { start: (2026, 11, 2), today: 9.0, groups: vec![
    GanttGroup::new("Design", vec![
        GanttTask::new(1, "Research", 0.0, 4.0).progress(1.0),
        GanttTask::new(2, "Wireframes", 4.0, 5.0).progress(0.6).after(&[1]),
    ]),
] };
GanttChart::new(&mut g, &mut state).height(480.0).show(ui);"##,
            show: |ui| {
                let t = Theme::of(ui);
                let s = persist(ui, "gantt", || (gantt_data(&t), GanttState::default()));
                let mut s = s.lock().unwrap();
                let (g, st) = &mut *s;
                GanttChart::new(g, st).height(H + 50.0).show(ui);
            },
        },
        Demo {
            id: "mind-map",
            name: "Mind Map",
            category: C,
            description: "Auto-laid-out mind map with tapered, color-coded branches. Tab adds a child, Enter a sibling, Space folds a branch with an animated collapse; switch between balanced, right and radial layouts.",
            covers: &["MindNode", "XMind: Mind map", "Miro: Mind map", "Whimsical: Mind maps", "markmap"],
            code: r##"let mut map = MindMap::new("Product launch");
let root = map.root().unwrap();
let mkt = map.add_child(root, "Marketing");
map.add_child(mkt, "Launch video");
MindMapEditor::new(&mut map, &mut state).height(480.0).show(ui);"##,
            show: |ui| {
                let s = persist(ui, "mm", || (mind_data(), MindMapState::default()));
                let mut s = s.lock().unwrap();
                let (m, st) = &mut *s;
                MindMapEditor::new(m, st).height(H).show(ui);
            },
        },
        Demo {
            id: "timeline-editor",
            name: "Timeline Editor",
            category: C,
            description: "Video-editor tracks with thumbnail strips and waveforms: drag clips across tracks, trim either edge with magnetic snapping, scrub the playhead on the zoomable ruler, press space to play.",
            covers: &["Premiere Pro: Timeline", "DaVinci Resolve: Edit page", "CapCut: Timeline", "Remotion: Timeline", "react-timeline-editor"],
            code: r##"let mut tl = Timeline { fps: 30.0, duration: 90.0, tracks: vec![
    Track::new("V1", ClipKind::Video, vec![Clip::new(1, "Intro.mp4", 0.0, 8.0)]),
    Track::new("A1", ClipKind::Audio, vec![Clip::new(2, "Music.wav", 0.0, 60.0)]),
] };
TimelineEditor::new(&mut tl, &mut state).height(420.0).show(ui);"##,
            show: |ui| {
                let s = persist(ui, "tl", || (timeline_data(), { let mut st = TimelineState::default(); st.playhead = 12.4; st }));
                let mut s = s.lock().unwrap();
                let (tl, st) = &mut *s;
                TimelineEditor::new(tl, st).height(H).show(ui);
            },
        },
        Demo {
            id: "data-grid",
            name: "Data Grid",
            category: C,
            description: "A virtualized spreadsheet with 10,000 rows: frozen header, row numbers and first column, drag-select ranges, type to edit, Enter / Tab navigation, resize columns, TSV copy / paste and a live sum.",
            covers: &["Glide Data Grid", "AG Grid", "Kibo UI: Table", "TanStack Table: Virtualized", "Excel / Google Sheets grid"],
            code: r##"let mut data = GridData {
    columns: vec![
        GridColumn::new("Customer", 180.0, CellKind::Text),
        GridColumn::new("MRR", 110.0, CellKind::Currency),
        GridColumn::new("Usage", 140.0, CellKind::Percent),
    ],
    rows, // Vec<Vec<String>>
};
DataGrid::new(&mut data, &mut state).height(480.0).frozen_columns(1).show(ui);"##,
            show: |ui| {
                let s = persist(ui, "grid", || (grid_data(), GridState::default()));
                let mut s = s.lock().unwrap();
                let (d, st) = &mut *s;
                DataGrid::new(d, st).height(H).show(ui);
            },
        },
        Demo {
            id: "org-chart",
            name: "Org Chart",
            category: C,
            description: "A tidy top-down tree on a pan / zoom canvas: person cards, collapsible teams with report counts, compact stacking for leaf teams, and the selected person's chain of command highlighted.",
            covers: &["d3-org-chart", "React Flow: Tree layout", "Lucidchart: Org chart", "Kendo: OrgChart"],
            code: r##"let mut chart = OrgChart { people: vec![
    OrgPerson::new(1, None, "Ada Park", "CEO"),
    OrgPerson::new(2, Some(1), "Lin Chen", "CTO"),
    OrgPerson::new(3, Some(2), "Sam Ortiz", "Staff Engineer"),
] };
OrgChartView::new(&mut chart, &mut state).height(480.0).show(ui);"##,
            show: |ui| {
                let t = Theme::of(ui);
                let s = persist(ui, "org", || (org_data(&t), OrgChartState::default()));
                let mut s = s.lock().unwrap();
                let (c, st) = &mut *s;
                OrgChartView::new(c, st).height(H).show(ui);
            },
        },
        Demo {
            id: "signature-pad",
            name: "Signature Pad",
            category: C,
            description: "Smooth ink capture with speed-based thinning and tapered ends: a signature field with a baseline and a free drawing pad, with undo, clear and point export.",
            covers: &["signature_pad", "react-signature-canvas", "perfect-freehand", "shadcn: Signature"],
            code: r##"let mut ink = Ink::default();
ui.add(SignaturePad::new(&mut ink).size(vec2(520.0, 200.0)));
if !ink.is_empty() {
    let json = ink.to_json();       // [[[x, y, t], ...], ...]
    let svg = ink.to_svg_path();    // "M1 2 L3 4 ..."
}"##,
            show: |ui| {
                let t = Theme::of(ui);
                let s = persist(ui, "sig", || (sample_signature(), Ink::default(), 0usize));
                let mut s = s.lock().unwrap();
                let (sig, pad, color) = &mut *s;
                ui.add_space(8.0);
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Signature").size(13.0).strong());
                        ui.add_space(4.0);
                        ui.add(SignaturePad::new(sig).size(vec2(460.0, 210.0)));
                        ui.add_space(6.0);
                        let json = sig.to_json();
                        ui.label(egui::RichText::new(format!("{} strokes · {} points · {:.1} KB JSON", sig.strokes.len(), sig.point_count(), json.len() as f32 / 1024.0)).size(11.5).color(t.muted_foreground));
                        let preview: String = json.chars().take(64).collect();
                        ui.label(egui::RichText::new(format!("{preview}…")).monospace().size(10.5).color(t.muted_foreground));
                    });
                    ui.add_space(18.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Drawing pad").size(13.0).strong());
                        ui.add_space(4.0);
                        let colors = [t.foreground, t.chart[0], t.chart[1], t.chart[2], t.chart[4]];
                        let w = (ui.available_width()).clamp(240.0, 460.0);
                        ui.add(SignaturePad::new(pad).size(vec2(w, 210.0)).placeholder("Draw anything").baseline(false).color(colors[*color % colors.len()]).style(InkStyle { size: 6.0, ..InkStyle::default() }));
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            for (k, c) in colors.iter().enumerate() {
                                let (r, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), egui::Sense::click());
                                let p = ui.painter();
                                p.circle(r.center(), if *color == k { 9.0 } else { 7.5 }, *c, egui::Stroke::new(if *color == k { 2.0 } else { 1.0 }, if *color == k { t.ring } else { t.border }));
                                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    *color = k;
                                }
                            }
                            ui.label(egui::RichText::new(format!("{} points", pad.point_count())).size(11.5).color(t.muted_foreground));
                        });
                    });
                });
            },
        },
    ]
}

// ------------------------------------------------------------------------------------------------
// Infinite canvas demo content
// ------------------------------------------------------------------------------------------------

fn infinite_demo(ui: &mut Ui, view: &mut CanvasView, bg: Background) {
    let t = Theme::of(ui);
    let frames: [(&str, Rect, Color32); 5] = [
        ("Home", Rect::from_min_size(pos2(0.0, 0.0), vec2(260.0, 180.0)), t.chart[0]),
        ("Pricing", Rect::from_min_size(pos2(320.0, -40.0), vec2(220.0, 260.0)), t.chart[1]),
        ("Checkout", Rect::from_min_size(pos2(600.0, 30.0), vec2(240.0, 170.0)), t.chart[2]),
        ("Dashboard", Rect::from_min_size(pos2(80.0, 260.0), vec2(360.0, 200.0)), t.chart[3]),
        ("Settings", Rect::from_min_size(pos2(520.0, 270.0), vec2(200.0, 190.0)), t.chart[4]),
    ];
    let content = frames.iter().fold(Rect::NOTHING, |r, f| r.union(f.1)).expand2(vec2(0.0, 24.0));
    InfiniteCanvas::new(view).background(bg).height(H).show(ui, |cv| {
        if cv.view.needs_fit {
            cv.view.fit_now(cv.rect, content, 60.0);
        }
        let z = cv.zoom();
        let time = glowkit_core::time(cv.ui);
        for (k, (name, r, col)) in frames.iter().enumerate() {
            let sr = cv.rect_to_screen(*r);
            if !sr.expand(40.0).intersects(cv.rect) {
                continue;
            }
            cv.painter.text(
                sr.left_top() - vec2(0.0, 8.0 * z),
                egui::Align2::LEFT_BOTTOM,
                *name,
                crate::common::qfont(12.0 * z),
                t.muted_foreground,
            );
            paint::soft_shadow(
                &cv.painter,
                sr,
                10.0 * z,
                18.0 * z,
                vec2(0.0, 6.0 * z),
                Color32::from_black_alpha(if t.dark { 120 } else { 30 }),
            );
            cv.painter.rect(
                sr,
                CornerRadius::same((10.0 * z) as u8),
                t.card,
                egui::Stroke::new(1.0, t.border),
                egui::StrokeKind::Inside,
            );
            // mock UI inside each frame
            let pad = 14.0 * z;
            let inner = sr.shrink(pad);
            cv.painter.rect_filled(
                Rect::from_min_size(inner.min, vec2(inner.width() * 0.45, 8.0 * z)),
                CornerRadius::same(3),
                with_alpha(t.foreground, 0.7),
            );
            cv.painter.rect_filled(
                Rect::from_min_size(inner.min + vec2(0.0, 16.0 * z), vec2(inner.width() * 0.7, 6.0 * z)),
                CornerRadius::same(3),
                with_alpha(t.foreground, 0.2),
            );
            let hero =
                Rect::from_min_max(inner.min + vec2(0.0, 32.0 * z), pos2(inner.right(), inner.bottom() - 26.0 * z));
            if hero.height() > 4.0 {
                let c1 = *col;
                let c2 = t.chart[(k + 2) % 6];
                let shift = (time * 0.3 + k as f32).sin() * 0.5 + 0.5;
                paint::gradient_rect(
                    &cv.painter,
                    hero,
                    8.0 * z,
                    30.0 + 40.0 * shift,
                    &[with_alpha(c1, 0.85), with_alpha(c2, 0.6)],
                );
            }
            let btn = Rect::from_min_size(pos2(inner.left(), inner.bottom() - 18.0 * z), vec2(70.0 * z, 18.0 * z));
            cv.painter.rect_filled(btn, CornerRadius::same((9.0 * z) as u8), *col);
        }
        glowkit_core::animate(cv.ui);
        // hint
        let hint = "Drag or space+drag to pan  ·  Ctrl+wheel / pinch to zoom  ·  middle-drag anywhere";
        cv.painter.text(
            pos2(cv.rect.center().x, cv.rect.top() + 22.0),
            egui::Align2::CENTER_CENTER,
            hint,
            crate::common::qfont(11.5),
            t.muted_foreground,
        );
        let items: Vec<(Rect, Color32)> = frames.iter().map(|f| (f.1, f.2)).collect();
        let rect = cv.rect;
        MiniMap::new(&mut *cv.view, rect).items(&items).show(cv.ui);
        if let Some(ControlAction::Fit) = Controls::new(&mut *cv.view, rect).show(cv.ui) {
            cv.view.fit(rect, content, 60.0);
        }
    });
}

// ------------------------------------------------------------------------------------------------
// Whiteboard content
// ------------------------------------------------------------------------------------------------

fn whiteboard_doc() -> Document {
    let mut d = Document::default();
    let blue = WB_COLORS[4];
    let red = WB_COLORS[1];
    let green = WB_COLORS[3];
    d.add(
        WbShape::freehand(
            synthetic_stroke(&[pos2(-6.0, -62.0), pos2(80.0, -66.0), pos2(170.0, -61.0), pos2(236.0, -64.0)]),
            true,
        )
        .stroke(Color32::from_rgb(250, 204, 21))
        .width(16.0),
    );
    d.add(WbShape::text(pos2(-10.0, -96.0), "Launch plan · Q1", 30.0));
    let design = d.add(
        WbShape::new(ShapeKind::Rect, Rect::from_min_size(pos2(0.0, 20.0), vec2(170.0, 90.0)))
            .sketchy(true)
            .fill(with_alpha(blue, 0.22))
            .stroke(blue)
            .label("Design"),
    );
    let build = d.add(
        WbShape::new(ShapeKind::Diamond, Rect::from_min_size(pos2(260.0, 0.0), vec2(170.0, 130.0)))
            .sketchy(true)
            .fill(with_alpha(WB_COLORS[2], 0.22))
            .stroke(WB_COLORS[2])
            .label("Ready?"),
    );
    let ship = d.add(
        WbShape::new(ShapeKind::Ellipse, Rect::from_min_size(pos2(520.0, 15.0), vec2(170.0, 100.0)))
            .sketchy(true)
            .fill(with_alpha(green, 0.22))
            .stroke(green)
            .label("Ship"),
    );
    let mut a1 = WbShape::linear(ShapeKind::Arrow, pos2(170.0, 65.0), pos2(260.0, 65.0)).sketchy(true);
    a1.bind_a = Some(design);
    a1.bind_b = Some(build);
    d.add(a1);
    let mut a2 = WbShape::linear(ShapeKind::Arrow, pos2(430.0, 65.0), pos2(520.0, 65.0)).sketchy(true);
    a2.bind_a = Some(build);
    a2.bind_b = Some(ship);
    d.add(a2);
    d.add(
        WbShape::sticky(pos2(-30.0, 170.0), "Talk to 5 customers before freezing scope", STICKY_COLORS[0])
            .rotated(-0.04),
    );
    d.add(WbShape::sticky(pos2(170.0, 180.0), "Perf budget:\n< 1 ms / frame", STICKY_COLORS[2]).rotated(0.03));
    d.add(WbShape::sticky(pos2(370.0, 172.0), "Launch blog post + demo video", STICKY_COLORS[1]).rotated(-0.02));
    // a hand-drawn circle around "Ship" and a check mark
    let circle: Vec<Pos2> = (0..=26)
        .map(|i| {
            let a = -0.6 + i as f32 / 24.0 * std::f32::consts::TAU;
            pos2(605.0 + a.cos() * (112.0 + (i as f32 * 0.8).sin() * 3.0), 65.0 + a.sin() * 70.0)
        })
        .collect();
    d.add(WbShape::freehand(synthetic_stroke(&circle), false).stroke(red).width(2.6));
    d.add(
        WbShape::freehand(synthetic_stroke(&[pos2(600.0, 190.0), pos2(618.0, 212.0), pos2(660.0, 160.0)]), false)
            .stroke(green)
            .width(4.0),
    );
    d.add(WbShape::text(pos2(670.0, 176.0), "on track!", 20.0).stroke(green));
    d
}

// ------------------------------------------------------------------------------------------------
// Kanban content
// ------------------------------------------------------------------------------------------------

fn kanban_board(t: &Theme) -> Board {
    let (design, eng, bug, docs, growth) = (t.chart[0], t.chart[5], t.destructive, t.chart[4], t.chart[3]);
    Board {
        columns: vec![
            Column::new(1, "Backlog", t.muted_foreground).cards(vec![
                Card::new(1, "Dark mode for the analytics dashboard")
                    .label("Design", design)
                    .assignees(&["MK"])
                    .checklist(1, 4),
                Card::new(2, "Export reports as CSV").label("Feature", eng).due("Nov 28"),
                Card::new(3, "Audit onboarding emails").label("Growth", growth).assignees(&["JS", "AL"]),
            ]),
            Column::new(2, "To do", t.info).cards(vec![
                Card::new(4, "Rate-limit the public API")
                    .label("Backend", eng)
                    .label("P1", bug)
                    .assignees(&["DO"])
                    .due("Nov 18")
                    .priority(bug),
                Card::new(5, "Write migration guide for v2").label("Docs", docs).checklist(2, 6),
            ]),
            Column::new(3, "In progress", t.warning).wip(3).cards(vec![
                Card::new(6, "Kanban drag & drop polish")
                    .label("Frontend", design)
                    .assignees(&["AK", "MK"])
                    .checklist(5, 7)
                    .due("Nov 14"),
                Card::new(7, "Fix flaky checkout test")
                    .label("Bug", bug)
                    .assignees(&["DO"])
                    .due("!Nov 10")
                    .priority(bug),
                Card::new(8, "Pricing page experiment").label("Growth", growth).assignees(&["JS"]),
                Card::new(9, "Search indexing worker").label("Backend", eng).assignees(&["LC"]).checklist(3, 5),
            ]),
            Column::new(4, "Review", t.chart[2]).cards(vec![Card::new(10, "New billing settings screen")
                .label("Design", design)
                .assignees(&["AL"])
                .checklist(8, 8)]),
            Column::new(5, "Done", t.success).cards(vec![
                Card::new(11, "SSO with SAML").label("Backend", eng).assignees(&["LC"]),
                Card::new(12, "Changelog page").label("Docs", docs),
            ]),
        ],
    }
}

// ------------------------------------------------------------------------------------------------
// Gantt content
// ------------------------------------------------------------------------------------------------

fn gantt_data(t: &Theme) -> Gantt {
    Gantt {
        start: (2026, 11, 2),
        today: 9.0,
        groups: vec![
            GanttGroup::new(
                "Discovery",
                vec![
                    GanttTask::new(1, "User interviews", 0.0, 4.0).progress(1.0).assignee("JS"),
                    GanttTask::new(2, "Competitive audit", 1.0, 3.0).progress(1.0).assignee("MK"),
                    GanttTask::new(3, "Scope sign-off", 5.0, 0.0).after(&[1, 2]),
                ],
            )
            .color(t.chart[0]),
            GanttGroup::new(
                "Design",
                vec![
                    GanttTask::new(4, "Wireframes", 5.0, 5.0).progress(0.8).after(&[3]).assignee("AL"),
                    GanttTask::new(5, "Visual design", 10.0, 6.0).progress(0.25).after(&[4]).assignee("MK"),
                    GanttTask::new(6, "Prototype review", 16.0, 0.0).after(&[5]),
                ],
            )
            .color(t.chart[2]),
            GanttGroup::new(
                "Engineering",
                vec![
                    GanttTask::new(7, "API + data model", 7.0, 8.0).progress(0.45).after(&[3]).assignee("LC"),
                    GanttTask::new(8, "Frontend build", 15.0, 10.0).progress(0.0).after(&[5, 7]).assignee("AK"),
                    GanttTask::new(9, "QA & bug bash", 25.0, 4.0).after(&[8]).assignee("DO"),
                ],
            )
            .color(t.chart[1]),
            GanttGroup::new(
                "Launch",
                vec![
                    GanttTask::new(10, "Marketing site", 20.0, 7.0).progress(0.1).assignee("JS"),
                    GanttTask::new(11, "Public launch", 30.0, 0.0).after(&[9, 10]),
                ],
            )
            .color(t.chart[4]),
        ],
    }
}

// ------------------------------------------------------------------------------------------------
// Mind map content
// ------------------------------------------------------------------------------------------------

fn mind_data() -> MindMap {
    let mut m = MindMap::new("Product launch");
    let r = m.root().unwrap_or(1);
    let mk = m.add_child(r, "Marketing");
    for s in ["Launch video", "Press kit", "Social campaign"] {
        m.add_child(mk, s);
    }
    let pr = m.add_child(r, "Product");
    let core = m.add_child(pr, "Core features");
    m.add_child(core, "Realtime sync");
    m.add_child(core, "Offline mode");
    m.add_child(pr, "Beta feedback");
    let eng = m.add_child(r, "Engineering");
    m.add_child(eng, "Load testing");
    m.add_child(eng, "Rollout plan");
    let sales = m.add_child(r, "Sales");
    m.add_child(sales, "Pricing tiers");
    m.add_child(sales, "Partner deals");
    let sup = m.add_child(r, "Support");
    m.add_child(sup, "Help center");
    m.add_child(sup, "On-call rota");
    m
}

// ------------------------------------------------------------------------------------------------
// Timeline content
// ------------------------------------------------------------------------------------------------

fn timeline_data() -> Timeline {
    Timeline {
        fps: 30.0,
        duration: 64.0,
        tracks: vec![
            Track::new(
                "Effects",
                ClipKind::Effect,
                vec![Clip::new(13, "Color grade · warm", 0.0, 24.0), Clip::new(14, "Glow", 33.0, 9.0)],
            ),
            Track::new(
                "Titles",
                ClipKind::Title,
                vec![
                    Clip::new(1, "Opening title", 1.0, 4.5),
                    Clip::new(2, "Lower third · Maya", 14.0, 5.0),
                    Clip::new(3, "End card", 52.0, 6.0),
                ],
            ),
            Track::new(
                "Video 2",
                ClipKind::Video,
                vec![Clip::new(4, "B-roll city.mp4", 9.0, 7.5), Clip::new(5, "Drone.mov", 33.0, 9.0)],
            ),
            Track::new(
                "Video 1",
                ClipKind::Video,
                vec![
                    Clip::new(6, "Intro.mp4", 0.0, 9.0),
                    Clip::new(7, "Interview A.mp4", 9.0, 15.0),
                    Clip::new(8, "Demo screen.mov", 24.0, 18.0),
                    Clip::new(9, "Outro.mp4", 42.0, 16.0),
                ],
            ),
            Track::new(
                "Voice",
                ClipKind::Audio,
                vec![Clip::new(10, "VO take 3.wav", 2.0, 20.0), Clip::new(11, "VO take 5.wav", 25.0, 16.0)],
            ),
            Track::new("Music", ClipKind::Audio, vec![Clip::new(12, "Lo-fi bed.mp3", 0.0, 58.0)]),
            Track::new(
                "SFX",
                ClipKind::Audio,
                vec![
                    Clip::new(15, "Whoosh.wav", 8.4, 1.2),
                    Clip::new(16, "Click.wav", 23.6, 0.8),
                    Clip::new(17, "Riser.wav", 40.0, 2.4),
                ],
            ),
        ],
    }
}

// ------------------------------------------------------------------------------------------------
// Grid content
// ------------------------------------------------------------------------------------------------

fn grid_data() -> GridData {
    let first = [
        "Ada", "Lin", "Sam", "Maya", "Noah", "Iris", "Omar", "Zoe", "Liam", "Aria", "Kenji", "Lena", "Mateo", "Priya",
        "Elias", "Nora",
    ];
    let last = [
        "Park", "Chen", "Ortiz", "Patel", "Kim", "Novak", "Haddad", "Silva", "Berg", "Rossi", "Tanaka", "Fischer",
        "Diaz", "Rao", "Weber", "Lund",
    ];
    let company = [
        "Northwind",
        "Acme Labs",
        "Globex",
        "Initech",
        "Umbrella",
        "Hooli",
        "Vandelay",
        "Stark Ind.",
        "Wayne Corp",
        "Pied Piper",
        "Soylent",
        "Tyrell",
    ];
    let plan = ["Enterprise", "Pro", "Starter", "Trial", "Churned", "Paused"];
    let country = ["US", "DE", "JP", "BR", "IN", "FR", "UK", "CA", "SE", "KR"];
    let mut rng = glowkit_core::Rng::new(42);
    let rows = (0..10_000)
        .map(|i| {
            let f = *rng.pick(&first);
            let l = *rng.pick(&last);
            let p = *rng.pick(&plan);
            let seats = match p {
                "Enterprise" => 50 + rng.below(950),
                "Pro" => 5 + rng.below(60),
                _ => 1 + rng.below(8),
            };
            let mrr = match p {
                "Churned" | "Trial" => 0.0,
                "Enterprise" => seats as f32 * rng.range(28.0, 42.0),
                "Pro" => seats as f32 * 19.0,
                _ => seats as f32 * 9.0,
            };
            let day = 1 + rng.below(28);
            let month = 1 + rng.below(12);
            vec![
                format!("{f} {l}"),
                format!("CUS-{:05}", 10_000 + i),
                rng.pick(&company).to_string(),
                p.to_string(),
                format!("{mrr:.2}"),
                seats.to_string(),
                format!("{:.0}", rng.range(3.0, 100.0)),
                format!("2025-{month:02}-{day:02}"),
                rng.pick(&country).to_string(),
            ]
        })
        .collect();
    GridData {
        columns: vec![
            GridColumn::new("Customer", 170.0, CellKind::Text),
            GridColumn::new("ID", 104.0, CellKind::Mono),
            GridColumn::new("Company", 130.0, CellKind::Text),
            GridColumn::new("Plan", 120.0, CellKind::Status),
            GridColumn::new("MRR", 110.0, CellKind::Currency),
            GridColumn::new("Seats", 80.0, CellKind::Number),
            GridColumn::new("Usage", 150.0, CellKind::Percent),
            GridColumn::new("Signed up", 110.0, CellKind::Mono),
            GridColumn::new("Country", 90.0, CellKind::Text),
        ],
        rows,
    }
}

// ------------------------------------------------------------------------------------------------
// Org chart content
// ------------------------------------------------------------------------------------------------

fn org_data(t: &Theme) -> OrgChart {
    let (exec, eng, design, gtm) = (t.chart[0], t.chart[1], t.chart[2], t.chart[4]);
    OrgChart {
        people: vec![
            OrgPerson::new(1, None, "Ada Park", "Chief Executive Officer").color(exec),
            OrgPerson::new(2, Some(1), "Lin Chen", "Chief Technology Officer").color(eng),
            OrgPerson::new(3, Some(1), "Maya Patel", "Head of Design").color(design),
            OrgPerson::new(4, Some(1), "Omar Haddad", "VP Go-to-Market").color(gtm),
            OrgPerson::new(5, Some(2), "Sam Ortiz", "Eng Manager · Platform").color(eng),
            OrgPerson::new(6, Some(2), "Iris Novak", "Eng Manager · Product").color(eng),
            OrgPerson::new(7, Some(5), "Kenji Tanaka", "Staff Engineer").color(eng),
            OrgPerson::new(8, Some(5), "Lena Fischer", "Senior SRE").color(eng),
            OrgPerson::new(9, Some(5), "Elias Weber", "Backend Engineer").color(eng),
            OrgPerson::new(10, Some(6), "Zoe Silva", "Frontend Engineer").color(eng),
            OrgPerson::new(11, Some(6), "Liam Berg", "Mobile Engineer").color(eng),
            OrgPerson::new(12, Some(3), "Aria Rossi", "Product Designer").color(design),
            OrgPerson::new(13, Some(3), "Noah Kim", "Brand Designer").color(design),
            OrgPerson::new(14, Some(4), "Priya Rao", "Sales Lead").color(gtm),
            OrgPerson::new(15, Some(4), "Mateo Diaz", "Marketing Lead").color(gtm),
            OrgPerson::new(16, Some(14), "Nora Lund", "Account Executive").color(gtm),
            OrgPerson::new(17, Some(14), "Jon Snell", "SDR").color(gtm),
        ],
    }
}

// ------------------------------------------------------------------------------------------------
// Signature content
// ------------------------------------------------------------------------------------------------

fn sample_signature() -> Ink {
    use std::f32::consts::PI;
    let mut ink = Ink::default();
    let base = 150.0;
    let slant = 0.32;
    let sl = |p: Pos2| pos2(p.x + (base - p.y) * slant, p.y);
    // capital "J" with a curl
    let cap = [
        pos2(92.0, 70.0),
        pos2(118.0, 64.0),
        pos2(150.0, 66.0),
        pos2(126.0, 72.0),
        pos2(118.0, 110.0),
        pos2(112.0, 160.0),
        pos2(98.0, 186.0),
        pos2(80.0, 182.0),
        pos2(78.0, 166.0),
        pos2(98.0, 150.0),
    ];
    ink.strokes.push(synthetic_stroke(&cap.map(sl)));
    // the cursive body: humps of varying height, tall ones loop like ascenders
    let heights = [0.42, 0.5, 1.1, 0.46, 0.4, 0.95, 0.5, 0.36, 0.44];
    let widths = [20.0, 22.0, 15.0, 22.0, 18.0, 14.0, 24.0, 19.0, 22.0];
    let mut pts = vec![pos2(124.0, base)];
    let mut x0 = 126.0;
    for (h, w) in heights.iter().zip(widths.iter()) {
        let loop_k = if *h > 0.9 { 11.0 } else { 3.0 };
        for j in 1..=7 {
            let u = j as f32 / 7.0;
            let y = base - h * 52.0 * (PI * u).sin().max(0.0).powf(0.85);
            let x = x0 + w * u - loop_k * (2.0 * PI * u).sin();
            pts.push(pos2(x, y));
        }
        x0 += w;
    }
    // trailing swoop
    pts.extend([
        pos2(x0 + 14.0, base - 18.0),
        pos2(x0 + 30.0, base - 34.0),
        pos2(x0 + 22.0, base - 30.0),
        pos2(x0 + 34.0, base + 2.0),
        pos2(x0 + 60.0, base + 4.0),
    ]);
    let body: Vec<Pos2> = pts.into_iter().map(sl).collect();
    ink.strokes.push(synthetic_stroke(&body));
    // t-cross and i-dot
    ink.strokes.push(synthetic_stroke(&[pos2(196.0, 112.0), pos2(222.0, 108.0), pos2(240.0, 110.0)].map(sl)));
    ink.strokes.push(synthetic_stroke(&[pos2(330.0, 96.0), pos2(333.0, 94.0), pos2(334.0, 97.0)]));
    // underline flourish
    ink.strokes.push(synthetic_stroke(&[
        pos2(70.0, 176.0),
        pos2(160.0, 170.0),
        pos2(280.0, 166.0),
        pos2(400.0, 160.0),
        pos2(436.0, 152.0),
    ]));
    ink
}
