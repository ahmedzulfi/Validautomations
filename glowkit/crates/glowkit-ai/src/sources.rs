//! Source lists and inline citations.

use crate::common;
use crate::icons::{paint_icon, Icon};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Order, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind,
    Ui, Widget,
};
use glowkit_core::color::{hsv, mix, with_alpha};
use glowkit_core::Theme;

/// A web page the answer is based on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Source<'a> {
    /// Page title.
    pub title: &'a str,
    /// Full URL.
    pub url: &'a str,
    /// Optional excerpt shown in hover cards.
    pub snippet: &'a str,
}

impl<'a> Source<'a> {
    /// A source with a title and URL.
    pub const fn new(title: &'a str, url: &'a str) -> Self {
        Self { title, url, snippet: "" }
    }
    /// Add an excerpt.
    pub const fn snippet(mut self, s: &'a str) -> Self {
        self.snippet = s;
        self
    }
    /// The host part of the URL without `www.`.
    pub fn domain(&self) -> &'a str {
        let u = self.url;
        let u = u.split("://").nth(1).unwrap_or(u);
        let u = u.split('/').next().unwrap_or(u);
        u.strip_prefix("www.").unwrap_or(u)
    }
}

fn hash_str(s: &str) -> u32 {
    s.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619))
}

/// A tiny "favicon": a tinted circle with the domain's first letter.
pub(crate) fn favicon(p: &Painter, c: Pos2, r: f32, domain: &str, t: &Theme) {
    let hue = (hash_str(domain) % 360) as f32;
    let col = hsv(hue, if t.dark { 0.55 } else { 0.6 }, if t.dark { 0.85 } else { 0.7 });
    p.circle_filled(c, r, mix(t.card, col, if t.dark { 0.28 } else { 0.18 }));
    p.circle_stroke(c, r, Stroke::new(1.0, with_alpha(col, 0.45)));
    let letter: String = domain.chars().next().map(|c| c.to_ascii_uppercase()).into_iter().collect();
    p.text(c, Align2::CENTER_CENTER, letter, FontId::proportional(r * 1.05), col);
}

/// "Used 5 sources" with stacked favicons; expands into a list of linked pages.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{Source, Sources};
/// let sources = [Source::new("The Rust Book", "https://doc.rust-lang.org/book/")];
/// ui.add(Sources::new(&sources));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Sources<'a> {
    sources: &'a [Source<'a>],
    default_open: bool,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> Sources<'a> {
    /// The list of sources.
    pub fn new(sources: &'a [Source<'a>]) -> Self {
        Self { sources, default_open: false, id_salt: None }
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

impl Widget for Sources<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = common::widget_id(ui, self.id_salt);
        let (_, open) = common::toggle_state(ui, id, self.default_open, false);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let n = self.sources.len();
            let label = format!("Used {n} source{}", if n == 1 { "" } else { "s" });
            let g = common::galley(ui, label, 13.0, t.muted_foreground);
            let stack = n.min(4);
            let stack_w = if stack > 0 { 18.0 + (stack as f32 - 1.0) * 11.0 + 8.0 } else { 0.0 };
            let (r, resp) = ui.allocate_exact_size(vec2(stack_w + g.size().x + 26.0, 26.0), Sense::click());
            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.clicked() {
                common::toggle_state(ui, id, self.default_open, true);
            }
            let h = common::anim_bool(ui, id.with("hh"), resp.hovered(), 0.12);
            let col = t.muted_foreground.lerp_to_gamma(t.foreground, h * 0.8);
            let p = ui.painter();
            for (i, s) in self.sources.iter().take(stack).enumerate().rev() {
                let c = pos2(r.min.x + 9.0 + i as f32 * 11.0, r.center().y);
                p.circle_filled(c, 10.0, t.card);
                favicon(p, c, 8.5, s.domain(), &t);
            }
            p.galley(pos2(r.min.x + stack_w, r.center().y - g.size().y / 2.0), g.clone(), col);
            common::chevron(p, pos2(r.min.x + stack_w + g.size().x + 13.0, r.center().y), 9.0, col, open);
            let (_, open) = common::toggle_state(ui, id, self.default_open, false);
            common::reveal(ui, id.with("body"), open, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for (i, s) in self.sources.iter().enumerate() {
                    let w = ui.available_width().min(560.0);
                    let (row, rr) = ui.allocate_exact_size(vec2(w, 40.0), Sense::click());
                    let rr = rr.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(s.url);
                    let hh = common::anim_bool(ui, id.with(("row", i)), rr.hovered(), 0.1);
                    let p = ui.painter();
                    if hh > 0.0 {
                        p.rect_filled(row, CornerRadius::same(8), common::hover_fill(&t).gamma_multiply(hh));
                    }
                    favicon(p, pos2(row.min.x + 18.0, row.center().y), 9.0, s.domain(), &t);
                    let title = common::galley(ui, s.title, 13.0, t.foreground);
                    let tw = title.size().x.min(w - 200.0);
                    let clip =
                        Rect::from_min_max(pos2(row.min.x + 36.0, row.min.y), pos2(row.min.x + 36.0 + tw, row.max.y));
                    ui.painter().with_clip_rect(clip).galley(
                        pos2(row.min.x + 36.0, row.center().y - title.size().y / 2.0),
                        title,
                        t.foreground,
                    );
                    common::text(
                        ui.painter(),
                        pos2(row.min.x + 36.0 + tw + 10.0, row.center().y),
                        Align2::LEFT_CENTER,
                        s.domain(),
                        12.0,
                        t.muted_foreground,
                    );
                    paint_icon(
                        ui.painter(),
                        Icon::External,
                        Rect::from_center_size(pos2(row.max.x - 16.0, row.center().y), vec2(12.0, 12.0)),
                        with_alpha(t.muted_foreground, hh),
                    );
                    if rr.clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(s.url));
                    }
                }
                ui.add_space(4.0);
            });
        })
        .response
    }
}

/// An inline citation pill (`rust-lang.org +2`) that sits in a line of text and shows a hover
/// card with the source title, URL and excerpt; with several sources the card pages through them.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{InlineCitation, Source};
/// let src = [Source::new("Ownership", "https://doc.rust-lang.org/book/ch04-01.html")];
/// ui.horizontal(|ui| {
///     ui.label("Rust has no garbage collector");
///     ui.add(InlineCitation::new(&src));
/// });
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct InlineCitation<'a> {
    sources: &'a [Source<'a>],
    id_salt: Option<egui::IdSalt>,
}

impl<'a> InlineCitation<'a> {
    /// A citation for one or more sources.
    pub fn new(sources: &'a [Source<'a>]) -> Self {
        Self { sources, id_salt: None }
    }
    /// Stable id for the hover card state.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }
}

#[derive(Clone, Copy, Default)]
struct CardState {
    page: usize,
    card: Option<Rect>,
    last_inside: f64,
}

impl Widget for InlineCitation<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let Some(first) = self.sources.first() else {
            return ui.allocate_response(vec2(0.0, 0.0), Sense::hover());
        };
        let id = common::widget_id(ui, self.id_salt);
        let label = if self.sources.len() > 1 {
            format!("{} +{}", first.domain(), self.sources.len() - 1)
        } else {
            first.domain().to_owned()
        };
        let g = common::galley(ui, label, 11.5, t.muted_foreground);
        let (rect, resp) = ui.allocate_exact_size(vec2(g.size().x + 14.0, 20.0), Sense::click());
        let mut st: CardState = glowkit_core::state(ui, id.with("card"), CardState::default);
        let now = ui.input(|i| i.time);
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let inside = resp.hovered() || st.card.zip(pointer).is_some_and(|(c, p)| c.expand(6.0).contains(p));
        if inside {
            st.last_inside = now;
        }
        let show = now - st.last_inside < 0.15;
        let h = common::anim_bool(ui, id.with("hh"), resp.hovered() || show, 0.12);
        let p = ui.painter();
        p.rect_filled(rect, CornerRadius::same(10), mix(common::raised(&t), t.accent, h * 0.12));
        p.rect_stroke(
            rect,
            CornerRadius::same(10),
            Stroke::new(1.0, mix(t.border, t.accent, h * 0.5)),
            StrokeKind::Inside,
        );
        p.galley(
            pos2(rect.min.x + 7.0, rect.center().y - g.size().y / 2.0),
            g,
            t.muted_foreground.lerp_to_gamma(t.foreground, h),
        );
        if show {
            let n = self.sources.len();
            st.page = st.page.min(n - 1);
            let s = self.sources[st.page];
            let area = egui::Area::new(id.with("hover_card"))
                .order(Order::Tooltip)
                .fade_in(false)
                .fixed_pos(pos2(rect.min.x, rect.max.y + 6.0))
                .interactable(true)
                .show(ui.ctx(), |ui| {
                    egui::Frame::new()
                        .fill(t.popover)
                        .stroke(Stroke::new(1.0, t.border))
                        .corner_radius(t.corner())
                        .shadow(egui::Shadow {
                            offset: [0, 6],
                            blur: 24,
                            spread: 0,
                            color: Color32::from_black_alpha(if t.dark { 120 } else { 30 }),
                        })
                        .inner_margin(egui::Margin::same(14))
                        .show(ui, |ui| {
                            ui.set_width(300.0);
                            ui.spacing_mut().item_spacing.y = 6.0;
                            if n > 1 {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 2.0;
                                    if common::icon_button(ui, Icon::ChevronLeft, 22.0, None, "").clicked() {
                                        st.page = (st.page + n - 1) % n;
                                    }
                                    let g =
                                        common::galley(ui, format!("{}/{}", st.page + 1, n), 11.5, t.muted_foreground);
                                    let (r, _) = ui.allocate_exact_size(vec2(g.size().x + 8.0, 22.0), Sense::hover());
                                    ui.painter().galley(
                                        pos2(r.min.x + 4.0, r.center().y - g.size().y / 2.0),
                                        g,
                                        t.muted_foreground,
                                    );
                                    if common::icon_button(ui, Icon::ChevronRight, 22.0, None, "").clicked() {
                                        st.page = (st.page + 1) % n;
                                    }
                                });
                            }
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                                favicon(ui.painter(), r.center(), 8.0, s.domain(), &t);
                                ui.label(egui::RichText::new(s.domain()).size(12.0).color(t.muted_foreground));
                            });
                            let tg = common::galley_wrapped(ui, s.title, 13.5, t.foreground, 300.0);
                            let (r, _) = ui.allocate_exact_size(tg.size(), Sense::hover());
                            common::paint_galley_fx(ui.painter(), r.min, &tg, |_| common::Fx {
                                bold: true,
                                ..Default::default()
                            });
                            if !s.snippet.is_empty() {
                                let sg = common::galley_wrapped(ui, s.snippet, 12.5, t.muted_foreground, 300.0);
                                let (r, _) = ui.allocate_exact_size(sg.size(), Sense::hover());
                                ui.painter().galley(r.min, sg, t.muted_foreground);
                            }
                        });
                });
            st.card = Some(area.response.rect);
            ui.ctx().request_repaint();
        } else {
            st.card = None;
        }
        if resp.clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(first.url));
        }
        glowkit_core::set_state(ui, id.with("card"), st);
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}
