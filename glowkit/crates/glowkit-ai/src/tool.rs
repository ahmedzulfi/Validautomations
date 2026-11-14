//! Tool / function call card.

use crate::code::CodeBlock;
use crate::common;
use crate::icons::{paint_icon, Icon};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, Widget,
};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// Lifecycle of a [`ToolCall`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolStatus {
    /// Waiting to run (or awaiting approval).
    #[default]
    Pending,
    /// Executing: badge shows a spinner.
    Running,
    /// Finished successfully.
    Completed,
    /// Failed.
    Error,
}

impl ToolStatus {
    fn label(self) -> &'static str {
        match self {
            ToolStatus::Pending => "Pending",
            ToolStatus::Running => "Running",
            ToolStatus::Completed => "Completed",
            ToolStatus::Error => "Error",
        }
    }
    fn color(self, t: &Theme) -> Color32 {
        match self {
            ToolStatus::Pending => t.muted_foreground,
            ToolStatus::Running => t.info,
            ToolStatus::Completed => t.success,
            ToolStatus::Error => t.destructive,
        }
    }
}

/// A card for a tool (function) call: tool name, a status badge (pending, running with spinner,
/// completed, error) and collapsible parameters and result.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{ToolCall, ToolStatus};
/// ui.add(ToolCall::new("search_web")
///     .status(ToolStatus::Completed)
///     .input("{\n  \"query\": \"egui 0.37\"\n}")
///     .output("{ \"results\": 12 }"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ToolCall<'a> {
    name: &'a str,
    status: ToolStatus,
    input: Option<&'a str>,
    output: Option<&'a str>,
    error: Option<&'a str>,
    description: Option<&'a str>,
    icon: Icon,
    default_open: bool,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> ToolCall<'a> {
    /// A call to the tool named `name`.
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            status: ToolStatus::Pending,
            input: None,
            output: None,
            error: None,
            description: None,
            icon: Icon::Gear,
            default_open: false,
            id_salt: None,
        }
    }
    /// Current status.
    pub fn status(mut self, s: ToolStatus) -> Self {
        self.status = s;
        self
    }
    /// Parameters (JSON or any text; JSON gets highlighted).
    pub fn input(mut self, input: &'a str) -> Self {
        self.input = Some(input);
        self
    }
    /// Result.
    pub fn output(mut self, output: &'a str) -> Self {
        self.output = Some(output);
        self
    }
    /// Error message (shown instead of the result).
    pub fn error(mut self, error: &'a str) -> Self {
        self.error = Some(error);
        self
    }
    /// Short muted description next to the name.
    pub fn description(mut self, d: &'a str) -> Self {
        self.description = Some(d);
        self
    }
    /// Icon in the header.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = icon;
        self
    }
    /// Expanded when first shown.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }
    /// Stable id for the open state.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }
}

fn section_label(ui: &mut Ui, text: &str) {
    let t = Theme::of(ui);
    let g = ui.painter().layout_no_wrap(text.to_owned(), egui::FontId::proportional(11.0), t.muted_foreground);
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), g.size().y + 2.0), Sense::hover());
    common::paint_galley_fx(ui.painter(), r.min, &g, |_| common::Fx {
        bold: true,
        color: Some(with_alpha(t.muted_foreground, 0.85)),
        ..Default::default()
    });
}

impl Widget for ToolCall<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = common::widget_id(ui, self.id_salt);
        let (_, open) = common::toggle_state(ui, id, self.default_open, false);
        let width = ui.available_width();
        let top = ui.cursor().min;
        let cr = t.corner_scaled(0.9);
        // background goes behind everything: reserve a shape slot
        let bg = ui.painter().add(egui::Shape::Noop);
        let resp = ui
            .scope_builder(UiBuilder::new().max_rect(Rect::from_min_size(top, vec2(width, f32::INFINITY))), |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                // header
                let (hr, hresp) = ui.allocate_exact_size(vec2(width, 46.0), Sense::click());
                let hresp = hresp.on_hover_cursor(egui::CursorIcon::PointingHand);
                if hresp.clicked() {
                    common::toggle_state(ui, id, self.default_open, true);
                }
                let hov = common::anim_bool(ui, id.with("hh"), hresp.hovered(), 0.12);
                let p = ui.painter();
                if hov > 0.0 {
                    let r = if open > 0.01 { CornerRadius { nw: cr.nw, ne: cr.ne, sw: 0, se: 0 } } else { cr };
                    p.rect_filled(hr.shrink(1.0), r, common::hover_fill(&t).gamma_multiply(hov * 0.6));
                }
                let ib = Rect::from_center_size(pos2(hr.min.x + 25.0, hr.center().y), vec2(26.0, 26.0));
                p.rect_filled(ib, CornerRadius::same(7), common::raised(&t));
                p.rect_stroke(ib, CornerRadius::same(7), Stroke::new(1.0, t.border), StrokeKind::Inside);
                paint_icon(p, self.icon, ib.shrink(6.5), t.muted_foreground);
                let name_g =
                    ui.painter().layout_no_wrap(self.name.to_owned(), egui::FontId::monospace(13.0), t.foreground);
                let nx = ib.max.x + 11.0;
                ui.painter().galley(pos2(nx, hr.center().y - name_g.size().y / 2.0), name_g.clone(), t.foreground);
                if let Some(d) = self.description {
                    common::text(
                        ui.painter(),
                        pos2(nx + name_g.size().x + 10.0, hr.center().y),
                        Align2::LEFT_CENTER,
                        d,
                        12.5,
                        t.muted_foreground,
                    );
                }
                // status badge + chevron on the right
                let chev_c = pos2(hr.max.x - 20.0, hr.center().y);
                common::chevron(ui.painter(), chev_c, 9.0, t.muted_foreground, open);
                let sc = self.status.color(&t);
                let label = self.status.label();
                let g = common::galley(ui, label, 11.5, sc);
                let bw = g.size().x + 30.0;
                let br = Rect::from_min_size(pos2(chev_c.x - 18.0 - bw, hr.center().y - 11.0), vec2(bw, 22.0));
                let p = ui.painter();
                p.rect_filled(br, CornerRadius::same(11), with_alpha(sc, if t.dark { 0.12 } else { 0.09 }));
                p.rect_stroke(br, CornerRadius::same(11), Stroke::new(1.0, with_alpha(sc, 0.22)), StrokeKind::Inside);
                let ic = pos2(br.min.x + 12.0, br.center().y);
                match self.status {
                    ToolStatus::Running => {
                        common::spinner(p, ic, 4.6, 1.5, sc, glowkit_core::time(ui));
                        glowkit_core::animate(ui);
                    }
                    ToolStatus::Completed => {
                        paint_icon(p, Icon::Check, Rect::from_center_size(ic, vec2(11.0, 11.0)), sc)
                    }
                    ToolStatus::Error => paint_icon(p, Icon::X, Rect::from_center_size(ic, vec2(10.0, 10.0)), sc),
                    ToolStatus::Pending => paint_icon(p, Icon::Clock, Rect::from_center_size(ic, vec2(11.0, 11.0)), sc),
                }
                p.galley(pos2(br.min.x + 21.0, br.center().y - g.size().y / 2.0), g, sc);
                // body
                common::reveal(ui, id.with("body"), open, |ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    let r = ui.available_rect_before_wrap();
                    ui.painter().line_segment(
                        [pos2(r.min.x + 1.0, r.min.y), pos2(r.max.x - 1.0, r.min.y)],
                        Stroke::new(1.0, t.border),
                    );
                    let inner = Rect::from_min_max(r.min + vec2(14.0, 12.0), pos2(r.max.x - 14.0, r.max.y));
                    ui.scope_builder(UiBuilder::new().max_rect(inner), |ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        if let Some(input) = self.input {
                            section_label(ui, "PARAMETERS");
                            ui.add(CodeBlock::new(input).language("json").header(false).size(12.0));
                        }
                        if let Some(err) = self.error {
                            section_label(ui, "ERROR");
                            let w = ui.available_width();
                            let g = common::galley_wrapped(ui, err, 12.5, t.destructive, w - 24.0);
                            let (er, _) = ui.allocate_exact_size(vec2(w, g.size().y + 20.0), Sense::hover());
                            ui.painter().rect_filled(er, t.corner_scaled(0.8), with_alpha(t.destructive, 0.08));
                            ui.painter().rect_stroke(
                                er,
                                t.corner_scaled(0.8),
                                Stroke::new(1.0, with_alpha(t.destructive, 0.25)),
                                StrokeKind::Inside,
                            );
                            ui.painter().galley(er.min + vec2(12.0, 10.0), g, t.destructive);
                        } else if let Some(out) = self.output {
                            section_label(ui, "RESULT");
                            ui.add(CodeBlock::new(out).language("json").header(false).size(12.0));
                        } else if self.status == ToolStatus::Running {
                            ui.add(crate::ShimmerText::new("Waiting for result…").size(12.5));
                        }
                        ui.add_space(6.0);
                    });
                });
            })
            .response;
        let rect = Rect::from_min_size(top, vec2(width, resp.rect.height()));
        ui.painter().set(bg, egui::Shape::rect_filled(rect, cr, t.card));
        ui.painter().rect_stroke(rect, cr, Stroke::new(1.0, t.border), StrokeKind::Inside);
        resp
    }
}
