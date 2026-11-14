//! Artifact panel and generated-image placeholder.

use crate::common;
use crate::icons::{paint_icon, Icon};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, TextureId, Ui, UiBuilder,
    Widget,
};
use glowkit_core::color::{gradient, mix, with_alpha};
use glowkit_core::noise::fbm3;
use glowkit_core::{paint, Theme};

/// What happened in an [`Artifact`] header this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArtifactOutput {
    /// Index of the selected tab.
    pub tab: usize,
    /// Index into the actions slice of an action button clicked this frame.
    pub action: Option<usize>,
}

/// A titled panel for generated content (a document, a component, a chart) with segmented tabs
/// such as Preview / Code and a row of action buttons, like the artifact side panel of AI apps.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{Artifact, Icon};
/// Artifact::new("Pricing table")
///     .subtitle("React component · v2")
///     .tabs(&["Preview", "Code"])
///     .actions(&[(Icon::Copy, "Copy"), (Icon::Download, "Download")])
///     .show(ui, |ui, tab| {
///         ui.label(if tab == 0 { "preview" } else { "code" });
///     });
/// ```
#[must_use = "call `.show(ui, ..)`"]
pub struct Artifact<'a> {
    title: &'a str,
    subtitle: Option<&'a str>,
    icon: Icon,
    tabs: &'a [&'a str],
    actions: &'a [(Icon, &'a str)],
    height: Option<f32>,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> Artifact<'a> {
    /// A panel titled `title`.
    pub fn new(title: &'a str) -> Self {
        Self { title, subtitle: None, icon: Icon::File, tabs: &[], actions: &[], height: None, id_salt: None }
    }
    /// Muted line under the title.
    pub fn subtitle(mut self, s: &'a str) -> Self {
        self.subtitle = Some(s);
        self
    }
    /// Header icon.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = icon;
        self
    }
    /// Segmented tabs (e.g. Preview / Code). The selection is remembered.
    pub fn tabs(mut self, tabs: &'a [&'a str]) -> Self {
        self.tabs = tabs;
        self
    }
    /// Icon buttons on the right of the header (icon, tooltip).
    pub fn actions(mut self, actions: &'a [(Icon, &'a str)]) -> Self {
        self.actions = actions;
        self
    }
    /// Fixed body height (content scrolls). Default: fit content.
    pub fn height(mut self, h: f32) -> Self {
        self.height = Some(h);
        self
    }
    /// Stable id for the selected tab.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the panel; `body(ui, tab)` fills it for the selected tab.
    pub fn show(self, ui: &mut Ui, body: impl FnOnce(&mut Ui, usize)) -> ArtifactOutput {
        let t = Theme::of(ui);
        let id = common::widget_id(ui, self.id_salt);
        let mut tab: usize = glowkit_core::state(ui, id.with("tab"), || 0);
        let mut out = ArtifactOutput::default();
        let width = ui.available_width();
        let top = ui.cursor().min;
        let cr = t.corner_scaled(1.2);
        let bg = ui.painter().add(egui::Shape::Noop);
        let resp =
            ui.scope_builder(UiBuilder::new().max_rect(Rect::from_min_size(top, vec2(width, f32::INFINITY))), |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let (hr, _) = ui.allocate_exact_size(vec2(width, 58.0), Sense::hover());
                let p = ui.painter();
                p.rect_filled(hr, CornerRadius { nw: cr.nw, ne: cr.ne, sw: 0, se: 0 }, common::raised(&t));
                p.line_segment([pos2(hr.min.x, hr.max.y), pos2(hr.max.x, hr.max.y)], Stroke::new(1.0, t.border));
                let ib = Rect::from_center_size(pos2(hr.min.x + 30.0, hr.center().y), vec2(32.0, 32.0));
                paint::gradient_rect(
                    p,
                    ib,
                    8.0,
                    135.0,
                    &[with_alpha(t.gradient[0], 0.9), with_alpha(t.gradient[1], 0.9)],
                );
                paint_icon(p, self.icon, ib.shrink(8.0), Color32::WHITE);
                let tx = ib.max.x + 12.0;
                let tg = common::galley(ui, self.title, 14.0, t.foreground);
                let ty = if self.subtitle.is_some() {
                    hr.center().y - tg.size().y + 1.0
                } else {
                    hr.center().y - tg.size().y / 2.0
                };
                common::paint_galley_fx(ui.painter(), pos2(tx, ty), &tg, |_| common::Fx {
                    bold: true,
                    ..Default::default()
                });
                if let Some(s) = self.subtitle {
                    common::text(
                        ui.painter(),
                        pos2(tx, hr.center().y + 3.0),
                        Align2::LEFT_TOP,
                        s,
                        12.0,
                        t.muted_foreground,
                    );
                }
                // actions, right to left
                let mut x = hr.max.x - 12.0;
                for (i, (icon, tip)) in self.actions.iter().enumerate().rev() {
                    let r = Rect::from_min_size(pos2(x - 30.0, hr.center().y - 15.0), vec2(30.0, 30.0));
                    let mut child = ui.new_child(UiBuilder::new().max_rect(r));
                    if common::icon_button(&mut child, *icon, 30.0, None, tip).clicked() {
                        out.action = Some(i);
                    }
                    x -= 32.0;
                }
                // segmented tabs
                if !self.tabs.is_empty() {
                    let gs: Vec<_> = self.tabs.iter().map(|s| common::galley(ui, *s, 12.5, t.foreground)).collect();
                    let tw: f32 = gs.iter().map(|g| g.size().x + 24.0).sum::<f32>() + 6.0;
                    let seg = Rect::from_min_size(pos2(x - 10.0 - tw, hr.center().y - 16.0), vec2(tw, 32.0));
                    let p = ui.painter();
                    p.rect_filled(
                        seg,
                        CornerRadius::same(9),
                        if t.dark { with_alpha(t.foreground, 0.06) } else { t.muted },
                    );
                    let mut sx = seg.min.x + 3.0;
                    let mut rects = Vec::new();
                    for g in &gs {
                        let r = Rect::from_min_size(pos2(sx, seg.min.y + 3.0), vec2(g.size().x + 24.0, 26.0));
                        rects.push(r);
                        sx = r.max.x;
                    }
                    tab = tab.min(rects.len() - 1);
                    let target = rects[tab];
                    let ax = glowkit_core::anim::follow(ui, id.with("ax"), target.min.x, 18.0);
                    let aw = glowkit_core::anim::follow(ui, id.with("aw"), target.width(), 18.0);
                    let ar = Rect::from_min_size(pos2(ax, target.min.y), vec2(aw, target.height()));
                    let p = ui.painter();
                    paint::soft_shadow(
                        p,
                        ar,
                        7.0,
                        6.0,
                        vec2(0.0, 1.0),
                        Color32::from_black_alpha(if t.dark { 60 } else { 18 }),
                    );
                    p.rect_filled(ar, CornerRadius::same(7), if t.dark { t.secondary } else { t.background });
                    for (i, (r, g)) in rects.iter().zip(gs).enumerate() {
                        let rr = ui
                            .interact(*r, id.with(("tab", i)), Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if rr.clicked() {
                            tab = i;
                        }
                        let col = if i == tab {
                            t.foreground
                        } else {
                            t.muted_foreground.lerp_to_gamma(t.foreground, if rr.hovered() { 0.6 } else { 0.0 })
                        };
                        ui.painter().galley(
                            pos2(r.center().x - g.size().x / 2.0, r.center().y - g.size().y / 2.0),
                            g,
                            col,
                        );
                    }
                }
                // body
                let avail = ui.available_rect_before_wrap();
                let inner = Rect::from_min_max(avail.min + vec2(16.0, 16.0), pos2(avail.max.x - 16.0, avail.max.y));
                ui.scope_builder(UiBuilder::new().max_rect(inner), |ui| {
                    match self.height {
                        Some(h) => {
                            egui::ScrollArea::vertical()
                                .id_salt(id.with("scroll"))
                                .max_height(h)
                                .auto_shrink([false, false])
                                .show(ui, |ui| body(ui, tab));
                        }
                        None => body(ui, tab),
                    }
                    ui.add_space(16.0);
                });
            });
        glowkit_core::set_state(ui, id.with("tab"), tab);
        let rect = Rect::from_min_size(top, vec2(width, resp.response.rect.height()));
        ui.painter().set(bg, egui::Shape::rect_filled(rect, cr, t.card));
        ui.painter().rect_stroke(rect, cr, Stroke::new(1.0, t.border), StrokeKind::Inside);
        out.tab = tab;
        out
    }
}

/// A placeholder for an image being generated: a softly moving, shimmering gradient with a
/// progress chip; when `progress` reaches 1 it resolves (sharpening and fading in) to the final
/// image: either your texture or a procedural artwork derived from `seed`.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_ai::ImageGen::new(0.42).size(egui::vec2(320.0, 240.0)));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ImageGen<'a> {
    progress: f32,
    size: egui::Vec2,
    texture: Option<TextureId>,
    seed: u32,
    label: &'a str,
    colors: Option<[Color32; 3]>,
    background: Option<Color32>,
}

/// Paint the area between a rect's square corners and its rounded outline in `color`.
fn mask_corners(p: &egui::Painter, rect: Rect, r: f32, color: Color32) {
    use egui::epaint::Mesh;
    let mut m = Mesh::default();
    let corners = [
        (rect.left_top(), pos2(rect.min.x + r, rect.min.y + r), 180.0f32),
        (rect.right_top(), pos2(rect.max.x - r, rect.min.y + r), 270.0),
        (rect.right_bottom(), pos2(rect.max.x - r, rect.max.y - r), 0.0),
        (rect.left_bottom(), pos2(rect.min.x + r, rect.max.y - r), 90.0),
    ];
    for (corner, c, a0) in corners {
        let base = m.vertices.len() as u32;
        m.colored_vertex(corner, color);
        let n = 8;
        for i in 0..=n {
            let a = (a0 + 90.0 * i as f32 / n as f32).to_radians();
            m.colored_vertex(c + vec2(a.cos(), a.sin()) * (r + 0.3), color);
        }
        for i in 0..n as u32 {
            m.add_triangle(base, base + 1 + i, base + 2 + i);
        }
    }
    p.add(egui::Shape::mesh(m));
}

impl<'a> ImageGen<'a> {
    /// `progress` in 0..1; at 1 the image is shown.
    pub fn new(progress: f32) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            size: vec2(300.0, 300.0),
            texture: None,
            seed: 7,
            label: "Generating image",
            colors: None,
            background: None,
        }
    }
    /// Size of the image.
    pub fn size(mut self, size: egui::Vec2) -> Self {
        self.size = size;
        self
    }
    /// The final image to reveal when done.
    pub fn texture(mut self, id: TextureId) -> Self {
        self.texture = Some(id);
        self
    }
    /// Seed of the procedural stand-in image (when no texture is given).
    pub fn seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }
    /// Label in the progress chip.
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = label;
        self
    }
    /// Color of the surface behind the image (used to round its corners). Defaults to the theme background.
    pub fn background(mut self, c: Color32) -> Self {
        self.background = Some(c);
        self
    }
    /// Colors of the shimmer and of the procedural image. Defaults to the theme gradient.
    pub fn colors(mut self, c: [Color32; 3]) -> Self {
        self.colors = Some(c);
        self
    }
}

impl Widget for ImageGen<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(self.size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let time = glowkit_core::time(ui);
        let done = self.progress >= 1.0;
        let reveal = common::anim_bool(ui, resp.id.with("rev"), done, 1.1);
        let colors = self.colors.unwrap_or(t.gradient);
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        let cr = t.corner_scaled(1.2);
        let r = rect;
        let base = if t.dark { mix(t.card, t.muted, 0.7) } else { t.muted };
        if reveal < 1.0 {
            // generating: slow noise field tinted toward the palette, plus a sweep of light
            let seed = self.seed as f32 * 0.37;
            let p = self.progress;
            paint::shader_grid(&painter, rect, 40, 40, |uv| {
                let n = fbm3(uv.x * 1.4 + seed, uv.y * 1.4, time * 0.15, 2) * 0.5 + 0.5;
                let hue = glowkit_core::noise::noise3(uv.x * 0.9 - seed, uv.y * 0.9, time * 0.1) * 0.5 + 0.5;
                let tint = gradient(&colors, hue);
                let k = 0.18 + 0.4 * p;
                let c = mix(base, tint, k * (0.4 + 0.6 * n));
                let sweep = ((uv.x + uv.y * 0.4) - ((time * 0.45).fract() * 2.2 - 0.5)).abs();
                let shine = (1.0 - sweep / 0.25).clamp(0.0, 1.0);
                mix(c, Color32::WHITE, shine * if t.dark { 0.08 } else { 0.18 })
            });
            glowkit_core::animate(ui);
        }
        if reveal > 0.0 {
            let a = reveal;
            let seed = self.seed as f32;
            match self.texture {
                Some(tex) => {
                    painter.image(
                        tex,
                        rect,
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE.gamma_multiply(a),
                    );
                }
                None => {
                    // procedural "landscape": sky gradient, sun, layered ridges
                    let sky = [mix(colors[0], Color32::BLACK, 0.35), colors[1], colors[2]];
                    paint::shader_grid(&painter, rect, 32, 32, |uv| {
                        let s = gradient(&sky, uv.y * 1.1);
                        let ridge1 = 0.55 + fbm3(uv.x * 3.0 + seed, 0.0, 0.0, 3) * 0.12;
                        let ridge2 = 0.72 + fbm3(uv.x * 4.0 + seed * 2.0, 1.0, 0.0, 3) * 0.08;
                        let mut c = s;
                        if uv.y > ridge1 {
                            c = mix(colors[0], Color32::BLACK, 0.5);
                        }
                        if uv.y > ridge2 {
                            c = mix(colors[0], Color32::BLACK, 0.72);
                        }
                        c.gamma_multiply(a)
                    });
                    let sun = pos2(r.min.x + r.width() * 0.68, r.min.y + r.height() * 0.42);
                    paint::glow(&painter, sun, r.width() * 0.32, with_alpha(colors[2], 0.45 * a));
                    painter.circle_filled(sun, r.width() * 0.07, with_alpha(mix(colors[2], Color32::WHITE, 0.6), a));
                }
            }
        }
        // rounded corners: mask the square image corners with the surrounding color
        mask_corners(ui.painter(), rect, cr.nw as f32, self.background.unwrap_or(t.background));
        ui.painter().rect_stroke(
            rect,
            cr,
            Stroke::new(1.0, with_alpha(t.border, 1.0 - reveal * 0.5)),
            StrokeKind::Inside,
        );
        if !done || reveal < 1.0 {
            let label = format!("{}  {:.0}%", self.label, self.progress * 100.0);
            let g = common::galley(ui, label, 12.0, t.foreground);
            let chip = Rect::from_center_size(rect.center(), vec2(g.size().x + 40.0, 30.0));
            let p = ui.painter();
            let alpha = 1.0 - reveal;
            p.rect_filled(chip, CornerRadius::same(15), with_alpha(t.popover, 0.85 * alpha));
            p.rect_stroke(
                chip,
                CornerRadius::same(15),
                Stroke::new(1.0, with_alpha(t.border, alpha)),
                StrokeKind::Inside,
            );
            common::spinner(
                p,
                pos2(chip.min.x + 16.0, chip.center().y),
                5.5,
                1.6,
                with_alpha(t.foreground, alpha),
                time,
            );
            p.galley(pos2(chip.min.x + 28.0, chip.center().y - g.size().y / 2.0), g, with_alpha(t.foreground, alpha));
            // progress line along the bottom
            let pb = Rect::from_min_size(pos2(rect.min.x + 12.0, rect.max.y - 14.0), vec2(rect.width() - 24.0, 3.0));
            p.rect_filled(pb, CornerRadius::same(2), with_alpha(t.foreground, 0.12 * alpha));
            let pw = glowkit_core::anim::follow(ui, resp.id.with("pw"), self.progress, 5.0);
            p.rect_filled(
                Rect::from_min_size(pb.min, vec2(pb.width() * pw, 3.0)),
                CornerRadius::same(2),
                with_alpha(t.foreground, 0.75 * alpha),
            );
        }
        resp
    }
}
