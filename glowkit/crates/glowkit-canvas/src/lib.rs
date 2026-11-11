//! # glowkit-canvas
//!
//! Canvas-style productivity components for egui.
pub mod common;
mod freehand;
pub mod graph;
mod node_editor;
pub mod json;
mod undo;
mod view;

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
