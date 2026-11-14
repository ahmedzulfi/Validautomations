use super::common::{capture, clip_mesh, pointer_or_auto, subdivide};
use crate::buttons::util::{self, outline};
use crate::motion::common::sd_round_rect;
use egui::epaint::Mesh;
use egui::{vec2, Align, Color32, InnerResponse, Layout, Painter, Pos2, Rect, Sense, Shape, Ui, UiBuilder, Vec2};
use glowkit_core::anim::{smoothstep, spring2};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{noise, paint, Theme};

/// Outward normal of a rounded rect's distance field (smooth inside the bending zone when
/// `r` is at least the zone width).
fn rr_normal(p: Pos2, rect: Rect, r: f32) -> Vec2 {
    let d = p - rect.center();
    let half = rect.size() / 2.0 - Vec2::splat(r);
    let a = vec2(d.x.abs() - half.x, d.y.abs() - half.y);
    let sx = if d.x < 0.0 { -1.0 } else { 1.0 };
    let sy = if d.y < 0.0 { -1.0 } else { 1.0 };
    if a.x > 0.0 && a.y > 0.0 {
        let n = a.normalized();
        vec2(n.x * sx, n.y * sy)
    } else if a.x > a.y {
        vec2(sx, 0.0)
    } else {
        vec2(0.0, sy)
    }
}

/// A frosted, refracting glass panel over any backdrop.
///
/// egui can't read back what is already on screen, so the backdrop is supplied as a painter
/// closure. The panel paints it normally, then paints a second copy inside the glass that is
/// bent toward the rim (content just outside the edge is pulled in and compressed, like light
/// through a thick pane), optionally magnified, blurred and split into red/blue fringes, then
/// tinted and lit with specular edge highlights. Your content goes on top of the glass.
///
/// Turn on [`GlassSurface::lens`] and [`GlassSurface::follow_pointer`] for a liquid lens that
/// glides after the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::GlassSurface;
/// GlassSurface::new().size(egui::vec2(320.0, 160.0)).show(
///     ui,
///     |painter, rect| glowkit_core::paint::gradient_rect(painter, rect, 0.0, 30.0, &[glowkit_core::hex("#F472B6"), glowkit_core::hex("#6366F1")]),
///     |ui| { ui.heading("Glass"); },
/// );
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct GlassSurface {
    size: Vec2,
    height: f32,
    radius: Option<f32>,
    refraction: f32,
    rim: Option<f32>,
    magnify: f32,
    blur: f32,
    tint: Option<Color32>,
    chromatic: bool,
    wobble: f32,
    lens: bool,
    follow_pointer: bool,
    autoplay: bool,
    offset: Vec2,
}

impl Default for GlassSurface {
    fn default() -> Self {
        Self::new()
    }
}

impl GlassSurface {
    /// A glass panel.
    pub fn new() -> Self {
        Self {
            size: vec2(320.0, 170.0),
            height: 340.0,
            radius: None,
            refraction: 22.0,
            rim: None,
            magnify: 1.0,
            blur: 0.0,
            tint: None,
            chromatic: true,
            wobble: 0.0,
            lens: false,
            follow_pointer: false,
            autoplay: false,
            offset: Vec2::ZERO,
        }
    }
    /// Size of the glass panel.
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size.max(vec2(24.0, 24.0));
        self
    }
    /// Height of the stage the backdrop fills (width is all available).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(40.0);
        self
    }
    /// Corner radius (default: a soft 28, or fully round for a lens).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = Some(r.max(0.0));
        self
    }
    /// How far (points) backdrop content outside the rim is pulled in under the glass.
    pub fn refraction(mut self, px: f32) -> Self {
        self.refraction = px.max(0.0);
        self
    }
    /// Width of the bending zone along the rim (default: about a quarter of the short side).
    pub fn rim(mut self, px: f32) -> Self {
        self.rim = Some(px.max(1.0));
        self
    }
    /// Magnify what is seen through the glass (1 = none).
    pub fn magnify(mut self, k: f32) -> Self {
        self.magnify = k.clamp(0.5, 3.0);
        self
    }
    /// Frost: blur radius in points (0 = clear glass).
    pub fn blur(mut self, px: f32) -> Self {
        self.blur = px.clamp(0.0, 24.0);
        self
    }
    /// Tint color (its alpha is the tint strength). Default: a faint white.
    pub fn tint(mut self, c: Color32) -> Self {
        self.tint = Some(c);
        self
    }
    /// Red/blue dispersion fringes near the rim.
    pub fn chromatic(mut self, on: bool) -> Self {
        self.chromatic = on;
        self
    }
    /// Liquid wobble of the refraction (0 = still).
    pub fn wobble(mut self, k: f32) -> Self {
        self.wobble = k.max(0.0);
        self
    }
    /// Bend across the whole surface like a thick round lens instead of only at the rim.
    pub fn lens(mut self, on: bool) -> Self {
        self.lens = on;
        self
    }
    /// Glide after the pointer while it is over the stage.
    pub fn follow_pointer(mut self, on: bool) -> Self {
        self.follow_pointer = on;
        self
    }
    /// With `follow_pointer`, drift around on its own while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Offset of the panel from the stage center.
    pub fn offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    /// Paint the backdrop over the stage, the glass over it, and `add` inside the glass.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        backdrop: impl Fn(&Painter, Rect),
        add: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let t = Theme::of(ui);
        let (stage, resp) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let id = resp.id;
        let painter = ui.painter_at(stage);
        backdrop(&painter, stage);

        let mut center = stage.center() + self.offset;
        if self.follow_pointer {
            let ptr = pointer_or_auto(ui, stage, self.autoplay, 2.4);
            let last: Vec2 = glowkit_core::state(ui, id.with("last"), || center - stage.min);
            let target = ptr.map(|p| p.0 - stage.min).unwrap_or(last);
            glowkit_core::set_state(ui, id.with("last"), target);
            center = stage.min + spring2(ui, id.with("pos"), target, 90.0, 16.0);
        }
        let glass = Rect::from_center_size(center, self.size);
        let short = self.size.min_elem();
        let radius = self.radius.unwrap_or(if self.lens { short / 2.0 } else { 28.0 }).min(short / 2.0);
        let e = if self.lens { short / 2.0 } else { self.rim.unwrap_or(short * 0.25).min(short / 2.0) };
        let wr = radius.max(e).min(short / 2.0);
        let time = glowkit_core::time(ui);
        if self.wobble > 0.0 || self.follow_pointer {
            glowkit_core::animate(ui);
        }

        // drop shadow under the glass
        paint::soft_shadow(
            &painter,
            glass,
            radius,
            30.0,
            vec2(0.0, 12.0),
            Color32::BLACK.gamma_multiply(if t.dark { 0.45 } else { 0.18 }),
        );

        let src = capture(ui, |p| backdrop(&p.with_clip_rect(stage), stage));
        let region = glass.expand(self.refraction + 4.0);
        let src = subdivide(&src, if self.lens { 8.0 } else { 10.0 }, region);
        let clip = outline(glass, radius, 6.0);
        let m = self.refraction;
        let mag = self.magnify;
        let wob = self.wobble;
        let warp_with = |scale: f32| -> Mesh {
            let mut mesh = src.clone();
            let mm = m * scale;
            let p = (e + mm) / e.max(1.0);
            for v in &mut mesh.vertices {
                let s = v.pos;
                let sd = sd_round_rect(s, glass, wr);
                let mut q = s;
                if sd > -e && sd < mm + 2.0 {
                    let x = ((sd + e) / (e + mm)).clamp(0.0, 1.0);
                    let u = 1.0 - (1.0 - x).powf(p);
                    let g = -e + e * u;
                    q = s + rr_normal(s, glass, wr) * (g - sd);
                }
                if wob > 0.0 {
                    let w = smoothstep(-e * 1.2, 0.0, sd) * wob * 6.0;
                    q += vec2(
                        noise::noise3(s.x * 0.02, s.y * 0.02, time * 0.6),
                        noise::noise3(s.x * 0.02 + 9.1, s.y * 0.02, time * 0.6),
                    ) * w;
                }
                v.pos = center + (q - center) * mag;
            }
            clip_mesh(&mesh, &clip)
        };
        let main = warp_with(1.0);
        let tint = self.tint.unwrap_or(with_alpha(Color32::WHITE, if t.dark { 0.07 } else { 0.22 }));
        let ta = tint.a() as f32 / 255.0;
        let tint_rgb = tint.to_opaque();
        let tinted = |mut mesh: Mesh, alpha: f32| -> Mesh {
            for v in &mut mesh.vertices {
                let lift = smoothstep(-e, 0.0, sd_round_rect(v.pos, glass, wr)) * 0.06;
                v.color = mix(v.color, tint_rgb.gamma_multiply(v.color.a() as f32 / 255.0), ta + lift).gamma_multiply(alpha);
            }
            mesh
        };
        // opaque base so the unbent backdrop never shows through the glass
        paint::fill_convex_fn(&painter, &clip, 1, |_| if t.dark { t.background } else { Color32::WHITE });
        if self.blur > 0.5 {
            let taps = 7;
            painter.add(Shape::mesh(tinted(main.clone(), 1.0)));
            for k in 1..taps {
                let a = k as f32 / (taps - 1) as f32 * std::f32::consts::TAU + 0.4;
                let rr = self.blur * if k % 2 == 0 { 1.0 } else { 0.55 };
                let mut c = main.clone();
                c.translate(vec2(a.cos(), a.sin()) * rr);
                let c = clip_mesh(&c, &clip);
                painter.add(Shape::mesh(tinted(c, 1.0 / (k + 1) as f32)));
            }
        } else {
            painter.add(Shape::mesh(tinted(main, 1.0)));
        }
        if self.chromatic && m > 0.5 {
            for (scale, chan) in [(1.18f32, 0usize), (0.82, 2)] {
                let mut fr = warp_with(scale);
                for v in &mut fr.vertices {
                    let c = v.color;
                    // additive light: fade it out on bright content so it never washes out to white
                    let lum = (0.3 * c.r() as f32 + 0.55 * c.g() as f32 + 0.15 * c.b() as f32) / 255.0;
                    let w = smoothstep(-e * 0.6, 0.0, sd_round_rect(v.pos, glass, wr)) * 0.36 * (1.0 - lum).powi(2);
                    let val = if chan == 0 { c.r() } else { c.b() };
                    let x = (val as f32 * w) as u8;
                    // alpha 0: additive light
                    v.color = if chan == 0 {
                        Color32::from_rgba_premultiplied(x, 0, 0, 0)
                    } else {
                        Color32::from_rgba_premultiplied(0, x / 3, x, 0)
                    };
                }
                painter.add(Shape::mesh(fr));
            }
        }
        // a whisper of frost so plain areas still read as glass
        util::inner_glow(&painter, glass, radius, glass.left_top() + glass.size() * vec2(0.2, 0.0), short * 1.4, with_alpha(Color32::WHITE, if t.dark { 0.06 } else { 0.12 }));
        // specular rim: bright where the edge faces the light (top-left)
        let light = vec2(-0.6, -0.8);
        util::inner_border_fn(&painter, glass, radius, 1.4, |q, _| {
            let n = rr_normal(q, glass, radius.max(1.0));
            let d = n.dot(light);
            let a = 0.18 + 0.62 * d.max(0.0).powf(1.5) + 0.3 * (-d).max(0.0).powf(3.0);
            with_alpha(Color32::WHITE, a)
        });
        util::inner_border_fn(&painter, glass.shrink(1.4), (radius - 1.4).max(0.0), 5.0, |q, _| {
            let n = rr_normal(q, glass, radius.max(1.0));
            with_alpha(Color32::WHITE, 0.08 * n.dot(light).max(0.0))
        });
        painter.rect_stroke(
            glass,
            util::cr(radius),
            egui::Stroke::new(1.0, with_alpha(Color32::BLACK, if t.dark { 0.35 } else { 0.08 })),
            egui::StrokeKind::Outside,
        );

        // content, vertically centered using last frame's measured height
        let inner = glass.shrink(18.0);
        let h_key = id.with("content_h");
        let ch: f32 = glowkit_core::state(ui, h_key, || 0.0);
        let top = inner.min.y + ((inner.height() - ch) / 2.0).max(0.0);
        let mut child = ui.new_child(
            UiBuilder::new()
                .id_salt(id.with("content"))
                .max_rect(Rect::from_min_max(egui::pos2(inner.min.x, top), inner.max))
                .layout(Layout::top_down(Align::Center)),
        );
        child.set_clip_rect(glass.intersect(ui.clip_rect()));
        let r = add(&mut child);
        let measured = child.min_rect().height();
        if (measured - ch).abs() > 0.5 {
            glowkit_core::set_state(ui, h_key, measured);
            ui.ctx().request_repaint();
        }
        InnerResponse::new(r, resp)
    }
}
