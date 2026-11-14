//! Tool-approval card ("human in the loop").

use crate::code::CodeBlock;
use crate::common;
use crate::icons::{paint_icon, Icon};
use crate::media::{self, PillStyle};
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, Layout, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui,
    UiBuilder, Widget,
};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// Decision state of a [`Confirmation`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ApprovalState {
    /// Waiting for the user.
    #[default]
    Pending,
    /// The user approved the request.
    Approved,
    /// The user denied the request.
    Denied,
}

/// A tool-approval card: the agent asks before running a sensitive tool. Shows the tool, what it
/// will do (key / value details and an optional command or payload), and Deny / Approve buttons.
/// Once decided it settles into a compact resolved state with a colored status line. The decision
/// is written into the caller-owned [`ApprovalState`] and the response is marked changed.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{ApprovalState, Confirmation};
/// let mut state = ApprovalState::Pending;
/// ui.add(Confirmation::new(&mut state, "Run shell command?")
///     .tool("bash")
///     .detail("Directory", "~/projects/app")
///     .code("rm -rf node_modules && npm install"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Confirmation<'a> {
    state: &'a mut ApprovalState,
    title: &'a str,
    description: Option<&'a str>,
    tool: Option<&'a str>,
    details: Vec<(&'a str, &'a str)>,
    code: Option<(&'a str, &'a str)>,
    approve: &'a str,
    deny: &'a str,
    approved_text: &'a str,
    denied_text: &'a str,
    icon: Icon,
}

impl<'a> Confirmation<'a> {
    /// An approval request titled `title`, bound to `state`.
    pub fn new(state: &'a mut ApprovalState, title: &'a str) -> Self {
        Self {
            state,
            title,
            description: None,
            tool: None,
            details: Vec::new(),
            code: None,
            approve: "Approve",
            deny: "Deny",
            approved_text: "Approved: the tool ran",
            denied_text: "Denied: the agent will try another way",
            icon: Icon::Lock,
        }
    }
    /// Muted explanation under the title.
    pub fn description(mut self, d: &'a str) -> Self {
        self.description = Some(d);
        self
    }
    /// Tool name, shown as a monospace chip.
    pub fn tool(mut self, t: &'a str) -> Self {
        self.tool = Some(t);
        self
    }
    /// A labelled request detail (can be called repeatedly).
    pub fn detail(mut self, key: &'a str, value: &'a str) -> Self {
        self.details.push((key, value));
        self
    }
    /// The command / payload, highlighted as shell.
    pub fn code(mut self, code: &'a str) -> Self {
        self.code = Some((code, "bash"));
        self
    }
    /// The payload with an explicit highlight language (e.g. `"json"`).
    pub fn code_lang(mut self, code: &'a str, lang: &'a str) -> Self {
        self.code = Some((code, lang));
        self
    }
    /// Approve / deny button labels.
    pub fn labels(mut self, approve: &'a str, deny: &'a str) -> Self {
        self.approve = approve;
        self.deny = deny;
        self
    }
    /// Status line texts shown after approving / denying.
    pub fn resolved_texts(mut self, approved: &'a str, denied: &'a str) -> Self {
        self.approved_text = approved;
        self.denied_text = denied;
        self
    }
    /// Header icon (default: lock).
    pub fn icon(mut self, i: Icon) -> Self {
        self.icon = i;
        self
    }
}

impl Widget for Confirmation<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let width = ui.available_width();
        let top = ui.cursor().min;
        let state = *self.state;
        let (status_c, status_label) = match state {
            ApprovalState::Pending => (t.warning, "Approval required"),
            ApprovalState::Approved => (t.success, "Approved"),
            ApprovalState::Denied => (t.destructive, "Denied"),
        };
        let bg = ui.painter().add(Shape::Noop);
        let cid = ui.next_auto_id().with("confirmation");
        let mut decided = None;
        let resp = ui
            .scope_builder(UiBuilder::new().max_rect(Rect::from_min_size(top, vec2(width, f32::INFINITY))), |ui| {
                let pad = 16.0;
                let inner_w = width - pad * 2.0;
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.add_space(pad);
                // header row
                let (hr, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
                let p = ui.painter();
                let ib = Rect::from_min_size(pos2(hr.min.x + pad, hr.min.y + 2.0), vec2(32.0, 32.0));
                p.rect(
                    ib,
                    CornerRadius::same(8),
                    with_alpha(status_c, if t.dark { 0.14 } else { 0.1 }),
                    Stroke::new(1.0, with_alpha(status_c, 0.3)),
                    StrokeKind::Inside,
                );
                let icon = match state {
                    ApprovalState::Pending => self.icon,
                    ApprovalState::Approved => Icon::Check,
                    ApprovalState::Denied => Icon::X,
                };
                paint_icon(p, icon, ib.shrink(8.0), status_c);
                let tx = ib.max.x + 12.0;
                let title_r =
                    common::text(p, pos2(tx, hr.min.y + 10.0), Align2::LEFT_CENTER, self.title, 14.5, t.foreground);
                if let Some(d) = self.description {
                    common::text(
                        ui.painter(),
                        pos2(tx, hr.min.y + 28.0),
                        Align2::LEFT_CENTER,
                        d,
                        12.5,
                        t.muted_foreground,
                    );
                }
                // status badge
                let g = common::galley(ui, status_label, 11.5, status_c);
                let bw = g.size().x + 26.0;
                let br = Rect::from_min_size(pos2(hr.max.x - pad - bw, hr.min.y + 1.0), vec2(bw, 22.0));
                let p = ui.painter();
                p.rect(
                    br,
                    CornerRadius::same(11),
                    with_alpha(status_c, if t.dark { 0.12 } else { 0.09 }),
                    Stroke::new(1.0, with_alpha(status_c, 0.25)),
                    StrokeKind::Inside,
                );
                let dc = pos2(br.min.x + 11.0, br.center().y);
                if state == ApprovalState::Pending {
                    let ph = (glowkit_core::time(ui) * 2.2).sin() * 0.5 + 0.5;
                    p.circle_filled(dc, 3.0 + 3.0 * ph, with_alpha(status_c, 0.35 * (1.0 - ph)));
                    glowkit_core::animate(ui);
                }
                p.circle_filled(dc, 3.0, status_c);
                p.galley(pos2(br.min.x + 19.0, br.center().y - g.size().y / 2.0), g, status_c);
                if let Some(tool) = self.tool {
                    let g =
                        ui.painter().layout_no_wrap(tool.to_owned(), egui::FontId::monospace(11.5), t.muted_foreground);
                    let cr =
                        Rect::from_min_size(pos2(title_r.max.x + 8.0, hr.min.y + 1.0), vec2(g.size().x + 12.0, 19.0));
                    if cr.max.x < br.min.x - 8.0 {
                        ui.painter().rect(
                            cr,
                            CornerRadius::same(5),
                            common::raised(&t),
                            Stroke::new(1.0, t.border),
                            StrokeKind::Inside,
                        );
                        ui.painter().galley(
                            pos2(cr.min.x + 6.0, cr.center().y - g.size().y / 2.0),
                            g,
                            t.muted_foreground,
                        );
                    }
                }

                // body (details + code); collapses a little once resolved
                let open = common::anim_bool(ui, cid.with("open"), state == ApprovalState::Pending, 0.25);
                let open = glowkit_core::ease(glowkit_core::Easing::OutCubic, open);
                let body_id = cid.with("body");
                ui.add_space(12.0);
                common::reveal(ui, body_id, open.max(0.0), |ui| {
                    let r = ui.available_rect_before_wrap();
                    let inner =
                        Rect::from_min_max(pos2(r.min.x + pad, r.min.y), pos2(r.min.x + pad + inner_w, r.max.y));
                    ui.scope_builder(UiBuilder::new().max_rect(inner), |ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        if !self.details.is_empty() {
                            let row_h = 26.0;
                            let (dr, _) = ui.allocate_exact_size(
                                vec2(inner_w, row_h * self.details.len() as f32 + 12.0),
                                Sense::hover(),
                            );
                            let p = ui.painter();
                            p.rect(
                                dr,
                                t.corner_scaled(0.8),
                                common::raised(&t),
                                Stroke::new(1.0, t.border),
                                StrokeKind::Inside,
                            );
                            for (i, (k, v)) in self.details.iter().enumerate() {
                                let y = dr.min.y + 6.0 + row_h * (i as f32 + 0.5);
                                common::text(
                                    p,
                                    pos2(dr.min.x + 12.0, y),
                                    Align2::LEFT_CENTER,
                                    *k,
                                    12.5,
                                    t.muted_foreground,
                                );
                                common::text(p, pos2(dr.min.x + 130.0, y), Align2::LEFT_CENTER, *v, 12.5, t.foreground);
                                if i + 1 < self.details.len() {
                                    let ly = y + row_h / 2.0;
                                    p.line_segment(
                                        [pos2(dr.min.x + 12.0, ly), pos2(dr.max.x - 12.0, ly)],
                                        Stroke::new(1.0, with_alpha(t.border, 0.7)),
                                    );
                                }
                            }
                        }
                        if let Some((code, lang)) = self.code {
                            ui.add(CodeBlock::new(code).language(lang).header(false).size(12.0));
                        }
                    });
                    ui.add_space(14.0);
                });
                // footer
                let line_y = ui.cursor().min.y;
                if state == ApprovalState::Pending {
                    ui.painter().line_segment(
                        [pos2(top.x + 1.0, line_y), pos2(top.x + width - 1.0, line_y)],
                        Stroke::new(1.0, t.border),
                    );
                    ui.add_space(12.0);
                    let fr = Rect::from_min_size(pos2(top.x + pad, ui.cursor().min.y), vec2(inner_w, 32.0));
                    ui.scope_builder(
                        UiBuilder::new().max_rect(fr).layout(Layout::right_to_left(Align::Center)),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            if media::pill_button(ui, self.approve, Some(Icon::Check), PillStyle::Primary, 32.0)
                                .clicked()
                            {
                                decided = Some(ApprovalState::Approved);
                            }
                            if media::pill_button(ui, self.deny, Some(Icon::X), PillStyle::Outline, 32.0).clicked() {
                                decided = Some(ApprovalState::Denied);
                            }
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                ui.label(
                                    egui::RichText::new("The agent is paused until you decide.")
                                        .size(12.0)
                                        .color(t.muted_foreground),
                                );
                            });
                        },
                    );
                    ui.add_space(pad);
                } else {
                    let text = if state == ApprovalState::Approved { self.approved_text } else { self.denied_text };
                    let fr = Rect::from_min_size(pos2(top.x + pad, line_y), vec2(inner_w, 34.0));
                    let p = ui.painter();
                    p.rect_filled(fr, t.corner_scaled(0.7), with_alpha(status_c, if t.dark { 0.08 } else { 0.06 }));
                    let ic = pos2(fr.min.x + 16.0, fr.center().y);
                    paint_icon(
                        p,
                        if state == ApprovalState::Approved { Icon::Check } else { Icon::X },
                        Rect::from_center_size(ic, vec2(13.0, 13.0)),
                        status_c,
                    );
                    common::text(
                        p,
                        pos2(fr.min.x + 30.0, fr.center().y),
                        Align2::LEFT_CENTER,
                        text,
                        12.5,
                        t.foreground,
                    );
                    // undo
                    let ur = Rect::from_min_size(pos2(fr.max.x - 64.0, fr.min.y + 5.0), vec2(56.0, 24.0));
                    let uresp = ui
                        .interact(ur, cid.with("undo"), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    let uh = common::anim_bool(ui, uresp.id.with("h"), uresp.hovered(), 0.1);
                    ui.painter().rect_filled(ur, CornerRadius::same(6), common::hover_fill(&t).gamma_multiply(uh));
                    paint_icon(
                        ui.painter(),
                        Icon::History,
                        Rect::from_center_size(pos2(ur.min.x + 13.0, ur.center().y), vec2(12.0, 12.0)),
                        t.muted_foreground,
                    );
                    common::text(
                        ui.painter(),
                        pos2(ur.min.x + 23.0, ur.center().y),
                        Align2::LEFT_CENTER,
                        "Undo",
                        12.0,
                        t.muted_foreground,
                    );
                    if uresp.clicked() {
                        decided = Some(ApprovalState::Pending);
                    }
                    ui.allocate_exact_size(vec2(width, 34.0 + pad), Sense::hover());
                }
            })
            .response;
        let rect = Rect::from_min_size(top, vec2(width, resp.rect.height()));
        let cr = t.corner_scaled(1.1);
        let mut shapes = Vec::new();
        if !t.dark {
            shapes.push(Shape::from(
                egui::epaint::Shadow { offset: [0, 2], blur: 10, spread: 0, color: Color32::from_black_alpha(12) }
                    .as_shape(rect, cr),
            ));
        }
        shapes.push(Shape::rect_filled(rect, cr, t.card));
        ui.painter().set(bg, Shape::Vec(shapes));
        let border = if state == ApprovalState::Pending { with_alpha(t.warning, 0.45) } else { t.border };
        ui.painter().rect_stroke(rect, cr, Stroke::new(1.0, border), StrokeKind::Inside);
        let mut resp = resp;
        if let Some(d) = decided {
            *self.state = d;
            resp.mark_changed();
        }
        resp
    }
}
