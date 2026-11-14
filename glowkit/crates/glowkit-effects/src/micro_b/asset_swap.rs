use super::common::{cr, hairline, icon, idle_for, shadow_color, surface, surface_hover, text_width, MicroIcon};
use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_core::anim::spring;
use glowkit_core::color::{contrast_text, with_alpha};
use glowkit_core::{paint, Theme};

/// One asset in an [`AssetSwap`]: a ticker, a name, a brand color and a price in dollars.
#[derive(Clone, Debug, PartialEq)]
pub struct SwapAsset {
    /// Short ticker, e.g. `"BTC"`.
    pub symbol: String,
    /// Full name shown in the picker.
    pub name: String,
    /// Coin color.
    pub color: Color32,
    /// Price of one unit in dollars.
    pub price: f64,
    /// Glyph on the coin (defaults to the first letter of the symbol).
    pub glyph: String,
}

impl SwapAsset {
    /// A new asset.
    pub fn new(symbol: impl Into<String>, name: impl Into<String>, color: Color32, price: f64) -> Self {
        let symbol = symbol.into();
        let glyph = symbol.chars().next().map(String::from).unwrap_or_default();
        Self { symbol, name: name.into(), color, price, glyph }
    }
    /// Set the glyph painted on the coin.
    pub fn glyph(mut self, g: impl Into<String>) -> Self {
        self.glyph = g.into();
        self
    }
}

#[derive(Clone, Copy, Debug)]
struct SwapState {
    /// Asset index for each card (card 0 starts on top).
    asset: [usize; 2],
    /// Card 0 is the "pay" card when false.
    flipped: bool,
    /// Amount being paid.
    amount: f64,
    picker: Option<usize>,
    since: f64,
}

/// A two-card swap widget: "You pay" on top, "You receive" below, joined by a round arrow
/// button. Press the arrow and the cards trade places on a bouncy spring while the arrow spins
/// round; the received amount becomes the new amount paid. Tap a coin chip and the card opens
/// into a list to pick another asset.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{AssetSwap, SwapAsset};
/// let assets = vec![
///     SwapAsset::new("BTC", "Bitcoin", glowkit_core::hex("#F7931A"), 97_700.0),
///     SwapAsset::new("ETH", "Ether", glowkit_core::hex("#627EEA"), 3_400.0),
/// ];
/// ui.add(AssetSwap::new(assets).amount(0.05));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AssetSwap {
    assets: Vec<SwapAsset>,
    amount: f64,
    width: f32,
    corner: f32,
    bounce: f32,
    dark: bool,
    stroke: bool,
    labels: [String; 2],
    autoplay: bool,
    salt: u64,
}

impl AssetSwap {
    /// A swap between the first two of `assets` (any further ones are offered in the picker).
    pub fn new(assets: Vec<SwapAsset>) -> Self {
        Self {
            assets,
            amount: 0.05,
            width: 300.0,
            corner: 20.0,
            bounce: 0.3,
            dark: false,
            stroke: true,
            labels: ["You pay".into(), "You receive".into()],
            autoplay: true,
            salt: 0,
        }
    }
    /// Starting amount paid.
    pub fn amount(mut self, a: f64) -> Self {
        self.amount = a.max(0.0);
        self
    }
    /// Card width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(200.0);
        self
    }
    /// Corner radius of the cards.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Overshoot of the swap (0..1).
    pub fn bounce(mut self, k: f32) -> Self {
        self.bounce = k.clamp(0.0, 1.0);
        self
    }
    /// Dark cards.
    pub fn dark(mut self, on: bool) -> Self {
        self.dark = on;
        self
    }
    /// Hairline outlines (default on).
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Captions of the top and bottom card.
    pub fn labels(mut self, pay: impl Into<String>, receive: impl Into<String>) -> Self {
        self.labels = [pay.into(), receive.into()];
        self
    }
    /// Swap back and forth on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Distinguish several swaps that share the same asset list (state is keyed by it).
    pub fn id_salt(mut self, salt: u64) -> Self {
        self.salt = salt;
        self
    }
}

/// `1234.5` -> `"1,234.50"`.
pub(crate) fn money(v: f64) -> String {
    let cents = (v.abs() * 100.0).round() as u64;
    let int = cents / 100;
    let s = int.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    format!("{}{out}.{:02}", if v < 0.0 { "-" } else { "" }, cents % 100)
}

/// Amount with about five significant digits.
fn amount_str(v: f64) -> String {
    if v >= 1000.0 {
        money(v)
    } else if v >= 1.0 {
        let s = format!("{v:.3}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        let digits = (5 - (v.max(1e-9).log10().floor() as i32 + 1)).clamp(2, 8) as usize;
        let s = format!("{v:.digits$}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn coin(painter: &Painter, c: Pos2, r: f32, a: &SwapAsset, alpha: f32) {
    painter.circle_filled(c, r, with_alpha(a.color, alpha));
    painter.text(c, Align2::CENTER_CENTER, &a.glyph, FontId::proportional(r * 1.15), with_alpha(contrast_text(a.color), alpha));
}

impl Widget for AssetSwap {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let card_h = 112.0;
        let gap = 5.0;
        let size = vec2(self.width, card_h * 2.0 + gap);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let key = self.assets.iter().map(|a| a.symbol.clone()).collect::<Vec<_>>().join("/");
        let id = ui.scope_id().with(("glowkit_asset_swap", key, self.salt));
        let now = ui.input(|i| i.time);
        let n = self.assets.len().max(1);
        let st0: SwapState = glowkit_core::state(ui, id, || SwapState {
            asset: [0, 1.min(n - 1)],
            flipped: false,
            amount: self.amount,
            picker: None,
            since: now,
        });
        if self.assets.len() < 2 {
            return response;
        }
        let slot = |s: usize| Rect::from_min_size(rect.min + vec2(0.0, s as f32 * (card_h + gap)), vec2(self.width, card_h));
        let btn = Rect::from_center_size(rect.center(), vec2(32.0, 32.0));
        let btn_resp = ui.interact(btn, id.with("btn"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        // the card each slot currently shows (slot 0 = pay)
        let card_in = |slot: usize, flipped: bool| if flipped { 1 - slot } else { slot };
        let chip_rect = |r: Rect, painter: &Painter, a: &SwapAsset| {
            let w = text_width(painter, &a.symbol, 12.5) + 50.0;
            Rect::from_min_size(pos2(r.max.x - 14.0 - w, r.max.y - 48.0), vec2(w, 28.0))
        };
        let painter = ui.painter().clone();
        let mut chip_clicked = None;
        if st0.picker.is_none() {
            for s in 0..2 {
                let c = card_in(s, st0.flipped);
                let cr_ = chip_rect(slot(s), &painter, &self.assets[st0.asset[c] % n]);
                if ui.interact(cr_, id.with(("chip", s)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    chip_clicked = Some(c);
                }
            }
        }
        // picker interactions
        let mut pick = None;
        let mut close_picker = false;
        let row_h = 32.0;
        if let Some(c) = st0.picker {
            let s = if card_in(0, st0.flipped) == c { 0 } else { 1 };
            let panel = picker_rect(slot(s), rect, n, row_h);
            let x = Rect::from_center_size(pos2(panel.max.x - 22.0, panel.min.y + 20.0), vec2(22.0, 22.0));
            if ui.interact(x, id.with("x"), Sense::click()).clicked() {
                close_picker = true;
            }
            for i in 0..n {
                let rr = Rect::from_min_size(pos2(panel.min.x + 6.0, panel.min.y + 36.0 + i as f32 * row_h), vec2(panel.width() - 12.0, row_h));
                if ui.interact(rr, id.with(("row", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    pick = Some((c, i));
                }
            }
        }
        let idle = idle_for(ui, id, ui.rect_contains_pointer(rect.expand(30.0)));
        let auto = self.autoplay && idle > 2.0;
        let prices: Vec<f64> = self.assets.iter().map(|a| a.price).collect();
        let st = glowkit_core::with_state(ui, id, || st0, |s| {
            let flip = btn_resp.clicked() || (auto && s.picker.is_none() && now - s.since > 2.6);
            if flip {
                // the amount received becomes the amount paid
                let pay = s.asset[card_in(0, s.flipped)] % n;
                let recv = s.asset[card_in(1, s.flipped)] % n;
                s.amount = s.amount * prices[pay] / prices[recv].max(1e-12) * 0.997;
                s.flipped = !s.flipped;
                s.since = now;
            }
            if let Some(c) = chip_clicked {
                s.picker = Some(c);
            }
            if close_picker {
                s.picker = None;
            }
            if let Some((c, i)) = pick {
                let other = 1 - c;
                if s.asset[other] == i {
                    s.asset[other] = s.asset[c];
                }
                s.asset[c] = i;
                s.picker = None;
            }
            *s
        });
        if auto {
            glowkit_core::animate(ui);
        }
        let k = spring(ui, id.with("flip"), if st.flipped { 1.0 } else { 0.0 }, 260.0, 24.0 - 14.0 * self.bounce);
        let spin = spring(ui, id.with("spin"), if st.flipped { 1.0 } else { 0.0 }, 200.0, 18.0);
        let pick_t = spring(ui, id.with("pick"), if st.picker.is_some() { 1.0 } else { 0.0 }, 320.0, 26.0);
        let last_pick: usize = {
            let v = glowkit_core::state(ui, id.with("lastpick"), || 0usize);
            let v = st.picker.unwrap_or(v);
            glowkit_core::set_state(ui, id.with("lastpick"), v);
            v
        };
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let (bg, fg, sub) = if self.dark {
            (t.primary, t.primary_foreground, with_alpha(t.primary_foreground, 0.55))
        } else {
            (surface(&t), t.foreground, t.muted_foreground)
        };
        let line = if self.stroke { hairline(&t) } else { Stroke::NONE };
        let pay_asset = &self.assets[st.asset[card_in(0, st.flipped)] % n];
        let recv_asset = &self.assets[st.asset[card_in(1, st.flipped)] % n];
        let recv_amount = st.amount * pay_asset.price / recv_asset.price.max(1e-12) * 0.997;
        // card positions: card 0 travels from slot 0 to slot 1 as k goes 0 -> 1
        let y0 = rect.min.y;
        let y1 = rect.min.y + card_h + gap;
        let mut order = [(0usize, y0 + (y1 - y0) * k), (1usize, y1 + (y0 - y1) * k)];
        // the card moving down passes behind
        if k > 0.0 && k < 1.0 {
            order.sort_by(|a, b| if st.flipped { b.0.cmp(&a.0) } else { a.0.cmp(&b.0) });
        }
        let mid = (k * std::f32::consts::PI).sin().abs();
        for (c, y) in order {
            let asset = &self.assets[st.asset[c] % n];
            let is_pay = card_in(0, st.flipped) == c;
            let scale = 1.0 - 0.035 * mid;
            let r = Rect::from_min_size(pos2(rect.min.x, y), vec2(self.width, card_h));
            let r = Rect::from_center_size(r.center(), r.size() * scale);
            paint::soft_shadow(&painter, r, self.corner, 14.0, vec2(0.0, 3.0), shadow_color(&t, 0.45));
            painter.rect(r, cr(self.corner), bg, line, egui::StrokeKind::Inside);
            let label = if is_pay { &self.labels[0] } else { &self.labels[1] };
            painter.text(r.min + vec2(16.0, 16.0), Align2::LEFT_TOP, label, FontId::proportional(12.0), sub);
            let amt = if is_pay { st.amount } else { recv_amount };
            painter.text(r.min + vec2(16.0, 36.0), Align2::LEFT_TOP, amount_str(amt), FontId::proportional(27.0), fg);
            painter.text(r.min + vec2(16.0, 72.0), Align2::LEFT_TOP, format!("${}", money(amt * asset.price)), FontId::proportional(12.0), sub);
            let chip = chip_rect(r, &painter, asset);
            painter.rect_filled(chip, cr(14.0), if self.dark { with_alpha(fg, 0.1) } else { surface_hover(&t) });
            coin(&painter, pos2(chip.min.x + 15.0, chip.center().y), 9.0, asset, 1.0);
            painter.text(pos2(chip.min.x + 29.0, chip.center().y), Align2::LEFT_CENTER, &asset.symbol, FontId::proportional(12.5), fg);
            icon(&painter, MicroIcon::ChevronDown, pos2(chip.max.x - 13.0, chip.center().y + 0.5), 9.0, sub, 1.4);
        }
        // the round swap button, notched into the stage color
        let press = if btn_resp.is_pointer_button_down_on() { 0.9 } else { 1.0 };
        painter.circle_filled(btn.center(), 19.0, t.muted);
        painter.circle(btn.center(), 15.0 * press, bg, line);
        let ang = std::f32::consts::FRAC_PI_2 + spin * std::f32::consts::PI;
        super::common::arrow(&painter, btn.center(), 12.0 * press, ang, Stroke::new(1.6, fg));
        // asset picker panel
        if pick_t > 0.01 {
            let c = last_pick;
            let s = if card_in(0, st.flipped) == c { 0 } else { 1 };
            let full = picker_rect(slot(s), rect, n, row_h);
            let panel = Rect::from_min_max(full.min, pos2(full.max.x, full.min.y + full.height() * pick_t.clamp(0.0, 1.05)));
            let a = pick_t.clamp(0.0, 1.0);
            paint::soft_shadow(&painter, panel, self.corner, 24.0, vec2(0.0, 8.0), shadow_color(&t, 0.9 * a));
            painter.rect(panel, cr(self.corner), with_alpha(bg, a), line, egui::StrokeKind::Inside);
            let clip = painter.with_clip_rect(panel);
            let title = if s == 0 { &self.labels[0] } else { &self.labels[1] };
            clip.text(panel.min + vec2(16.0, 20.0), Align2::LEFT_CENTER, title, FontId::proportional(12.0), with_alpha(sub, a));
            icon(&clip, MicroIcon::Close, pos2(panel.max.x - 22.0, panel.min.y + 20.0), 9.0, with_alpha(sub, a), 1.4);
            let hover = ui.input(|i| i.pointer.hover_pos());
            for (i, asset) in self.assets.iter().enumerate() {
                let e = glowkit_core::anim::stagger(pick_t, i, 0.06, 0.6);
                let rr = Rect::from_min_size(pos2(panel.min.x + 6.0, panel.min.y + 36.0 + i as f32 * row_h), vec2(panel.width() - 12.0, row_h));
                if hover.is_some_and(|p| rr.contains(p)) {
                    clip.rect_filled(rr, cr(10.0), if self.dark { with_alpha(fg, 0.08) } else { surface_hover(&t) });
                }
                let y = rr.center().y + (1.0 - e) * 6.0;
                coin(&clip, pos2(rr.min.x + 18.0, y), 9.0, asset, e);
                clip.text(pos2(rr.min.x + 34.0, y), Align2::LEFT_CENTER, &asset.symbol, FontId::proportional(12.5), with_alpha(fg, e));
                let sw = text_width(&clip, &asset.symbol, 12.5);
                clip.text(pos2(rr.min.x + 42.0 + sw, y), Align2::LEFT_CENTER, &asset.name, FontId::proportional(12.0), with_alpha(sub, e));
                if st.asset[c] == i {
                    icon(&clip, MicroIcon::Check, pos2(rr.max.x - 16.0, y), 11.0, with_alpha(fg, e), 1.6);
                }
            }
        }
        response.union(btn_resp)
    }
}

/// The picker opens over the card, kept inside the widget when it is taller than one card.
fn picker_rect(card: Rect, bounds: Rect, n: usize, row_h: f32) -> Rect {
    let h = 44.0 + n as f32 * row_h;
    let top = card.min.y.min(bounds.max.y - h).max(bounds.min.y);
    Rect::from_min_size(pos2(card.min.x, top), vec2(card.width(), h))
}
