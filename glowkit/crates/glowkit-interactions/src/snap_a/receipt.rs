use super::common::{cached_ink, idle_time, lerp_c, sstep, wall_dt, Cam, Ink, Sheet, Soup, V3};
use egui::epaint::Mesh;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::{hex, paint, Theme};

/// One plan of a [`ReceiptPrinter`].
#[derive(Clone, Debug)]
pub struct ReceiptPlan {
    /// Plan name.
    pub name: String,
    /// One-line pitch.
    pub blurb: String,
    /// Monthly price.
    pub price: f32,
    /// Receipt rows: `(label, value)`.
    pub rows: Vec<(String, String)>,
}

impl ReceiptPlan {
    /// A plan called `name` at `price` per month.
    pub fn new(name: impl Into<String>, blurb: impl Into<String>, price: f32) -> Self {
        Self { name: name.into(), blurb: blurb.into(), price, rows: Vec::new() }
    }
    /// Add a receipt row.
    pub fn row(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.rows.push((label.into(), value.into()));
        self
    }
}

fn default_plans() -> Vec<ReceiptPlan> {
    vec![
        ReceiptPlan::new("Solo", "One person, a few good clients.", 0.0)
            .row("PLAN", "SOLO")
            .row("CLIENTS", "3 ACTIVE")
            .row("INVOICES", "UNLIMITED")
            .row("PAYOUTS", "3 DAYS")
            .row("SEATS x1", "FREE"),
        ReceiptPlan::new("Studio", "Small teams sharing the hours.", 16.0)
            .row("PLAN", "STUDIO")
            .row("CLIENTS", "UNLIMITED")
            .row("RETAINERS", "AUTO-BILL")
            .row("PAYOUTS", "NEXT DAY")
            .row("SEATS x2", "INCLUDED"),
        ReceiptPlan::new("Agency", "Many clients, many hands.", 40.0)
            .row("PLAN", "AGENCY")
            .row("PORTALS", "WHITE-LABEL")
            .row("SSO + AUDIT", "INCLUDED")
            .row("PAYOUTS", "INSTANT")
            .row("SEATS x5", "INCLUDED"),
    ]
}

#[derive(Clone, Copy, Debug)]
struct PrinterState {
    plan: usize,
    shown: usize,
    feed: f32,
    rewinding: bool,
    hour: f32,
    torn: Option<(f32, Vec2, f32)>,
    kept: Option<usize>,
    auto_t: f32,
    auto_n: u32,
    drag: f32,
}

/// A pricing section where a thermal printer prints the price. Pick a plan and the receipt
/// rewinds into the slot and a new one feeds out line by line, the total is circled in red ink,
/// and the sea behind the printer changes hour with the plan. Drag the receipt down to tear it
/// off: it falls away and the call to action remembers the plan you kept.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::ReceiptPrinter::new().headline("Bill the hours.", "Keep the change."));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ReceiptPrinter {
    plans: Vec<ReceiptPlan>,
    brand: String,
    headline: (String, String),
    size: Option<Vec2>,
    feed_speed: f32,
    autoplay: bool,
}

impl Default for ReceiptPrinter {
    fn default() -> Self {
        Self::new()
    }
}

impl ReceiptPrinter {
    /// A printer with three built-in plans.
    pub fn new() -> Self {
        Self {
            plans: default_plans(),
            brand: "FERRYLOG".into(),
            headline: ("Bill the hours.".into(), "Keep the change.".into()),
            size: None,
            feed_speed: 1.0,
            autoplay: true,
        }
    }
    /// Replace the plans (needs at least one).
    pub fn plans(mut self, plans: Vec<ReceiptPlan>) -> Self {
        if !plans.is_empty() {
            self.plans = plans;
        }
        self
    }
    /// Brand printed on the receipt header.
    pub fn brand(mut self, s: impl Into<String>) -> Self {
        self.brand = s.into();
        self
    }
    /// Two-line headline.
    pub fn headline(mut self, a: impl Into<String>, b: impl Into<String>) -> Self {
        self.headline = (a.into(), b.into());
        self
    }
    /// Widget size (default: available width x 420).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Paper feed speed multiplier (default 1).
    pub fn feed_speed(mut self, k: f32) -> Self {
        self.feed_speed = k.clamp(0.3, 3.0);
        self
    }
    /// Cycle through the plans (and tear the odd receipt) while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const RECEIPT_H: f32 = 300.0;

fn print_receipt(ink: &mut Ink, w: f32, brand: &str, plan: &ReceiptPlan) {
    let ink_c = hex("#26221E");
    let soft = hex("#6E675F");
    ink.text(pos2(w * 0.5, 18.0), Align2::CENTER_TOP, &format!("~ {brand} ~"), FontId::monospace(11.0), ink_c);
    ink.text(pos2(w * 0.5, 33.0), Align2::CENTER_TOP, "PIER 4 · OPEN TILL LATE", FontId::monospace(5.5), soft);
    ink.text(pos2(14.0, 50.0), Align2::LEFT_TOP, "NO. 000421", FontId::monospace(5.5), soft);
    ink.text(pos2(w - 14.0, 50.0), Align2::RIGHT_TOP, "14 NOV 21:04", FontId::monospace(5.5), soft);
    dashed(ink, 62.0, w);
    let mut y = 72.0;
    for (a, b) in &plan.rows {
        ink.text(pos2(14.0, y), Align2::LEFT_TOP, a, FontId::monospace(6.5), ink_c);
        ink.text(pos2(w - 14.0, y), Align2::RIGHT_TOP, b, FontId::monospace(6.5), ink_c);
        y += 13.0;
    }
    dashed(ink, y + 4.0, w);
    ink.text(pos2(14.0, y + 20.0), Align2::LEFT_TOP, "TOTAL", FontId::monospace(8.0), ink_c);
    ink.text(pos2(14.0, y + 31.0), Align2::LEFT_TOP, "PER MONTH", FontId::monospace(5.0), soft);
    ink.text(pos2(w - 14.0, y + 14.0), Align2::RIGHT_TOP, &format!("${:.2}", plan.price), FontId::monospace(22.0), ink_c);
    // barcode
    let mut x = 30.0;
    let mut k = 99u64 + plan.price as u64;
    let by = y + 58.0;
    while x < w - 30.0 {
        k = k.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let bw = (1 + (k >> 61) % 3) as f32 * 0.9;
        ink.rect(Rect::from_min_size(pos2(x, by), vec2(bw, 20.0)), 0.0, ink_c);
        x += bw + (1 + (k >> 52) % 3) as f32 * 0.9;
    }
    ink.text(pos2(w * 0.5, by + 26.0), Align2::CENTER_TOP, "THANK YOU FOR CLOCKING IN", FontId::monospace(5.0), soft);
}

fn dashed(ink: &mut Ink, y: f32, w: f32) {
    let mut x = 12.0;
    while x < w - 12.0 {
        ink.rect(Rect::from_min_size(pos2(x, y), vec2(3.0, 0.7)), 0.0, hex("#8C857C"));
        x += 5.0;
    }
}

/// Keep only triangles fully above `max_y` (lines appear as they leave the slot).
fn clip_y(m: &Mesh, max_y: f32) -> Mesh {
    let mut out = Mesh::default();
    for t in m.indices.chunks_exact(3) {
        let v = [m.vertices[t[0] as usize], m.vertices[t[1] as usize], m.vertices[t[2] as usize]];
        if v.iter().all(|v| v.pos.y <= max_y) {
            let b = out.vertices.len() as u32;
            out.vertices.extend_from_slice(&v);
            out.add_triangle(b, b + 1, b + 2);
        }
    }
    out
}

/// Sky/sea palette for an hour value: 0 dawn, 1 day, 2 night.
fn sea_colors(hour: f32) -> [Color32; 4] {
    let dawn = [hex("#E9B7B0"), hex("#F6D7C3"), hex("#7FA3B5"), hex("#3E6478")];
    let day = [hex("#8CC2E8"), hex("#D8ECF7"), hex("#3D8FB3"), hex("#1E5B7A")];
    let night = [hex("#121A3A"), hex("#2C3566"), hex("#1B2850"), hex("#0B1230")];
    let (a, b, t) = if hour < 1.0 { (dawn, day, hour) } else { (day, night, hour - 1.0) };
    std::array::from_fn(|i| lerp_c(a[i], b[i], t))
}

fn paint_sea(painter: &egui::Painter, r: Rect, hour: f32, time: f32) {
    let c = sea_colors(hour);
    let horizon = 0.58;
    paint::shader_grid(painter, r, 1, 24, |uv| {
        if uv.y < horizon {
            lerp_c(c[0], c[1], uv.y / horizon)
        } else {
            lerp_c(c[2], c[3], (uv.y - horizon) / (1.0 - horizon))
        }
    });
    // sun / moon
    let night = (hour - 1.0).clamp(0.0, 1.0);
    let sun = pos2(r.min.x + r.width() * 0.84, r.min.y + r.height() * (0.5 - 0.22 * (1.0 - (hour - 0.9).abs().min(1.0))));
    paint::glow(painter, sun, 40.0, with_alpha(lerp_c(hex("#FFE7C2"), hex("#DDE6FF"), night), 0.5));
    painter.circle_filled(sun, 11.0, lerp_c(hex("#FFF4DF"), hex("#F2F4FF"), night));
    if night > 0.3 {
        painter.circle_filled(sun + vec2(4.0, -3.0), 10.0, with_alpha(c[0], night));
    }
    // swell lines
    for i in 0..14 {
        let fy = horizon + (i as f32 + 0.5) / 14.0 * (1.0 - horizon);
        let y = r.min.y + r.height() * fy;
        let depth = (fy - horizon) / (1.0 - horizon);
        let pts: Vec<Pos2> = (0..=30)
            .map(|k| {
                let x = r.min.x + k as f32 / 30.0 * r.width();
                pos2(x, y + ((x * 0.03 + time * 0.8 + i as f32).sin()) * (1.0 + depth * 2.0))
            })
            .collect();
        painter.add(Shape::line(pts, Stroke::new(0.6 + depth, with_alpha(Color32::WHITE, 0.06 + 0.08 * (1.0 - depth)))));
    }
    // shore
    let shore_y = r.max.y - r.height() * 0.06;
    paint::shader_grid(painter, Rect::from_min_max(pos2(r.min.x, shore_y), r.max), 1, 3, |uv| {
        lerp_c(lerp_c(hex("#D9C29C"), hex("#3B3A44"), night), lerp_c(hex("#C7AC82"), hex("#26252E"), night), uv.y)
    });
}

impl Widget for ReceiptPrinter {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = glowkit_core::desired_size(ui, self.size, 420.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let id = response.id;
        let n = self.plans.len();
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let time = glowkit_core::time(ui);
        let idle = idle_time(ui, id, rect);
        let left_w = (rect.width() * 0.42).clamp(220.0, 330.0);
        let scene = Rect::from_min_size(rect.min, vec2(left_w, rect.height())).shrink(4.0);
        let right = Rect::from_min_max(pos2(scene.max.x + 28.0, rect.min.y + 8.0), rect.max - vec2(8.0, 8.0));
        // printer geometry
        let printer = Rect::from_min_size(scene.min + vec2(14.0, 14.0), vec2(scene.width() - 28.0, 64.0));
        let slot_y = printer.max.y - 6.0;
        let paper_w = (printer.width() - 52.0).min(190.0);
        let paper_x = printer.center().x - paper_w * 0.5;
        // plan rows on the right
        let row_h = 46.0;
        let rows_top = right.min.y + 118.0;
        let row_rect = |i: usize| Rect::from_min_size(pos2(right.min.x, rows_top + i as f32 * (row_h + 6.0)), vec2(right.width().min(380.0), row_h));
        let mut picked = None;
        for i in 0..n {
            let r = ui.interact(row_rect(i), id.with(("plan", i)), Sense::click());
            if r.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if r.clicked() {
                picked = Some(i);
            }
        }
        let st0: PrinterState = glowkit_core::state(ui, id, || PrinterState {
            plan: 1,
            shown: 1,
            feed: 0.0,
            rewinding: false,
            hour: 1.0,
            torn: None,
            kept: None,
            auto_t: 3.5,
            auto_n: 0,
            drag: 0.0,
        });
        let paper_len = st0.feed * RECEIPT_H;
        let paper_rect = Rect::from_min_size(pos2(paper_x, slot_y), vec2(paper_w, paper_len.max(1.0)));
        let paper_resp = ui.interact(paper_rect, id.with("paper"), Sense::drag());
        if paper_resp.hovered() && st0.feed > 0.6 {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        let drag_dy = if paper_resp.dragged() { paper_resp.drag_delta().y } else { 0.0 };
        let drag_end = paper_resp.drag_stopped();
        let speed = self.feed_speed;
        let autoplay = self.autoplay && idle > 2.5;
        let st = glowkit_core::with_state(ui, id, || st0, |s| {
            if let Some(p) = picked {
                if p != s.plan {
                    s.plan = p;
                    s.rewinding = s.feed > 0.0;
                }
                s.auto_t = 5.0;
            }
            if autoplay {
                s.auto_t -= wdt;
                if s.auto_t <= 0.0 {
                    s.auto_n += 1;
                    if s.auto_n % 3 == 0 && s.feed > 0.95 && s.torn.is_none() {
                        s.torn = Some((0.0, vec2(0.0, 40.0), 0.0));
                        s.kept = Some(s.shown);
                        s.feed = 0.0;
                    } else {
                        s.plan = (s.plan + 1) % n;
                        s.rewinding = s.feed > 0.0;
                    }
                    s.auto_t = 4.6;
                }
            }
            // drag the paper down to tear it off
            if drag_dy != 0.0 && s.feed > 0.6 && !s.rewinding {
                s.drag = (s.drag + drag_dy).max(0.0);
                if s.drag > 46.0 {
                    s.torn = Some((0.0, vec2(0.0, s.drag * 4.0), 0.0));
                    s.kept = Some(s.shown);
                    s.feed = 0.0;
                    s.drag = 0.0;
                }
            }
            if drag_end || drag_dy == 0.0 {
                s.drag *= (-12.0 * dt).exp();
            }
            if s.rewinding {
                s.feed -= dt * 2.4 * speed;
                if s.feed <= 0.0 {
                    s.feed = 0.0;
                    s.rewinding = false;
                }
            } else {
                if s.feed <= 0.0 {
                    s.shown = s.plan;
                }
                if s.torn.is_none_or(|(tt, _, _)| tt > 0.5) {
                    s.feed = (s.feed + dt * 0.62 * speed).min(1.0);
                }
            }
            if let Some((tt, v, a)) = s.torn.as_mut() {
                *tt += dt;
                v.y += 900.0 * dt;
                *a += dt * 0.9;
                if *tt > 1.3 {
                    s.torn = None;
                }
            }
            let target_hour = if n > 1 { s.plan as f32 / (n - 1) as f32 * 2.0 } else { 1.0 };
            s.hour += (target_hour - s.hour) * (1.0 - (-3.0 * dt).exp());
            *s
        });
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        // ---- scene card
        let cr = CornerRadius::same((t.radius * 1.4) as u8);
        paint::soft_shadow(&painter, scene, t.radius * 1.4, 18.0, vec2(0.0, 8.0), Color32::from_black_alpha(if t.dark { 90 } else { 30 }));
        let scene_p = painter.with_clip_rect(scene);
        paint_sea(&scene_p, scene, st.hour, time);
        scene_p.rect_stroke(scene, cr, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.15)), egui::StrokeKind::Inside);
        // receipt (behind the printer front)
        let plan = &self.plans[st.shown.min(n - 1)];
        let ink = cached_ink(ui, id.with(("receipt", st.shown, plan.price.to_bits())), 6.0, |ink| print_receipt(ink, paper_w, &self.brand, plan));
        let paper = hex("#F7F4EC");
        let draw_paper = |soup: &mut Soup, cam: &Cam, len: f32, sway: f32, alpha: f32| {
            let len = len.max(2.0);
            let ink = clip_y(&ink, len - 4.0);
            let deform = move |u: f32, v: f32| {
                let y = v * len;
                // the free end curls toward you and sways a little
                let tail = ((y - (len - 70.0)).max(0.0) / 70.0).min(1.0);
                let z = tail * tail * 26.0 + ((u - 0.5) * std::f32::consts::PI).cos() * 3.0;
                V3 { x: (u - 0.5) * paper_w + sway * (y / RECEIPT_H).powi(2) * 8.0, y, z }
            };
            let front = move |_u: f32, v: f32| lerp_c(paper, hex("#EDE8DC"), v * len / RECEIPT_H);
            Sheet {
                size: vec2(paper_w, len),
                cols: 6,
                rows: ((len / 10.0) as usize).clamp(2, 32),
                deform: &deform,
                front: &front,
                back: hex("#E4DFD3"),
                ambient: 0.55,
                gloss: 0.15,
                bias: 0.0,
                alpha,
                ink_bias: 400.0,
            }
            .emit(soup, cam, Some(&ink));
        };
        let len = st.feed * RECEIPT_H + st.drag * 0.25;
        if len > 1.0 {
            let mut cam = Cam::new(pos2(printer.center().x, slot_y));
            cam.focal = 900.0;
            let mut soup = Soup::new();
            draw_paper(&mut soup, &cam, len, (time * 1.3).sin() * st.feed, 1.0);
            let sh = Rect::from_min_size(pos2(paper_x + 6.0, slot_y + 8.0), vec2(paper_w, len));
            paint::soft_shadow(&scene_p, sh, 2.0, 14.0, Vec2::ZERO, Color32::from_black_alpha(50));
            soup.paint(&scene_p);
            // the circled total, inked in once printed
            let total_y = 72.0 + plan.rows.len() as f32 * 13.0 + 26.0;
            let circle_k = sstep(total_y + 30.0, total_y + 80.0, len);
            if circle_k > 0.0 {
                let c = pos2(paper_x + paper_w - 46.0, slot_y + total_y + 2.0);
                let pts: Vec<Pos2> = (0..=48)
                    .map(|i| {
                        let a = -2.6 + i as f32 / 48.0 * 6.6 * circle_k;
                        c + vec2(a.cos() * 44.0, a.sin() * 15.0 + (a * 0.5).sin() * 2.0)
                    })
                    .collect();
                scene_p.add(Shape::line(pts, Stroke::new(1.6, with_alpha(hex("#C62E2E"), 0.85))));
            }
        }
        if let Some((tt, v, a)) = st.torn {
            let mut cam = Cam::new(pos2(printer.center().x, slot_y) + v * tt * 0.5);
            cam.roll = a * 0.4;
            cam.pitch = -a * 0.8;
            let mut soup = Soup::new();
            draw_paper(&mut soup, &cam, RECEIPT_H, 0.0, 1.0 - sstep(0.6, 1.3, tt));
            soup.paint(&scene_p);
        }
        // printer body
        paint::soft_shadow(&scene_p, printer, 12.0, 14.0, vec2(0.0, 6.0), Color32::from_black_alpha(80));
        paint::gradient_rect(&scene_p, printer, 12.0, 90.0, &[hex("#3A3A3E"), hex("#1E1E21")]);
        scene_p.rect_stroke(printer, 12, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.12)), egui::StrokeKind::Inside);
        let slot = Rect::from_center_size(pos2(printer.center().x, slot_y), vec2(paper_w + 14.0, 5.0));
        scene_p.rect_filled(slot, 2.5, hex("#0B0B0C"));
        // LCD
        let lcd = Rect::from_min_size(printer.min + vec2(12.0, 11.0), vec2(96.0, 28.0));
        scene_p.rect_filled(lcd, 5.0, hex("#120C04"));
        let amber = hex("#FFB547");
        let shown_price = self.plans[st.plan.min(n - 1)].price;
        paint::glow(&scene_p, lcd.center(), 30.0, with_alpha(amber, 0.12));
        scene_p.text(lcd.right_center() - vec2(8.0, 0.0), Align2::RIGHT_CENTER, format!("${shown_price:.2}"), FontId::monospace(14.0), amber);
        // plan keys
        for i in 0..n {
            let kr = Rect::from_min_size(pos2(lcd.max.x + 10.0 + i as f32 * 38.0, printer.min.y + 11.0), vec2(32.0, 28.0));
            if kr.max.x > printer.max.x - 8.0 {
                break;
            }
            let on = i == st.plan;
            scene_p.rect_filled(kr.translate(vec2(0.0, if on { 1.5 } else { 0.0 })), 6.0, if on { hex("#4A4A50") } else { hex("#2C2C30") });
            scene_p.circle_filled(kr.center_top() + vec2(0.0, 6.0), 1.6, if on { hex("#7CFF9A") } else { hex("#55555B") });
            scene_p.text(kr.center() + vec2(0.0, 4.0), Align2::CENTER_CENTER, &self.plans[i].name, FontId::proportional(7.5), hex("#D8D8DC"));
        }

        // ---- right column
        let red = hex("#C62E2E");
        painter.text(right.min, Align2::LEFT_TOP, "first invoice is on the house", FontId::proportional(12.0), red);
        painter.text(right.min + vec2(0.0, 22.0), Align2::LEFT_TOP, &self.headline.0, FontId::proportional(30.0), t.foreground);
        painter.text(right.min + vec2(0.0, 56.0), Align2::LEFT_TOP, &self.headline.1, FontId::proportional(30.0), t.foreground);
        painter.text(right.min + vec2(0.0, 96.0), Align2::LEFT_TOP, "Track time, send invoices, get paid the same afternoon.", FontId::proportional(12.5), t.muted_foreground);
        for i in 0..n {
            let r = row_rect(i);
            let on = i == st.plan;
            let fill = if on { if t.dark { t.muted } else { Color32::WHITE } } else { with_alpha(t.muted, 0.35) };
            painter.rect_filled(r, 10.0, fill);
            if on {
                painter.rect_stroke(r, 10, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
            }
            let dot = pos2(r.min.x + 18.0, r.center().y);
            painter.circle_stroke(dot, 6.0, Stroke::new(1.2, if on { t.foreground } else { t.muted_foreground }));
            if on {
                painter.circle_filled(dot, 3.2, t.foreground);
            }
            let p = &self.plans[i];
            painter.text(pos2(r.min.x + 34.0, r.min.y + 9.0), Align2::LEFT_TOP, &p.name, FontId::proportional(14.0), t.foreground);
            painter.text(pos2(r.min.x + 34.0, r.min.y + 27.0), Align2::LEFT_TOP, &p.blurb, FontId::proportional(11.0), t.muted_foreground);
            let price = if p.price == 0.0 { "Free".to_string() } else { format!("${:.0} /mo", p.price) };
            painter.text(pos2(r.max.x - 14.0, r.center().y), Align2::RIGHT_CENTER, price, FontId::proportional(13.0), t.foreground);
        }
        let cta_y = rows_top + n as f32 * (row_h + 6.0) + 14.0;
        let (label, fill) = match st.kept {
            Some(k) => (format!("Receipt kept · check out ${:.0}/mo  →", self.plans[k.min(n - 1)].price), red),
            None => ("Start for free  →".to_string(), t.foreground),
        };
        let g = painter.layout_no_wrap(label, FontId::proportional(13.0), contrast(fill));
        let cta = Rect::from_min_size(pos2(right.min.x, cta_y), vec2(g.size().x + 32.0, 38.0));
        painter.rect_filled(cta, 19.0, fill);
        painter.galley(cta.center() - g.size() * 0.5, g, contrast(fill));
        painter.text(pos2(cta.max.x + 16.0, cta.center().y), Align2::LEFT_CENTER, "Drag the receipt down to tear it off", FontId::proportional(11.0), t.muted_foreground);
        response
    }
}

fn contrast(c: Color32) -> Color32 {
    glowkit_core::color::contrast_text(c)
}
