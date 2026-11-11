//! # glowkit-canvas
//!
//! Canvas-style productivity components for egui.
pub mod common;
mod freehand;
mod gantt;
mod grid;
mod kanban;
mod mindmap;
pub mod graph;
mod node_editor;
mod orgchart;
mod signature;
pub mod json;
mod timeline;
mod undo;
mod view;
mod whiteboard;

pub use common::Icon;
pub use freehand::{paint_ink, paint_ribbon, stroke_radii, InkPoint, InkStyle};
pub use undo::History;
pub use view::{fit_view, paint_background, Background, CanvasResponse, CanvasUi, CanvasView, ControlAction, Controls, InfiniteCanvas, MiniMap, WheelMode};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}
pub use graph::{ConnectError, Edge, EdgeAnim, EdgeId, EdgeKind, Graph, LayoutDirection, Node, NodeId, NodeShape, NodeStatus, Port, PortLayout, PortRef, PortSide, PortType};
pub use node_editor::{paint_node, shape_points, NodeEditor, NodeEditorResponse, NodeEditorState, PaletteItem};
pub use whiteboard::{paint_wb_shape, rough_segment, synthetic_stroke, Document, ShapeKind, Tool, WbShape, WbStyle, Whiteboard, WhiteboardState, INK, STICKY_COLORS, WB_COLORS};
pub use gantt::{civil_from_days, days_from_civil, weekday, Gantt, GanttChart, GanttGroup, GanttState, GanttTask, GanttZoom};
pub use kanban::{insertion_index, Board, Card, Column, KanbanBoard, KanbanState, Label};
pub use mindmap::{mind_layout, MindLayout, MindMap, MindMapEditor, MindMapState, MindNode};
pub use timeline::{ruler_step, snap_to, timecode, Clip, ClipKind, Timeline, TimelineEditor, TimelineState, Track};
pub use grid::{group_thousands, parse_number, CellKind, CellRange, DataGrid, GridColumn, GridData, GridState};
pub use orgchart::{tree_layout, OrgChart, OrgChartState, OrgChartView, OrgPerson};
pub use signature::{Ink, SignaturePad};
