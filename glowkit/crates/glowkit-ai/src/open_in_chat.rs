//! "Open in chat" dropdown button.

use crate::common;
use crate::icons::{paint_icon, Icon};
use crate::media;
use crate::misc::provider_badge;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, Order, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_core::Theme;

/// A chat app a prompt can be opened in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChatProvider<'a> {
    /// Display name.
    pub name: &'a str,
    /// Muted hint (domain, model family).
    pub hint: &'a str,
}

impl<'a> ChatProvider<'a> {
    /// A provider called `name` with a muted `hint`.
    pub fn new(name: &'a str, hint: &'a str) -> Self {
        Self { name, hint }
    }
}

/// An "Open in chat" dropdown button: clicking it lists chat apps (each with a monogram badge,
/// a hint and an external-link arrow) so the user can continue the current prompt elsewhere.
/// The prompt itself is shown truncated at the top of the menu, with a copy row at the bottom.
/// `show` returns the index of the provider picked this frame.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{ChatProvider, OpenInChat};
/// let providers = [ChatProvider::new("Lumen Chat", "lumen.chat"), ChatProvider::new("Orbit", "orbit.ai")];
/// if let Some(i) = OpenInChat::new("Explain ownership in Rust", &providers).show(ui) {
///     // open providers[i] with the prompt
/// }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct OpenInChat<'a> {
    prompt: &'a str,
    providers: &'a [ChatProvider<'a>],
    label: &'a str,
    open: Option<bool>,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> OpenInChat<'a> {
    /// A dropdown offering to open `prompt` in one of `providers`.
    pub fn new(prompt: &'a str, providers: &'a [ChatProvider<'a>]) -> Self {
        Self { prompt, providers, label: "Open in chat", open: None, id_salt: None }
    }
    /// Button label (default "Open in chat").
    pub fn label(mut self, l: &'a str) -> Self {
        self.label = l;
        self
    }
    /// Force the menu open or closed (e.g. for previews); `None` lets clicks control it.
    pub fn force_open(mut self, open: Option<bool>) -> Self {
        self.open = open;
        self
    }
    /// Stable id for the open state.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the button (and its menu when open). Returns the provider picked this frame;
    /// `Some(providers.len())` means "Copy prompt" was chosen.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = Theme::of(ui);
        let id = self
            .id_salt
            .map(|s| ui.scope_id().with_salt(s))
            .unwrap_or_else(|| ui.scope_id().with("glowkit_open_in_chat"));
        let g = common::galley(ui, self.label, 13.0, t.foreground);
        let (rect, _) = ui.allocate_exact_size(vec2(g.size().x + 64.0, 34.0), Sense::hover());
        let resp = ui.interact(rect, id.with("btn"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let mut open: bool = glowkit_core::state(ui, id.with("open"), || false);
        let toggled = resp.clicked();
        if toggled {
            open = !open;
        }
        if let Some(f) = self.open {
            open = f;
        }
        let h = common::anim_bool(ui, id.with("h"), resp.hovered() || open, 0.12);
        let p = ui.painter();
        let cr = CornerRadius::same((t.radius * 0.8) as u8);
        if !t.dark {
            glowkit_core::paint::soft_shadow(p, rect, cr.nw as f32, 2.0, vec2(0.0, 1.0), Color32::from_black_alpha(10));
        }
        p.rect(rect, cr, t.card, Stroke::new(1.0, common::border_strong(&t).lerp_to_gamma(t.muted_foreground, 0.2 * h)), StrokeKind::Inside);
        if h > 0.0 {
            p.rect_filled(rect, cr, common::hover_fill(&t).gamma_multiply(h));
        }
        glowkit_core::paint::sparkle(p, pos2(rect.min.x + 18.0, rect.center().y), 7.0, 0.0, t.accent);
        p.galley(pos2(rect.min.x + 32.0, rect.center().y - g.size().y / 2.0), g, t.foreground);
        // split divider + chevron
        let dx = rect.max.x - 30.0;
        p.line_segment([pos2(dx, rect.min.y + 7.0), pos2(dx, rect.max.y - 7.0)], Stroke::new(1.0, t.border));
        let ot = common::anim_bool(ui, id.with("o"), open, 0.15);
        let cc = pos2(rect.max.x - 15.0, rect.center().y);
        let flip = 1.0 - 2.0 * ot;
        p.add(egui::Shape::line(vec![cc + vec2(-4.0, -2.0 * flip), cc + vec2(0.0, 2.0 * flip), cc + vec2(4.0, -2.0 * flip)], Stroke::new(1.5, t.muted_foreground)));

        let mut picked = None;
        if open || ot > 0.01 {
            let width = 280.0;
            let slide = (1.0 - ot) * 6.0;
            let area = egui::Area::new(id.with("menu"))
                .order(Order::Foreground)
                .fade_in(false)
                .fixed_pos(pos2(rect.min.x, rect.max.y + 6.0 - slide))
                .interactable(open)
                .show(ui.ctx(), |ui| {
                    ui.multiply_opacity(ot);
                    media::popover_frame(&t).show(ui, |ui| {
                        ui.set_width(width);
                        ui.spacing_mut().item_spacing.y = 2.0;
                        // prompt preview
                        let (pr, _) = ui.allocate_exact_size(vec2(width, 44.0), Sense::hover());
                        common::text(ui.painter(), pos2(pr.min.x + 10.0, pr.min.y + 12.0), Align2::LEFT_CENTER, "PROMPT", 10.5, t.muted_foreground);
                        let mut s: String = self.prompt.chars().take(38).collect();
                        if self.prompt.chars().count() > 38 {
                            s.push('…');
                        }
                        common::text(ui.painter(), pos2(pr.min.x + 10.0, pr.min.y + 31.0), Align2::LEFT_CENTER, format!("“{s}”"), 12.5, t.foreground);
                        separator(ui, width, &t);
                        for (i, prov) in self.providers.iter().enumerate() {
                            if row(ui, id.with(("row", i)), width, &t, true, |p, r| {
                                provider_badge(p, pos2(r.min.x + 20.0, r.center().y), 24.0, prov.name, &t);
                                common::text(p, pos2(r.min.x + 42.0, r.min.y + 13.0), Align2::LEFT_CENTER, format!("Open in {}", prov.name), 13.0, t.foreground);
                                common::text(p, pos2(r.min.x + 42.0, r.min.y + 30.0), Align2::LEFT_CENTER, prov.hint, 11.5, t.muted_foreground);
                            }) {
                                picked = Some(i);
                            }
                        }
                        separator(ui, width, &t);
                        if row(ui, id.with("copy"), width, &t, false, |p, r| {
                            let ir = Rect::from_center_size(pos2(r.min.x + 20.0, r.center().y), vec2(15.0, 15.0));
                            paint_icon(p, Icon::Copy, ir, t.muted_foreground);
                            common::text(p, pos2(r.min.x + 42.0, r.center().y), Align2::LEFT_CENTER, "Copy prompt", 13.0, t.foreground);
                        }) {
                            ui.ctx().copy_text(self.prompt.to_owned());
                            picked = Some(self.providers.len());
                        }
                    });
                });
            open = media::close_on_outside(ui, open, toggled, &[area.response.rect, rect]);
            if picked.is_some() {
                open = false;
            }
        }
        if self.open.is_none() {
            glowkit_core::set_state(ui, id.with("open"), open);
        }
        picked
    }
}

fn separator(ui: &mut Ui, width: f32, t: &Theme) {
    let (r, _) = ui.allocate_exact_size(vec2(width, 7.0), Sense::hover());
    ui.painter().line_segment([pos2(r.min.x - 6.0, r.center().y), pos2(r.max.x + 6.0, r.center().y)], Stroke::new(1.0, t.border));
}

fn row(ui: &mut Ui, id: egui::Id, width: f32, t: &Theme, ext: bool, paint: impl FnOnce(&egui::Painter, Rect)) -> bool {
    let (r, _) = ui.allocate_exact_size(vec2(width, 44.0), Sense::hover());
    let rr = ui.interact(r, id, Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    let hh = common::anim_bool(ui, id.with("h"), rr.hovered(), 0.08);
    let p = ui.painter();
    if hh > 0.0 {
        p.rect_filled(r, CornerRadius::same(8), common::hover_fill(t).gamma_multiply(hh));
    }
    paint(p, r);
    if !ext {
        return rr.clicked();
    }
    let ec = pos2(r.max.x - 18.0 + 2.0 * hh, r.center().y);
    paint_icon(p, Icon::External, Rect::from_center_size(ec, vec2(13.0, 13.0)), t.muted_foreground.lerp_to_gamma(t.foreground, hh).gamma_multiply(0.4 + 0.6 * hh));
    rr.clicked()
}
