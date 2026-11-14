use super::util::{bold_text, corner_mask, rounded_ring};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, Vec2,
};
use glowkit_core::{color::with_alpha, hex, mix, paint, Theme};

/// Run `add` in a child `Ui` whose max rect and clip rect are `screen`.
fn screen_ui(ui: &mut Ui, screen: Rect, cr: CornerRadius, bg: Color32, add: impl FnOnce(&mut Ui)) {
    ui.painter().rect_filled(screen, cr, bg);
    let mut child = ui.new_child(UiBuilder::new().max_rect(screen).layout(egui::Layout::top_down(egui::Align::Min)));
    child.set_clip_rect(screen.intersect(ui.clip_rect()));
    add(&mut child);
}

fn lock_icon(p: &Painter, c: Pos2, col: Color32) {
    let body = Rect::from_center_size(c + vec2(0.0, 1.5), vec2(8.0, 6.5));
    p.rect_filled(body, 1.5, col);
    let pts: Vec<Pos2> = (0..=12)
        .map(|i| {
            let a = std::f32::consts::PI * (1.0 + i as f32 / 12.0);
            c + vec2(0.0, -1.5) + vec2(a.cos() * 2.7, a.sin() * 3.0)
        })
        .collect();
    let mut pts = pts;
    pts.insert(0, c + vec2(-2.7, 1.0));
    pts.push(c + vec2(2.7, 1.0));
    p.add(egui::Shape::line(pts, Stroke::new(1.3, col)));
}

/// A browser window mockup (Safari-style chrome: traffic lights, navigation arrows and a
/// centered address bar) with your content rendered, clipped, inside the page area.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_blocks::SafariWindow::new().url("glowkit.rs").size(640.0, 400.0).show(ui, |ui| {
///     ui.label("Hello from inside the browser");
/// });
/// ```
#[must_use = "call `.show(ui, |ui| ...)`"]
pub struct SafariWindow {
    url: String,
    size: Vec2,
    page_bg: Option<Color32>,
}

impl Default for SafariWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl SafariWindow {
    /// A window with an empty address bar.
    pub fn new() -> Self {
        Self { url: "glowkit.rs".into(), size: vec2(640.0, 400.0), page_bg: None }
    }
    /// Text shown in the address bar.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }
    /// Outer size of the window in points.
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = vec2(w, h);
        self
    }
    /// Page background color (defaults to the theme background).
    pub fn page_background(mut self, c: Color32) -> Self {
        self.page_bg = Some(c);
        self
    }
    /// Paint the window and render `add` inside the page area.
    pub fn show(self, ui: &mut Ui, add: impl FnOnce(&mut Ui)) -> Response {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(self.size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let r = 12.0;
        let p = ui.painter().clone();
        let chrome = if t.dark { hex("#1E1E22") } else { hex("#F1F1F3") };
        let border = if t.dark { hex("#34343A") } else { hex("#D9D9DE") };
        paint::soft_shadow(
            &p,
            rect,
            r,
            30.0,
            vec2(0.0, 14.0),
            Color32::from_black_alpha(if t.dark { 140 } else { 40 }),
        );
        p.rect_filled(rect, r, chrome);
        let bar_h = 44.0;
        let bar = Rect::from_min_size(rect.min, vec2(rect.width(), bar_h));
        for (i, c) in [hex("#FF5F57"), hex("#FEBC2E"), hex("#28C840")].into_iter().enumerate() {
            p.circle_filled(pos2(bar.min.x + 20.0 + i as f32 * 20.0, bar.center().y), 6.0, c);
        }
        let icon = t.muted_foreground;
        let st = Stroke::new(1.6, icon);
        // sidebar toggle
        let sb = Rect::from_center_size(pos2(bar.min.x + 100.0, bar.center().y), vec2(16.0, 13.0));
        p.rect_stroke(sb, 3.0, Stroke::new(1.3, icon), StrokeKind::Middle);
        p.line_segment([pos2(sb.min.x + 5.5, sb.min.y), pos2(sb.min.x + 5.5, sb.max.y)], Stroke::new(1.3, icon));
        // back / forward
        super::util::chevron(&p, pos2(bar.min.x + 133.0, bar.center().y), 9.0, vec2(-1.0, 0.0), st);
        super::util::chevron(
            &p,
            pos2(bar.min.x + 155.0, bar.center().y),
            9.0,
            vec2(1.0, 0.0),
            Stroke::new(1.6, with_alpha(icon, 0.45)),
        );
        // address bar
        let aw = (rect.width() * 0.46).clamp(160.0, 420.0);
        let addr = Rect::from_center_size(bar.center(), vec2(aw, 28.0));
        p.rect_filled(addr, 8.0, if t.dark { hex("#2A2A30") } else { hex("#E4E4E8") });
        let g = p.layout_no_wrap(self.url.clone(), FontId::proportional(12.5), t.foreground);
        let total = g.size().x + 16.0;
        let x0 = addr.center().x - total / 2.0;
        lock_icon(&p, pos2(x0 + 4.0, addr.center().y), icon);
        p.galley(pos2(x0 + 16.0, addr.center().y - g.size().y / 2.0), g, t.foreground);
        // reload arrow at the end of the address bar
        let rc = pos2(addr.max.x - 16.0, addr.center().y);
        paint::arc(&p, rc, 5.0, -0.6, 4.4, 1.4, icon);
        // right icons: share, plus, tabs
        let rx = bar.max.x - 20.0;
        let tabs = Rect::from_center_size(pos2(rx - 4.0, bar.center().y), vec2(11.0, 11.0));
        p.rect_stroke(tabs, 2.0, Stroke::new(1.3, icon), StrokeKind::Middle);
        p.rect_stroke(tabs.translate(vec2(3.0, -3.0)), 2.0, Stroke::new(1.3, icon), StrokeKind::Middle);
        let pc = pos2(rx - 34.0, bar.center().y);
        p.line_segment([pc - vec2(6.0, 0.0), pc + vec2(6.0, 0.0)], st);
        p.line_segment([pc - vec2(0.0, 6.0), pc + vec2(0.0, 6.0)], st);
        let sc = pos2(rx - 64.0, bar.center().y);
        p.rect_stroke(
            Rect::from_center_size(sc + vec2(0.0, 2.5), vec2(11.0, 9.0)),
            2.0,
            Stroke::new(1.3, icon),
            StrokeKind::Middle,
        );
        p.line_segment([sc + vec2(0.0, 2.0), sc - vec2(0.0, 7.0)], Stroke::new(1.3, icon));
        super::util::chevron(&p, sc - vec2(0.0, 5.5), 6.0, vec2(0.0, -1.0), Stroke::new(1.3, icon));

        let page = Rect::from_min_max(pos2(rect.min.x, bar.max.y), rect.max);
        p.line_segment([page.left_top(), page.right_top()], Stroke::new(1.0, border));
        let page_inner = Rect::from_min_max(page.min + vec2(0.0, 1.0), page.max);
        let rr = r as u8;
        screen_ui(
            ui,
            page_inner,
            CornerRadius { nw: 0, ne: 0, sw: rr, se: rr },
            self.page_bg.unwrap_or(t.background),
            add,
        );
        p.rect_stroke(rect, r, Stroke::new(1.0, border), StrokeKind::Inside);
        resp
    }
}

/// Phone hardware style for [`PhoneFrame`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PhoneStyle {
    /// Rounded titanium frame with a "dynamic island" pill.
    #[default]
    Island,
    /// Flatter frame with a centered punch-hole camera.
    PunchHole,
}

/// A smartphone mockup: metal frame, black bezel, camera cut-out, status bar and home indicator,
/// with your content rendered (clipped to the rounded screen) inside.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_blocks::PhoneFrame::new().width(220.0).show(ui, |ui| {
///     ui.label("Hello from the phone");
/// });
/// ```
#[must_use = "call `.show(ui, |ui| ...)`"]
pub struct PhoneFrame {
    width: f32,
    style: PhoneStyle,
    frame: Option<Color32>,
    screen_bg: Option<Color32>,
    status_color: Option<Color32>,
    status_bar: bool,
}

impl Default for PhoneFrame {
    fn default() -> Self {
        Self::new()
    }
}

impl PhoneFrame {
    /// A phone with the default (island) style.
    pub fn new() -> Self {
        Self {
            width: 220.0,
            style: PhoneStyle::Island,
            frame: None,
            screen_bg: None,
            status_color: None,
            status_bar: true,
        }
    }
    /// Outer width in points (height follows the phone's aspect ratio).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Hardware style.
    pub fn style(mut self, s: PhoneStyle) -> Self {
        self.style = s;
        self
    }
    /// Frame (metal) color.
    pub fn frame_color(mut self, c: Color32) -> Self {
        self.frame = Some(c);
        self
    }
    /// Screen background color (defaults to the theme background).
    pub fn screen_background(mut self, c: Color32) -> Self {
        self.screen_bg = Some(c);
        self
    }
    /// Status bar text / icon color (defaults to the theme foreground).
    pub fn status_color(mut self, c: Color32) -> Self {
        self.status_color = Some(c);
        self
    }
    /// Show the status bar (time, signal, battery).
    pub fn status_bar(mut self, b: bool) -> Self {
        self.status_bar = b;
        self
    }
    /// Paint the phone and render `add` inside the screen.
    pub fn show(self, ui: &mut Ui, add: impl FnOnce(&mut Ui)) -> Response {
        let t = Theme::of(ui);
        let w = self.width;
        let h = w * 2.05;
        let (rect, resp) = ui.allocate_exact_size(vec2(w + 8.0, h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let body = Rect::from_center_size(rect.center(), vec2(w, h));
        let p = ui.painter().clone();
        let island = self.style == PhoneStyle::Island;
        let r_out = if island { w * 0.18 } else { w * 0.12 };
        let frame = self.frame.unwrap_or(if t.dark { hex("#3A3A40") } else { hex("#B8B8C0") });
        let bezel_w = w * 0.035;
        let frame_w = (w * 0.016).max(2.0);
        paint::soft_shadow(
            &p,
            body,
            r_out,
            40.0,
            vec2(0.0, 18.0),
            Color32::from_black_alpha(if t.dark { 150 } else { 60 }),
        );
        // side buttons
        let btn = mix(frame, Color32::BLACK, 0.15);
        for (y, len) in [(0.18, 0.05), (0.26, 0.09), (0.37, 0.09)] {
            p.rect_filled(
                Rect::from_min_size(pos2(body.min.x - 2.5, body.min.y + h * y), vec2(4.0, h * len)),
                1.5,
                btn,
            );
        }
        p.rect_filled(
            Rect::from_min_size(pos2(body.max.x - 1.5, body.min.y + h * 0.28), vec2(4.0, h * 0.13)),
            1.5,
            btn,
        );
        // metal frame with a vertical sheen
        let outline = paint::rounded_rect_points(body, r_out, 16);
        let hi = mix(frame, Color32::WHITE, 0.35);
        let lo = mix(frame, Color32::BLACK, 0.25);
        paint::fill_convex_fn(&p, &outline, 2, |q| {
            let u = (q.x - body.min.x) / body.width();
            let k = (u * std::f32::consts::PI * 2.0).cos() * 0.5 + 0.5;
            lo.lerp_to_gamma(hi, k)
        });
        let bezel_rect = body.shrink(frame_w);
        let r_bezel = r_out - frame_w;
        let screen = bezel_rect.shrink(bezel_w);
        let r_screen = (r_bezel - bezel_w).max(4.0);
        let bezel_col = hex("#050506");
        screen_ui(ui, screen, CornerRadius::ZERO, self.screen_bg.unwrap_or(t.background), add);
        rounded_ring(&p, bezel_rect, r_bezel, screen, r_screen, bezel_col);
        corner_mask(&p, screen, r_screen, bezel_col);
        // camera
        let sc = self.status_color.unwrap_or(t.foreground);
        let top_y = screen.min.y + w * 0.06;
        if island {
            let isl = Rect::from_center_size(pos2(screen.center().x, top_y), vec2(w * 0.3, w * 0.085));
            p.rect_filled(isl, CornerRadius::same(99), Color32::BLACK);
            p.circle_filled(pos2(isl.max.x - isl.height() * 0.5, isl.center().y), isl.height() * 0.22, hex("#14182A"));
        } else {
            p.circle_filled(pos2(screen.center().x, top_y), w * 0.03, Color32::BLACK);
            p.circle_filled(pos2(screen.center().x, top_y), w * 0.012, hex("#1B2238"));
        }
        if self.status_bar {
            let fs = (w * 0.058).max(9.0);
            bold_text(
                &p,
                pos2(screen.min.x + w * 0.16, top_y),
                Align2::CENTER_CENTER,
                "9:41",
                FontId::proportional(fs),
                sc,
            );
            // signal bars
            let bx = screen.max.x - w * 0.27;
            for i in 0..4 {
                let bh = fs * (0.3 + 0.18 * i as f32);
                p.rect_filled(
                    Rect::from_min_max(
                        pos2(bx + i as f32 * fs * 0.3, top_y + fs * 0.35 - bh),
                        pos2(bx + i as f32 * fs * 0.3 + fs * 0.2, top_y + fs * 0.35),
                    ),
                    0.8,
                    sc,
                );
            }
            // battery
            let bat = Rect::from_min_size(pos2(screen.max.x - w * 0.14, top_y - fs * 0.32), vec2(fs * 1.5, fs * 0.66));
            p.rect_stroke(bat, 2.5, Stroke::new(1.0, with_alpha(sc, 0.5)), StrokeKind::Inside);
            p.rect_filled(bat.shrink(1.8), 1.5, sc);
            p.rect_filled(
                Rect::from_min_size(pos2(bat.max.x + 1.0, bat.center().y - 2.0), vec2(1.5, 4.0)),
                1.0,
                with_alpha(sc, 0.5),
            );
            // home indicator
            let hb = Rect::from_center_size(
                pos2(screen.center().x, screen.max.y - w * 0.035),
                vec2(w * 0.36, (w * 0.016).max(3.0)),
            );
            p.rect_filled(hb, CornerRadius::same(99), with_alpha(sc, 0.85));
        }
        // glass edge
        p.rect_stroke(body.shrink(0.5), r_out, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.18)), StrokeKind::Inside);
        resp
    }
}

/// A laptop mockup: a thin-bezel lid with a webcam dot above a metal base with a thumb notch.
/// Your content renders inside the display.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_blocks::LaptopFrame::new().width(520.0).show(ui, |ui| {
///     ui.label("Hello from the laptop");
/// });
/// ```
#[must_use = "call `.show(ui, |ui| ...)`"]
pub struct LaptopFrame {
    width: f32,
    screen_bg: Option<Color32>,
    base: Option<Color32>,
}

impl Default for LaptopFrame {
    fn default() -> Self {
        Self::new()
    }
}

impl LaptopFrame {
    /// A laptop with default size.
    pub fn new() -> Self {
        Self { width: 520.0, screen_bg: None, base: None }
    }
    /// Width of the lid in points (the base is a bit wider).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Screen background color (defaults to the theme background).
    pub fn screen_background(mut self, c: Color32) -> Self {
        self.screen_bg = Some(c);
        self
    }
    /// Base (aluminium) color.
    pub fn base_color(mut self, c: Color32) -> Self {
        self.base = Some(c);
        self
    }
    /// Paint the laptop and render `add` inside the display.
    pub fn show(self, ui: &mut Ui, add: impl FnOnce(&mut Ui)) -> Response {
        let t = Theme::of(ui);
        let w = self.width;
        let lid_h = w * 0.64;
        let base_h = w * 0.035;
        let total = vec2(w * 1.18, lid_h + base_h + 6.0);
        let (rect, resp) = ui.allocate_exact_size(total, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let p = ui.painter().clone();
        let lid = Rect::from_min_size(pos2(rect.center().x - w / 2.0, rect.min.y), vec2(w, lid_h));
        let r = w * 0.03;
        let metal = self.base.unwrap_or(if t.dark { hex("#4A4A52") } else { hex("#C9C9D0") });
        paint::soft_shadow(&p, lid, r, 36.0, vec2(0.0, 18.0), Color32::from_black_alpha(if t.dark { 130 } else { 45 }));
        // lid: thin metal edge, black bezel
        p.rect_filled(lid, CornerRadius { nw: r as u8, ne: r as u8, sw: 2, se: 2 }, metal);
        let bez = lid.shrink(1.5);
        let bezel_col = hex("#0A0A0C");
        p.rect_filled(bez, CornerRadius { nw: (r - 1.5) as u8, ne: (r - 1.5) as u8, sw: 1, se: 1 }, bezel_col);
        let screen = Rect::from_min_max(bez.min + vec2(w * 0.022, w * 0.03), bez.max - vec2(w * 0.022, w * 0.035));
        screen_ui(ui, screen, CornerRadius::ZERO, self.screen_bg.unwrap_or(t.background), add);
        corner_mask(&p, screen, 4.0, bezel_col);
        p.circle_filled(pos2(lid.center().x, bez.min.y + w * 0.015), 2.2, hex("#1E2433"));
        p.circle_filled(pos2(lid.center().x - 0.6, bez.min.y + w * 0.015 - 0.6), 0.7, hex("#5A6A9A"));
        // hinge + base
        let base = Rect::from_min_size(pos2(rect.min.x, lid.max.y), vec2(rect.width(), base_h));
        let hi = mix(metal, Color32::WHITE, 0.35);
        let lo = mix(metal, Color32::BLACK, 0.35);
        let outline = vec![
            base.left_top(),
            base.right_top(),
            pos2(base.max.x - base_h * 0.5, base.max.y - base_h * 0.3),
            pos2(base.max.x - base_h * 1.6, base.max.y),
            pos2(base.min.x + base_h * 1.6, base.max.y),
            pos2(base.min.x + base_h * 0.5, base.max.y - base_h * 0.3),
        ];
        paint::fill_convex_fn(&p, &outline, 2, |q| {
            let k = ((q.y - base.min.y) / base.height()).clamp(0.0, 1.0);
            hi.lerp_to_gamma(lo, k)
        });
        p.line_segment(
            [base.left_top() + vec2(4.0, 0.5), base.right_top() + vec2(-4.0, 0.5)],
            Stroke::new(1.0, with_alpha(Color32::WHITE, 0.5)),
        );
        let notch =
            Rect::from_center_size(pos2(base.center().x, base.min.y + base_h * 0.22), vec2(w * 0.17, base_h * 0.44));
        p.rect_filled(notch, CornerRadius { nw: 0, ne: 0, sw: 6, se: 6 }, lo);
        // floor shadow
        paint::glow_ellipse(
            &p,
            pos2(base.center().x, base.max.y + 3.0),
            vec2(rect.width() * 0.5, 6.0),
            Color32::from_black_alpha(if t.dark { 120 } else { 40 }),
        );
        resp
    }
}
