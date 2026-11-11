//! # glowkit-canvas
//!
//! Canvas-style productivity components for egui: the Rust take on React Flow / xyflow,
//! tldraw / Excalidraw and friends. Every component keeps its data in caller-owned structs
//! (so you can save, diff, sync or undo them) plus a small caller-owned `*State` for view and
//! interaction state, and works natively and on WebAssembly.
//!
//! | Component | Data | State | Pattern |
//! |---|---|---|---|
//! | [`InfiniteCanvas`] | your own | [`CanvasView`] | pan / zoom surface, dot / line / cross backgrounds, [`MiniMap`], [`Controls`] |
//! | [`NodeEditor`] | [`Graph`] | [`NodeEditorState`] | React Flow: nodes, typed ports, edges, minimap, undo, auto-layout |
//! | [`Whiteboard`] | [`Document`] | [`WhiteboardState`] | tldraw / Excalidraw: pen, shapes, arrows, sticky notes, JSON |
//! | [`KanbanBoard`] | [`Board`] | [`KanbanState`] | Trello / Linear board with spring drag and drop |
//! | [`GanttChart`] | [`Gantt`] | [`GanttState`] | project timeline with dependencies |
//! | [`MindMapEditor`] | [`MindMap`] | [`MindMapState`] | auto-laid-out mind map |
//! | [`TimelineEditor`] | [`Timeline`] | [`TimelineState`] | video-editor tracks and clips |
//! | [`DataGrid`] | [`GridData`] | [`GridState`] | virtualized spreadsheet |
//! | [`OrgChartView`] | [`OrgChart`] | [`OrgChartState`] | org chart / tree diagram |
//! | [`SignaturePad`] | [`Ink`] | — | signature / drawing pad |
//!
//! Pure helpers are public and unit-tested: [`fit_view`], [`Graph::layered_positions`],
//! [`edge_path`], [`mind_layout`], [`tree_layout`], [`stroke_radii`], [`History`],
//! [`Document::to_json`] / [`Document::from_json`], calendar math ([`days_from_civil`]).
//!
//! ```no_run
//! # let ui: &mut egui::Ui = todo!();
//! use glowkit_canvas::{Graph, Node, NodeEditor, NodeEditorState, Port};
//! # let (mut graph, mut state) = (Graph::new(), NodeEditorState::default());
//! let a = graph.add(Node::new("Trigger", egui::pos2(0.0, 0.0)).output(Port::new("out")));
//! let b = graph.add(Node::new("Action", egui::pos2(260.0, 0.0)).input(Port::new("in")));
//! graph.link((a, 0), (b, 0));
//! NodeEditor::new(&mut graph, &mut state).height(480.0).show(ui);
//! ```
pub mod common;
mod freehand;
mod gantt;
pub mod graph;
mod grid;
pub mod json;
mod kanban;
mod mindmap;
mod node_editor;
mod orgchart;
mod signature;
mod timeline;
mod undo;
mod view;
mod whiteboard;

pub use common::Icon;
pub use freehand::{paint_ink, paint_ribbon, stroke_radii, InkPoint, InkStyle};
pub use gantt::{civil_from_days, days_from_civil, weekday, Gantt, GanttChart, GanttGroup, GanttState, GanttTask, GanttZoom};
pub use graph::{edge_path, ConnectError, Edge, EdgeAnim, EdgeId, EdgeKind, Graph, LayoutDirection, Node, NodeId, NodeShape, NodeStatus, Port, PortLayout, PortRef, PortSide, PortType};
pub use grid::{group_thousands, parse_number, CellKind, CellRange, DataGrid, GridColumn, GridData, GridState};
pub use kanban::{insertion_index, Board, Card, Column, KanbanBoard, KanbanState, Label};
pub use mindmap::{mind_layout, MindLayout, MindMap, MindMapEditor, MindMapState, MindNode};
pub use node_editor::{paint_node, shape_points, NodeEditor, NodeEditorResponse, NodeEditorState, PaletteItem};
pub use orgchart::{tree_layout, OrgChart, OrgChartState, OrgChartView, OrgPerson};
pub use signature::{Ink, SignaturePad};
pub use timeline::{ruler_step, snap_to, timecode, Clip, ClipKind, Timeline, TimelineEditor, TimelineState, Track};
pub use undo::History;
pub use view::{fit_view, paint_background, Background, CanvasResponse, CanvasUi, CanvasView, ControlAction, Controls, InfiniteCanvas, MiniMap, WheelMode};
pub use whiteboard::{paint_wb_shape, rough_segment, synthetic_stroke, Document, ShapeKind, Tool, WbShape, WbStyle, Whiteboard, WhiteboardState, INK, STICKY_COLORS, WB_COLORS};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
