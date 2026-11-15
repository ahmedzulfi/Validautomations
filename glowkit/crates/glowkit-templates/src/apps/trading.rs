//! Trading terminal template: live candlesticks with indicators, order book depth, trade tape,
//! positions with PnL, order entry and a watchlist with sparklines.

use egui::{
    pos2, vec2, Align2, Color32, CursorIcon, FontId, Id, Key, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui,
};
use glowkit_components::{toast, LineIcon, Toast, UiIcon};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{Rng, Theme};

use super::common::{demo_notes, demo_user};
use super::prokit as pk;
use crate::shells::kit;
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification, Workspace};

#[derive(Clone, Copy, Debug)]
struct Bar {
    o: f64,
    h: f64,
    l: f64,
    c: f64,
    v: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct Level {
    size: f64,
    flash: f64,
    up: bool,
}

#[derive(Clone, Debug)]
struct Print {
    time: String,
    price: f64,
    size: f64,
    buy: bool,
    born: f64,
}

/// An open position in the [`TradingTerminalApp`].
#[derive(Clone, Debug)]
pub struct TermPosition {
    /// Watchlist index of the instrument.
    pub symbol: usize,
    /// Long (true) or short.
    pub long: bool,
    /// Quantity in base units.
    pub qty: f64,
    /// Average entry price.
    pub entry: f64,
}

#[derive(Clone, Debug)]
struct OpenOrder {
    symbol: usize,
    buy: bool,
    qty: f64,
    price: f64,
    placed: String,
}

#[derive(Clone, Debug)]
struct Watch {
    sym: &'static str,
    name: &'static str,
    price: f64,
    open: f64,
    hist: Vec<f64>,
}

const SYMBOLS: [(&str, &str, f64); 7] = [
    ("BTC", "Bitcoin", 64_380.0),
    ("ETH", "Ether", 3_412.6),
    ("SOL", "Solana", 148.22),
    ("AVAX", "Avalanche", 36.41),
    ("LINK", "Chainlink", 17.853),
    ("DOT", "Polkadot", 7.124),
    ("ARB", "Arbitrum", 1.1432),
];
const TFS: [&str; 6] = ["1m", "5m", "15m", "1h", "4h", "1D"];
const TF_MIN: [u32; 6] = [1, 5, 15, 60, 240, 1440];
const LEVELS: usize = 14;
const TICK_SECS: f64 = 0.25;
const TICKS_PER_BAR: u64 = 10;

fn tick_size(p: f64) -> f64 {
    10f64.powi(p.log10().floor() as i32 - 4) * 5.0 / 10.0
}

fn decimals(p: f64) -> usize {
    let t = tick_size(p);
    if t >= 0.5 {
        1
    } else if t >= 0.05 {
        2
    } else if t >= 0.005 {
        3
    } else {
        4
    }
}

fn px(v: f64) -> String {
    kit::thousands(v, decimals(v.abs().max(1e-6)))
}

fn gen_bars(seed: u64, base: f64, n: usize, vol: f64) -> Vec<Bar> {
    let mut rng = Rng::new(seed);
    let mut p = base * (1.0 - vol * 6.0);
    let mut drift = 0.0;
    (0..n)
        .map(|i| {
            if i % 18 == 0 {
                drift = (rng.f32() as f64 - 0.45) * vol * 0.6;
            }
            let o = p;
            let c = o * (1.0 + drift + rng.normal() as f64 * vol);
            let h = o.max(c) * (1.0 + rng.f32() as f64 * vol * 0.8);
            let l = o.min(c) * (1.0 - rng.f32() as f64 * vol * 0.8);
            p = c;
            Bar { o, h, l, c, v: 20.0 + rng.f32() as f64 * 80.0 + ((c - o).abs() / (o * vol)) * 30.0 }
        })
        .collect()
}

fn ema(vals: &[f64], n: usize) -> Vec<f64> {
    let k = 2.0 / (n as f64 + 1.0);
    let mut out = Vec::with_capacity(vals.len());
    let mut e = vals.first().copied().unwrap_or(0.0);
    for v in vals {
        e = v * k + e * (1.0 - k);
        out.push(e);
    }
    out
}

fn bollinger(vals: &[f64], n: usize) -> Vec<(f64, f64)> {
    (0..vals.len())
        .map(|i| {
            let a = i.saturating_sub(n - 1);
            let w = &vals[a..=i];
            let m = w.iter().sum::<f64>() / w.len() as f64;
            let sd = (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / w.len() as f64).sqrt();
            (m + 2.0 * sd, m - 2.0 * sd)
        })
        .collect()
}

/// A crypto / stock trading terminal: a live candlestick chart (EMA 9 / 21, Bollinger bands,
/// volume, crosshair with OHLC readout, wheel zoom and drag pan), an order book with depth
/// bars and flashing level updates (click a level to use its price), a trade tape, positions
/// with live PnL and one-click close, an order entry panel (buy / sell, limit / market, size
/// presets) and a watchlist with sparklines. Dark terminal look by default
/// ([`dark_by_default`](Self::dark_by_default)); the light theme is fully supported.
///
/// Keys: B / S pick the side, 1-6 switch the timeframe.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::TradingTerminalApp::default();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct TradingTerminalApp {
    /// Active route.
    pub route: String,
    /// Shell navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// Open positions.
    pub positions: Vec<TermPosition>,
    /// Switch the shell to the dark theme on first show.
    pub dark_by_default: bool,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    symbol: usize,
    tf: usize,
    bars: Vec<Bar>,
    price: f64,
    prev_price: f64,
    price_flash: f64,
    asks: Vec<Level>,
    bids: Vec<Level>,
    tape: Vec<Print>,
    orders: Vec<OpenOrder>,
    fills: Vec<Print>,
    watch: Vec<Watch>,
    tick_n: u64,
    last_tick: f64,
    seed: u64,
    side: usize,
    otype: usize,
    price_s: String,
    qty_s: String,
    bottom_tab: usize,
    indicators: [bool; 4],
    zoom: f32,
    pan: f32,
    applied_theme: bool,
    balance: f64,
}

impl Default for TradingTerminalApp {
    fn default() -> Self {
        let watch = SYMBOLS
            .iter()
            .enumerate()
            .map(|(i, (s, n, p))| {
                let hist: Vec<f64> = gen_bars(100 + i as u64, *p, 40, 0.004).iter().map(|b| b.c).collect();
                let open = hist[0];
                let last = *hist.last().unwrap_or(p);
                Watch { sym: s, name: n, price: last, open, hist }
            })
            .collect();
        let mut app = Self {
            route: "trade".into(),
            nav: vec![
                NavSection::new("Markets")
                    .item(NavItem::new("trade", "Trade").icon(AppIcon::TrendUp))
                    .item(NavItem::new("portfolio", "Portfolio").icon(AppIcon::Wallet))
                    .item(NavItem::new("orders", "Orders").icon(AppIcon::ListChecks).badge("2"))
                    .item(NavItem::new("alerts", "Alerts").icon(LineIcon::Bell)),
                NavSection::new("Account")
                    .item(NavItem::new("funding", "Funding").icon(LineIcon::CreditCard))
                    .item(NavItem::new("settings", "Settings").icon(LineIcon::Settings)),
            ],
            notifications: demo_notes(),
            positions: vec![
                TermPosition { symbol: 0, long: true, qty: 0.85, entry: 63_412.5 },
                TermPosition { symbol: 1, long: false, qty: 12.0, entry: 3_448.2 },
                TermPosition { symbol: 2, long: true, qty: 140.0, entry: 151.06 },
            ],
            dark_by_default: true,
            scoped_theme: false,
            symbol: 0,
            tf: 0,
            bars: Vec::new(),
            price: 0.0,
            prev_price: 0.0,
            price_flash: -10.0,
            asks: vec![Level::default(); LEVELS],
            bids: vec![Level::default(); LEVELS],
            tape: Vec::new(),
            orders: Vec::new(),
            fills: Vec::new(),
            watch,
            tick_n: 0,
            last_tick: 0.0,
            seed: 7,
            side: 0,
            otype: 0,
            price_s: String::new(),
            qty_s: "0.25".into(),
            bottom_tab: 0,
            indicators: [true, true, true, true],
            zoom: 9.0,
            pan: 0.0,
            applied_theme: false,
            balance: 48_210.55,
        };
        app.load_symbol(0);
        let wp = |i: usize, k: f64| {
            let v = app.watch[i].price * k;
            let ts = tick_size(v);
            (v / ts).round() * ts
        };
        app.positions = vec![
            TermPosition { symbol: 0, long: true, qty: 0.85, entry: wp(0, 0.988) },
            TermPosition { symbol: 1, long: false, qty: 12.0, entry: wp(1, 1.006) },
            TermPosition { symbol: 2, long: true, qty: 140.0, entry: wp(2, 1.012) },
        ];
        app.orders = vec![
            OpenOrder { symbol: 0, buy: true, qty: 0.4, price: wp(0, 0.985), placed: "13:42:10".into() },
            OpenOrder { symbol: 1, buy: false, qty: 5.0, price: wp(1, 1.02), placed: "12:08:55".into() },
        ];
        app
    }
}

impl TradingTerminalApp {
    fn load_symbol(&mut self, s: usize) {
        self.symbol = s;
        let base = self.watch[s].price;
        let vol = 0.0018 * (TF_MIN[self.tf] as f64).sqrt().min(12.0) * if s == 0 { 1.0 } else { 1.4 };
        let mut bars = gen_bars(31 + s as u64 * 17 + self.tf as u64 * 5, base, 140, vol);
        // rescale so the last close equals the live price
        let k = base / bars.last().map_or(base, |b| b.c);
        for b in &mut bars {
            b.o *= k;
            b.h *= k;
            b.l *= k;
            b.c *= k;
        }
        self.bars = bars;
        self.price = base;
        self.prev_price = base;
        self.price_s = px(base);
        self.pan = 0.0;
        let mut rng = Rng::new(s as u64 + 3);
        for l in self.asks.iter_mut().chain(self.bids.iter_mut()) {
            *l = Level { size: 0.05 + rng.f32() as f64 * 2.4, flash: -10.0, up: true };
        }
        self.tape.clear();
    }

    fn now_label(&self, offset: u64) -> String {
        let secs = 14 * 3600 + 27 * 60 + (self.tick_n + offset) / 4;
        format!("{:02}:{:02}:{:02}", secs / 3600 % 24, secs / 60 % 60, secs % 60)
    }

    fn tick(&mut self, now: f64, ctx: &egui::Context) {
        if self.last_tick == 0.0 {
            self.last_tick = now;
            // pre-fill the tape
            for _ in 0..24 {
                self.step(now - 100.0, ctx);
            }
        }
        let mut guard = 0;
        while now - self.last_tick >= TICK_SECS && guard < 8 {
            self.last_tick += TICK_SECS;
            self.step(now, ctx);
            guard += 1;
        }
        if guard == 8 {
            self.last_tick = now;
        }
    }

    fn step(&mut self, now: f64, ctx: &egui::Context) {
        self.tick_n += 1;
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let mut rng = Rng::new(self.seed);
        let vol = 0.00045 * if self.symbol == 0 { 1.0 } else { 1.5 };
        self.prev_price = self.price;
        let ts = tick_size(self.price);
        let np = self.price * (1.0 + rng.normal() as f64 * vol);
        self.price = (np / ts).round() * ts;
        if self.price != self.prev_price {
            self.price_flash = now;
        }
        // candles
        if self.tick_n % TICKS_PER_BAR == 0 {
            let c = self.price;
            self.bars.push(Bar { o: c, h: c, l: c, c, v: 1.0 });
            if self.bars.len() > 400 {
                self.bars.remove(0);
            }
        }
        if let Some(b) = self.bars.last_mut() {
            b.c = self.price;
            b.h = b.h.max(self.price);
            b.l = b.l.min(self.price);
            b.v += rng.f32() as f64 * 9.0;
        }
        // book
        for side in 0..2 {
            for _ in 0..3 {
                let i = rng.below(LEVELS);
                let lv = if side == 0 { &mut self.asks[i] } else { &mut self.bids[i] };
                let old = lv.size;
                let nv = (old * (0.55 + rng.f32() as f64 * 0.9) + rng.f32() as f64 * 0.2).clamp(0.01, 6.0);
                lv.up = nv > old;
                lv.size = nv;
                lv.flash = now;
            }
        }
        // prints
        let n = rng.below(3);
        for _ in 0..n {
            let buy = rng.bool(if self.price >= self.prev_price { 0.62 } else { 0.38 });
            let off = rng.below(3) as f64 * ts;
            let p = if buy { self.price + off } else { self.price - off };
            let size = (rng.f32() as f64).powi(3) * 2.5 + 0.001;
            self.tape.insert(0, Print { time: self.now_label(0), price: p, size, buy, born: now });
        }
        self.tape.truncate(60);
        // watchlist
        let active = self.symbol;
        for (k, w) in self.watch.iter_mut().enumerate() {
            if k == active {
                w.price = self.price;
            } else {
                let t = tick_size(w.price);
                w.price = ((w.price * (1.0 + rng.normal() as f64 * 0.0006)) / t).round() * t;
            }
            if self.tick_n % TICKS_PER_BAR == 0 {
                w.hist.remove(0);
                w.hist.push(w.price);
            } else if let Some(l) = w.hist.last_mut() {
                *l = w.price;
            }
        }
        // limit fills
        let mut filled = Vec::new();
        for (k, o) in self.orders.iter().enumerate() {
            let p = self.watch[o.symbol].price;
            if (o.buy && p <= o.price) || (!o.buy && p >= o.price) {
                filled.push(k);
            }
        }
        for k in filled.into_iter().rev() {
            let o = self.orders.remove(k);
            self.apply_fill(o.symbol, o.buy, o.qty, o.price, now);
            toast(
                ctx,
                Toast::success("Limit order filled").description(format!(
                    "{} {} {} @ {}",
                    if o.buy { "Bought" } else { "Sold" },
                    o.qty,
                    SYMBOLS[o.symbol].0,
                    px(o.price)
                )),
            );
        }
    }

    fn apply_fill(&mut self, symbol: usize, buy: bool, qty: f64, price: f64, now: f64) {
        self.fills
            .insert(0, Print { time: self.now_label(0), price, size: if buy { qty } else { -qty }, buy, born: now });
        if let Some(p) = self.positions.iter_mut().find(|p| p.symbol == symbol) {
            let signed = if p.long { p.qty } else { -p.qty };
            let add = if buy { qty } else { -qty };
            let new = signed + add;
            if new.abs() < 1e-9 {
                self.positions.retain(|p| p.symbol != symbol);
                return;
            }
            if signed.signum() == add.signum() {
                p.entry = (p.entry * signed.abs() + price * qty) / new.abs();
            } else if new.signum() != signed.signum() {
                p.entry = price;
            }
            p.long = new > 0.0;
            p.qty = new.abs();
        } else {
            self.positions.push(TermPosition { symbol, long: buy, qty, entry: price });
        }
    }

    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Sidebar)
            .start_collapsed(true)
            .workspaces(vec![Workspace::new("Meridian Pro", "Margin · Tier 2")])
            .user(demo_user())
            .notifications(&mut notes)
            .padding(0.0)
            .scoped_theme(self.scoped_theme)
            .id_salt("trading_terminal_app")
            .show(ui, |ui, sc| {
                if self.dark_by_default && !self.applied_theme {
                    self.applied_theme = true;
                    if !Theme::of(ui).dark {
                        sc.set_theme(Theme::dark());
                    }
                }
                if sc.route.as_str() == "trade" {
                    self.terminal(ui, sc.rect);
                } else {
                    super::analytics::placeholder(ui, sc.rect.shrink(24.0), sc.route);
                }
            });
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
    }

    fn terminal(&mut self, ui: &mut Ui, rect: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("trading");
        let now = ui.input(|i| i.time);
        self.tick(now, &ui.ctx().clone());
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
        if !pk::typing(ui) {
            let keys = ui.input(|i| {
                (
                    i.key_pressed(Key::B),
                    i.key_pressed(Key::S),
                    [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6].map(|k| i.key_pressed(k)),
                )
            });
            if keys.0 {
                self.side = 0;
            }
            if keys.1 {
                self.side = 1;
            }
            if let Some(k) = keys.2.iter().position(|b| *b) {
                if k != self.tf {
                    self.tf = k;
                    self.load_symbol(self.symbol);
                }
            }
        }
        let p = ui.painter().clone();
        let bg = if t.dark { mix(t.background, Color32::BLACK, 0.25) } else { mix(t.background, t.foreground, 0.03) };
        p.rect_filled(rect, 0.0, bg);
        let strip = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + 52.0));
        self.ticker(ui, strip, now);
        let body = Rect::from_min_max(pos2(rect.min.x + 6.0, strip.max.y + 6.0), rect.max - vec2(6.0, 6.0));
        let cols = kit::split_x(body, &[0.0, 248.0, 258.0], 6.0);
        let (left, mid, right) = (cols[0], cols[1], cols[2]);
        let bottom_h = (left.height() * 0.32).clamp(150.0, 230.0);
        let chart_r = Rect::from_min_max(left.min, pos2(left.max.x, left.max.y - bottom_h - 6.0));
        let pos_r = Rect::from_min_max(pos2(left.min.x, chart_r.max.y + 6.0), left.max);
        let book_h = (mid.height() * 0.6).round();
        let book_r = Rect::from_min_size(mid.min, vec2(mid.width(), book_h));
        let tape_r = Rect::from_min_max(pos2(mid.min.x, book_r.max.y + 6.0), mid.max);
        let entry_r = Rect::from_min_size(right.min, vec2(right.width(), 356.0f32.min(right.height() - 150.0)));
        let watch_r = Rect::from_min_max(pos2(right.min.x, entry_r.max.y + 6.0), right.max);
        for r in [chart_r, pos_r, book_r, tape_r, entry_r, watch_r] {
            p.rect_filled(r, 6.0, t.card);
            p.rect_stroke(r, 6.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
        }
        kit::child(ui, chart_r, |ui| self.chart(ui, chart_r, id.with("chart"), now));
        kit::child(ui, pos_r, |ui| self.bottom(ui, pos_r, id.with("bottom"), now));
        kit::child(ui, book_r, |ui| self.book(ui, book_r, id.with("book"), now));
        kit::child(ui, tape_r, |ui| self.trades(ui, tape_r, now));
        kit::child(ui, entry_r, |ui| self.entry(ui, entry_r, id.with("entry"), now));
        kit::child(ui, watch_r, |ui| self.watchlist(ui, watch_r, id.with("watch")));
    }

    fn up_down(t: &Theme) -> (Color32, Color32) {
        if t.dark {
            (Color32::from_rgb(38, 194, 129), Color32::from_rgb(240, 84, 92))
        } else {
            (Color32::from_rgb(12, 158, 98), Color32::from_rgb(218, 54, 66))
        }
    }

    fn ticker(&mut self, ui: &mut Ui, r: Rect, now: f64) {
        let t = Theme::of(ui);
        let (up, dn) = Self::up_down(&t);
        let p = ui.painter().clone();
        p.rect_filled(r, 0.0, t.card);
        kit::hline(&p, r.min.x, r.max.x, r.max.y, &t);
        let w = &self.watch[self.symbol];
        let cy = r.center().y;
        let mut x = r.min.x + 16.0;
        p.circle_filled(pos2(x + 12.0, cy), 12.0, pk::soft(&t, t.chart[self.symbol % 6], 2.0));
        kit::text_at(
            &p,
            pos2(x + 12.0, cy),
            Align2::CENTER_CENTER,
            &w.sym[..1],
            12.0,
            pk::tone_fg(&t, t.chart[self.symbol % 6]),
            20.0,
        );
        x += 32.0;
        let sr = kit::text_at(
            &p,
            pos2(x, cy - 8.0),
            Align2::LEFT_CENTER,
            &format!("{}-USD", w.sym),
            14.0,
            t.foreground,
            110.0,
        );
        kit::text_at(
            &p,
            pos2(x, cy + 10.0),
            Align2::LEFT_CENTER,
            &format!("{} · Perpetual", w.name),
            11.0,
            t.muted_foreground,
            140.0,
        );
        x = sr.max.x.max(x + 110.0) + 18.0;
        let fl = (1.0 - ((now - self.price_flash) / 0.6) as f32).clamp(0.0, 1.0);
        let dir_up = self.price >= self.prev_price;
        let pc = if dir_up { up } else { dn };
        let pr = pk::mono(
            &p,
            pos2(x, cy),
            Align2::LEFT_CENTER,
            &px(self.price),
            20.0,
            t.foreground.lerp_to_gamma(pc, 0.4 + 0.6 * fl),
            180.0,
        );
        x = pr.max.x + 22.0;
        let chg = (w.price / w.open - 1.0) * 100.0;
        let lo = w.hist.iter().copied().fold(f64::MAX, f64::min).min(w.price);
        let hi = w.hist.iter().copied().fold(f64::MIN, f64::max).max(w.price);
        let funding_left = 3 * 3600 + 12 * 60 + 44 - (self.tick_n / 4) as i64;
        let stats = [
            (
                "24h change",
                format!("{}{:.2}%", if chg >= 0.0 { "+" } else { "" }, chg),
                Some(if chg >= 0.0 { up } else { dn }),
            ),
            ("24h high", px(hi), None),
            ("24h low", px(lo), None),
            (
                "24h volume",
                format!("{} {}", glowkit_charts::format_compact(18_400.0 + self.tick_n as f64 * 0.7), w.sym),
                None,
            ),
            (
                "Funding / next",
                format!("0.0100% · {:02}:{:02}:{:02}", funding_left / 3600, funding_left / 60 % 60, funding_left % 60),
                None,
            ),
        ];
        let right_limit = r.max.x - 16.0;
        for (cap, val, c) in stats {
            let vw = kit::text_w(&p, &val, 12.5).max(kit::text_w(&p, cap, 10.5)) + 6.0;
            if x + vw > right_limit {
                break;
            }
            kit::text_at(&p, pos2(x, cy - 8.0), Align2::LEFT_CENTER, cap, 10.5, t.muted_foreground, vw);
            pk::mono(&p, pos2(x, cy + 9.0), Align2::LEFT_CENTER, &val, 12.0, c.unwrap_or(t.foreground), vw + 20.0);
            x += vw + 22.0;
        }
    }

    // -----------------------------------------------------------------------------------------
    // chart
    // -----------------------------------------------------------------------------------------

    fn chart(&mut self, ui: &mut Ui, r: Rect, id: Id, now: f64) {
        let t = Theme::of(ui);
        let (up, dn) = Self::up_down(&t);
        let p = ui.painter().clone();
        // header: timeframes + indicators
        let hy = r.min.y + 18.0;
        let mut x = r.min.x + 10.0;
        for (k, tf) in TFS.iter().enumerate() {
            let w = kit::text_w(&p, tf, 12.0) + 14.0;
            let br = Rect::from_min_size(pos2(x, hy - 11.0), vec2(w, 22.0));
            let resp = ui.interact(br, id.with(("tf", k)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let hv = kit::hover(ui, id.with(("tfh", k)), resp.hovered());
            let on = self.tf == k;
            if on {
                p.rect_filled(br, 5.0, kit::active_fill(&t, t.card));
            } else if hv > 0.01 {
                p.rect_filled(br, 5.0, kit::hover_fill(&t, hv));
            }
            p.text(
                br.center(),
                Align2::CENTER_CENTER,
                *tf,
                FontId::proportional(12.0),
                if on { t.foreground } else { t.muted_foreground.lerp_to_gamma(t.foreground, hv) },
            );
            kit::tip(&resp, &format!("{tf} candles"), Some(&format!("{}", k + 1)));
            if resp.clicked() && !on {
                self.tf = k;
                self.load_symbol(self.symbol);
            }
            x += w + 2.0;
        }
        kit::vline(&p, x + 6.0, hy - 9.0, hy + 9.0, &t);
        x += 14.0;
        let ind_names = ["EMA 9", "EMA 21", "BB 20", "Volume"];
        let ind_cols = [t.chart[3], t.chart[1], t.chart[4], t.muted_foreground];
        for (k, n) in ind_names.iter().enumerate() {
            let w = pk::chip_w(&p, n);
            if x + w > r.max.x - 10.0 {
                break;
            }
            let cr = Rect::from_min_size(pos2(x, hy - 10.0), vec2(w, 20.0));
            if pk::chip(ui, cr, id.with(("ind", k)), n, self.indicators[k], ind_cols[k]).clicked() {
                self.indicators[k] = !self.indicators[k];
            }
            x += w + 5.0;
        }
        // plot area
        let axis_w = 70.0;
        let area = Rect::from_min_max(pos2(r.min.x + 8.0, r.min.y + 36.0), pos2(r.max.x - axis_w, r.max.y - 22.0));
        let resp = ui.interact(area, id.with("plot"), Sense::click_and_drag());
        // zoom + pan
        if resp.hovered() {
            let (scroll, zoom_delta) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            if scroll != 0.0 || zoom_delta != 1.0 {
                self.zoom = (self.zoom * (1.0 + scroll * 0.0025) * zoom_delta).clamp(3.0, 28.0);
            }
        }
        if resp.dragged() {
            self.pan =
                (self.pan + resp.drag_delta().x / self.zoom).clamp(0.0, (self.bars.len() as f32 - 20.0).max(0.0));
        }
        if resp.double_clicked() {
            self.pan = 0.0;
            self.zoom = 9.0;
        }
        let spacing = self.zoom;
        let n_vis = ((area.width() - spacing) / spacing).floor().max(5.0) as usize;
        let end = self.bars.len().saturating_sub(self.pan.round() as usize);
        let start = end.saturating_sub(n_vis);
        let closes: Vec<f64> = self.bars.iter().map(|b| b.c).collect();
        let e9 = ema(&closes, 9);
        let e21 = ema(&closes, 21);
        let bb = bollinger(&closes, 20);
        let vis = &self.bars[start..end];
        let mut lo = vis.iter().map(|b| b.l).fold(f64::MAX, f64::min);
        let mut hi = vis.iter().map(|b| b.h).fold(f64::MIN, f64::max);
        if self.indicators[2] {
            for (a, b) in &bb[start..end] {
                hi = hi.max(*a);
                lo = lo.min(*b);
            }
        }
        let pad = (hi - lo).max(self.price * 0.0005) * 0.08;
        let lo_t = kit::follow(ui.ctx(), id.with("lo"), (lo - pad) as f32, 10.0) as f64;
        let hi_t = kit::follow(ui.ctx(), id.with("hi"), (hi + pad) as f32, 10.0) as f64;
        let vol_h = if self.indicators[3] { area.height() * 0.18 } else { 0.0 };
        let price_area = Rect::from_min_max(area.min, pos2(area.max.x, area.max.y - vol_h));
        let y_of = |v: f64| price_area.max.y - ((v - lo_t) / (hi_t - lo_t).max(1e-9)) as f32 * price_area.height();
        let x_of = |i: usize| area.max.x - spacing * 0.5 - (end - 1 - i) as f32 * spacing - 4.0;
        // grid + price axis
        let grid = with_alpha(t.border, 0.6);
        let steps = 6;
        for k in 0..=steps {
            let v = lo_t + (hi_t - lo_t) * k as f64 / steps as f64;
            let y = y_of(v);
            if y < price_area.min.y + 4.0 || y > price_area.max.y - 2.0 {
                continue;
            }
            p.line_segment([pos2(area.min.x, y), pos2(area.max.x, y)], Stroke::new(1.0, grid));
            pk::mono(
                &p,
                pos2(area.max.x + 8.0, y),
                Align2::LEFT_CENTER,
                &px(v),
                10.5,
                t.muted_foreground,
                axis_w - 10.0,
            );
        }
        let tf_min = TF_MIN[self.tf] as i64;
        let every = ((70.0 / spacing).ceil() as usize).max(1);
        for i in start..end {
            if i % every != 0 {
                continue;
            }
            let x = x_of(i);
            p.line_segment([pos2(x, area.min.y), pos2(x, area.max.y)], Stroke::new(1.0, with_alpha(grid, 0.5)));
            let mins = 14 * 60 + 27 - (self.bars.len() as i64 - 1 - i as i64) * tf_min;
            let label = if tf_min >= 1440 {
                format!("Oct {}", (30 + mins / 1440).rem_euclid(30) + 1)
            } else {
                format!("{:02}:{:02}", mins.div_euclid(60).rem_euclid(24), mins.rem_euclid(60))
            };
            p.text(pos2(x, r.max.y - 11.0), Align2::CENTER_CENTER, label, FontId::monospace(10.0), t.muted_foreground);
        }
        // bollinger band
        if self.indicators[2] && vis.len() > 1 {
            let mut m = egui::Mesh::default();
            let fill = with_alpha(t.chart[4], if t.dark { 0.08 } else { 0.07 });
            for (k, i) in (start..end).enumerate() {
                let x = x_of(i);
                m.colored_vertex(pos2(x, y_of(bb[i].0)), fill);
                m.colored_vertex(pos2(x, y_of(bb[i].1)), fill);
                if k > 0 {
                    let b = (k as u32) * 2;
                    m.add_triangle(b - 2, b - 1, b);
                    m.add_triangle(b - 1, b + 1, b);
                }
            }
            p.add(Shape::mesh(m));
            for side in 0..2 {
                let pts: Vec<Pos2> =
                    (start..end).map(|i| pos2(x_of(i), y_of(if side == 0 { bb[i].0 } else { bb[i].1 }))).collect();
                p.add(Shape::line(pts, Stroke::new(1.0, with_alpha(t.chart[4], 0.5))));
            }
        }
        // volume
        if self.indicators[3] {
            let vmax = vis.iter().map(|b| b.v).fold(1.0, f64::max);
            for (k, b) in vis.iter().enumerate() {
                let i = start + k;
                let x = x_of(i);
                let h = (b.v / vmax) as f32 * (vol_h - 6.0);
                let c = if b.c >= b.o { up } else { dn };
                let w = (spacing * 0.62).max(1.0);
                p.rect_filled(
                    Rect::from_min_max(pos2(x - w / 2.0, area.max.y - h), pos2(x + w / 2.0, area.max.y)),
                    0.0,
                    with_alpha(c, 0.32),
                );
            }
        }
        // candles
        for (k, b) in vis.iter().enumerate() {
            let i = start + k;
            let x = x_of(i);
            let c = if b.c >= b.o { up } else { dn };
            p.line_segment([pos2(x, y_of(b.h)), pos2(x, y_of(b.l))], Stroke::new(1.0, c));
            let w = (spacing * 0.64).max(1.0);
            let (y0, y1) = (y_of(b.o.max(b.c)), y_of(b.o.min(b.c)));
            let body = Rect::from_min_max(pos2(x - w / 2.0, y0), pos2(x + w / 2.0, y1.max(y0 + 1.0)));
            p.rect_filled(body, 0.5, c);
        }
        // EMAs
        for (k, (on, series, col)) in
            [(self.indicators[0], &e9, ind_cols[0]), (self.indicators[1], &e21, ind_cols[1])].into_iter().enumerate()
        {
            if !on || vis.len() < 2 {
                continue;
            }
            let _ = k;
            let pts: Vec<Pos2> = (start..end).map(|i| pos2(x_of(i), y_of(series[i]))).collect();
            p.add(Shape::line(pts, Stroke::new(1.4, col)));
        }
        // last price line
        let dir_up = self.price >= self.prev_price;
        let lc = if dir_up { up } else { dn };
        let ly = y_of(self.price).clamp(price_area.min.y, price_area.max.y);
        let mut xx = area.min.x;
        while xx < area.max.x {
            p.line_segment([pos2(xx, ly), pos2((xx + 4.0).min(area.max.x), ly)], Stroke::new(1.0, with_alpha(lc, 0.8)));
            xx += 8.0;
        }
        let tag = Rect::from_min_max(pos2(area.max.x + 2.0, ly - 9.0), pos2(r.max.x - 4.0, ly + 9.0));
        p.rect_filled(tag, 3.0, lc);
        pk::mono(&p, tag.center(), Align2::CENTER_CENTER, &px(self.price), 10.5, Color32::WHITE, tag.width());
        // pulse on the last candle
        let lx = x_of(end.saturating_sub(1));
        if self.pan < 0.5 {
            pk::dot(&p, pos2(lx, ly), 2.5, lc, Some(now));
        }
        // crosshair + legend
        let hover_i = resp.hover_pos().filter(|q| area.contains(*q)).map(|q| {
            let k = ((area.max.x - 4.0 - spacing * 0.5 - q.x) / spacing).round().max(0.0) as usize;
            (end.saturating_sub(1 + k).max(start), q)
        });
        let li = hover_i.map_or(end.saturating_sub(1), |h| h.0);
        if let Some((i, q)) = hover_i {
            let x = x_of(i);
            let cs = Stroke::new(1.0, with_alpha(t.muted_foreground, 0.7));
            let mut yy = area.min.y;
            while yy < area.max.y {
                p.line_segment([pos2(x, yy), pos2(x, (yy + 3.0).min(area.max.y))], cs);
                yy += 6.0;
            }
            let mut xx = area.min.x;
            while xx < area.max.x {
                p.line_segment([pos2(xx, q.y), pos2((xx + 3.0).min(area.max.x), q.y)], cs);
                xx += 6.0;
            }
            if q.y < price_area.max.y {
                let v = lo_t + (price_area.max.y - q.y) as f64 / price_area.height() as f64 * (hi_t - lo_t);
                let tr = Rect::from_min_max(pos2(area.max.x + 2.0, q.y - 9.0), pos2(r.max.x - 4.0, q.y + 9.0));
                p.rect_filled(tr, 3.0, t.foreground);
                pk::mono(&p, tr.center(), Align2::CENTER_CENTER, &px(v), 10.5, t.background, tr.width());
            }
            ui.ctx().set_cursor_icon(if resp.dragged() { CursorIcon::Grabbing } else { CursorIcon::Crosshair });
        }
        if let Some(b) = self.bars.get(li) {
            let c = if b.c >= b.o { up } else { dn };
            let mut lx = area.min.x + 4.0;
            let ly = area.min.y + 10.0;
            for (k, v) in [("O", b.o), ("H", b.h), ("L", b.l), ("C", b.c)] {
                let kr = pk::mono(&p, pos2(lx, ly), Align2::LEFT_CENTER, k, 10.5, t.muted_foreground, 12.0);
                let vr = pk::mono(&p, pos2(kr.max.x + 3.0, ly), Align2::LEFT_CENTER, &px(v), 10.5, c, 90.0);
                lx = vr.max.x + 10.0;
            }
            let ch = (b.c / b.o - 1.0) * 100.0;
            pk::mono(
                &p,
                pos2(lx, ly),
                Align2::LEFT_CENTER,
                &format!("{}{:.2}%", if ch >= 0.0 { "+" } else { "" }, ch),
                10.5,
                c,
                70.0,
            );
            let mut lx = area.min.x + 4.0;
            let ly2 = ly + 16.0;
            for (k, (on, series, col, name)) in
                [(self.indicators[0], &e9, ind_cols[0], "EMA 9"), (self.indicators[1], &e21, ind_cols[1], "EMA 21")]
                    .into_iter()
                    .enumerate()
            {
                if !on {
                    continue;
                }
                let _ = k;
                let nr = pk::mono(&p, pos2(lx, ly2), Align2::LEFT_CENTER, name, 10.5, col, 60.0);
                let vr = pk::mono(&p, pos2(nr.max.x + 4.0, ly2), Align2::LEFT_CENTER, &px(series[li]), 10.5, col, 90.0);
                lx = vr.max.x + 12.0;
            }
        }
        if self.pan > 0.5 {
            let br = Rect::from_min_size(pos2(area.max.x - 96.0, area.max.y - vol_h - 30.0), vec2(90.0, 24.0));
            if pk::small_btn(ui, br, id.with("live"), "Go live", Some(LineIcon::ChevronRight.into()), None, None)
                .clicked()
            {
                self.pan = 0.0;
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // order book + tape
    // -----------------------------------------------------------------------------------------

    #[allow(clippy::needless_range_loop)]
    fn book(&mut self, ui: &mut Ui, r: Rect, id: Id, now: f64) {
        let t = Theme::of(ui);
        let (up, dn) = Self::up_down(&t);
        let p = ui.painter().clone();
        let x0 = r.min.x + 10.0;
        let x1 = r.max.x - 10.0;
        kit::text_at(&p, pos2(x0, r.min.y + 18.0), Align2::LEFT_CENTER, "Order book", 13.0, t.foreground, 120.0);
        let ts = tick_size(self.price);
        kit::text_at(
            &p,
            pos2(x1, r.min.y + 18.0),
            Align2::RIGHT_CENTER,
            &format!("tick {ts}"),
            11.0,
            t.muted_foreground,
            80.0,
        );
        let hy = r.min.y + 40.0;
        let cx2 = x0 + (x1 - x0) * 0.62;
        kit::text_at(&p, pos2(x0, hy), Align2::LEFT_CENTER, "Price (USD)", 10.5, t.muted_foreground, 80.0);
        kit::text_at(
            &p,
            pos2(cx2, hy),
            Align2::RIGHT_CENTER,
            &format!("Size ({})", SYMBOLS[self.symbol].0),
            10.5,
            t.muted_foreground,
            80.0,
        );
        kit::text_at(&p, pos2(x1, hy), Align2::RIGHT_CENTER, "Total", 10.5, t.muted_foreground, 60.0);
        let rows_top = hy + 10.0;
        let mid_h = 30.0;
        let avail = r.max.y - 6.0 - rows_top - mid_h;
        let rh = (avail / (2 * LEVELS) as f32).clamp(12.0, 18.0);
        let n = ((avail / 2.0) / rh).floor() as usize;
        let n = n.min(LEVELS);
        let spread_ticks = 1.0;
        let best_ask = self.price + ts * spread_ticks * 0.5;
        let best_bid = self.price - ts * spread_ticks * 0.5;
        let cum_ask: Vec<f64> = self
            .asks
            .iter()
            .scan(0.0, |a, l| {
                *a += l.size;
                Some(*a)
            })
            .collect();
        let cum_bid: Vec<f64> = self
            .bids
            .iter()
            .scan(0.0, |a, l| {
                *a += l.size;
                Some(*a)
            })
            .collect();
        let maxc = cum_ask[n.saturating_sub(1)].max(cum_bid[n.saturating_sub(1)]).max(1e-9);
        let mut clicked_price = None;
        let mut row =
            |p: &egui::Painter, ui: &mut Ui, y: f32, price: f64, lv: Level, cum: f64, ask: bool, key: usize| {
                let rr = Rect::from_min_max(pos2(r.min.x + 2.0, y), pos2(r.max.x - 2.0, y + rh));
                let resp = ui
                    .interact(rr, id.with(("lv", ask, key)), Sense::click())
                    .on_hover_cursor(CursorIcon::PointingHand);
                let c = if ask { dn } else { up };
                let depth_w = (cum / maxc) as f32 * rr.width();
                p.rect_filled(
                    Rect::from_min_max(pos2(rr.max.x - depth_w, rr.min.y + 1.0), pos2(rr.max.x, rr.max.y - 1.0)),
                    0.0,
                    with_alpha(c, if t.dark { 0.13 } else { 0.1 }),
                );
                let fl = (1.0 - ((now - lv.flash) / 0.5) as f32).clamp(0.0, 1.0);
                if fl > 0.0 {
                    let fc = if lv.up { up } else { dn };
                    p.rect_filled(
                        Rect::from_min_max(pos2(cx2 - 64.0, rr.min.y + 1.0), pos2(cx2 + 2.0, rr.max.y - 1.0)),
                        2.0,
                        with_alpha(fc, 0.22 * fl),
                    );
                }
                if resp.hovered() {
                    p.rect_filled(rr, 0.0, kit::hover_fill(&t, 1.4));
                }
                let fs = (rh - 3.0).clamp(10.0, 11.5);
                pk::mono(p, pos2(x0, rr.center().y), Align2::LEFT_CENTER, &px(price), fs, c, 90.0);
                pk::mono(
                    p,
                    pos2(cx2, rr.center().y),
                    Align2::RIGHT_CENTER,
                    &format!("{:.4}", lv.size),
                    fs,
                    t.foreground,
                    70.0,
                );
                pk::mono(
                    p,
                    pos2(x1, rr.center().y),
                    Align2::RIGHT_CENTER,
                    &format!("{:.3}", cum),
                    fs,
                    t.muted_foreground,
                    60.0,
                );
                if resp.clicked() {
                    clicked_price = Some(price);
                }
            };
        // asks: best at the bottom
        for k in 0..n {
            let lvl = n - 1 - k;
            let y = rows_top + k as f32 * rh;
            row(&p, ui, y, best_ask + ts * lvl as f64, self.asks[lvl], cum_ask[lvl], true, lvl);
        }
        // spread row
        let my = rows_top + n as f32 * rh;
        let mr = Rect::from_min_max(pos2(r.min.x + 1.0, my), pos2(r.max.x - 1.0, my + mid_h));
        p.rect_filled(mr, 0.0, mix(t.card, t.foreground, 0.03));
        let dir_up = self.price >= self.prev_price;
        let fl = (1.0 - ((now - self.price_flash) / 0.6) as f32).clamp(0.0, 1.0);
        let pc = if dir_up { up } else { dn };
        let pr = pk::mono(
            &p,
            pos2(x0, mr.center().y),
            Align2::LEFT_CENTER,
            &px(self.price),
            15.0,
            t.foreground.lerp_to_gamma(pc, 0.5 + 0.5 * fl),
            120.0,
        );
        let arrow = if dir_up { UiIcon::ArrowUp } else { UiIcon::ArrowDown };
        AppIcon::Ui(arrow).paint(&p, Rect::from_center_size(pos2(pr.max.x + 9.0, mr.center().y), vec2(12.0, 12.0)), pc);
        kit::text_at(
            &p,
            pos2(x1, mr.center().y),
            Align2::RIGHT_CENTER,
            &format!("Spread {ts} · {:.3}%", ts / self.price * 100.0),
            10.5,
            t.muted_foreground,
            120.0,
        );
        // bids
        for k in 0..n {
            let y = my + mid_h + k as f32 * rh;
            row(&p, ui, y, best_bid - ts * k as f64, self.bids[k], cum_bid[k], false, k);
        }
        if let Some(v) = clicked_price {
            self.price_s = px(v);
            self.otype = 0;
            toast(ui.ctx(), Toast::new(format!("Limit price set to {}", px(v))));
        }
    }

    fn trades(&mut self, ui: &mut Ui, r: Rect, now: f64) {
        let t = Theme::of(ui);
        let (up, dn) = Self::up_down(&t);
        let p = ui.painter().clone();
        let x0 = r.min.x + 10.0;
        let x1 = r.max.x - 10.0;
        kit::text_at(&p, pos2(x0, r.min.y + 18.0), Align2::LEFT_CENTER, "Trades", 13.0, t.foreground, 100.0);
        pk::dot(&p, pos2(x1 - 30.0, r.min.y + 18.0), 3.0, up, Some(now));
        kit::text_at(&p, pos2(x1, r.min.y + 18.0), Align2::RIGHT_CENTER, "Live", 11.0, t.muted_foreground, 30.0);
        let hy = r.min.y + 38.0;
        let cx2 = x0 + (x1 - x0) * 0.62;
        kit::text_at(&p, pos2(x0, hy), Align2::LEFT_CENTER, "Price", 10.5, t.muted_foreground, 60.0);
        kit::text_at(&p, pos2(cx2, hy), Align2::RIGHT_CENTER, "Size", 10.5, t.muted_foreground, 60.0);
        kit::text_at(&p, pos2(x1, hy), Align2::RIGHT_CENTER, "Time", 10.5, t.muted_foreground, 60.0);
        let rh = 17.0;
        let mut y = hy + 10.0;
        let clip = r.shrink(1.0);
        let p = p.with_clip_rect(clip);
        for pr in &self.tape {
            if y + rh > r.max.y - 4.0 {
                break;
            }
            let rr = Rect::from_min_max(pos2(r.min.x + 2.0, y), pos2(r.max.x - 2.0, y + rh));
            let c = if pr.buy { up } else { dn };
            let fl = (1.0 - ((now - pr.born) / 0.7) as f32).clamp(0.0, 1.0);
            if fl > 0.0 {
                p.rect_filled(rr, 0.0, with_alpha(c, 0.18 * fl));
            }
            if pr.size > 1.2 {
                p.rect_filled(Rect::from_min_size(rr.min, vec2(2.0, rh)), 0.0, c);
            }
            pk::mono(&p, pos2(x0, rr.center().y), Align2::LEFT_CENTER, &px(pr.price), 11.0, c, 90.0);
            pk::mono(
                &p,
                pos2(cx2, rr.center().y),
                Align2::RIGHT_CENTER,
                &format!("{:.4}", pr.size),
                11.0,
                if pr.size > 1.2 { t.foreground } else { mix(t.foreground, t.muted_foreground, 0.4) },
                70.0,
            );
            pk::mono(&p, pos2(x1, rr.center().y), Align2::RIGHT_CENTER, &pr.time, 11.0, t.muted_foreground, 70.0);
            y += rh;
        }
    }

    // -----------------------------------------------------------------------------------------
    // positions / orders / fills
    // -----------------------------------------------------------------------------------------

    fn bottom(&mut self, ui: &mut Ui, r: Rect, id: Id, now: f64) {
        let t = Theme::of(ui);
        let (up, dn) = Self::up_down(&t);
        let p = ui.painter().clone();
        let labels = [
            format!("Positions ({})", self.positions.len()),
            format!("Open orders ({})", self.orders.len()),
            "Fills".to_string(),
        ];
        let lr: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        pk::mini_tabs(
            ui,
            Rect::from_min_size(pos2(r.min.x + 6.0, r.min.y + 2.0), vec2(360.0, 32.0)),
            id.with("tabs"),
            &lr,
            &mut self.bottom_tab,
        );
        // totals
        let mut upnl = 0.0;
        for ps in &self.positions {
            let m = self.watch[ps.symbol].price;
            upnl += (m - ps.entry) * ps.qty * if ps.long { 1.0 } else { -1.0 };
        }
        let tr = pk::mono(
            &p,
            pos2(r.max.x - 12.0, r.min.y + 18.0),
            Align2::RIGHT_CENTER,
            &format!("{}{}", if upnl >= 0.0 { "+" } else { "" }, kit::money(upnl)),
            12.5,
            if upnl >= 0.0 { up } else { dn },
            140.0,
        );
        kit::text_at(
            &p,
            pos2(tr.min.x - 8.0, r.min.y + 18.0),
            Align2::RIGHT_CENTER,
            "Unrealized PnL",
            11.0,
            t.muted_foreground,
            120.0,
        );
        kit::hline(&p, r.min.x + 1.0, r.max.x - 1.0, r.min.y + 34.0, &t);
        let x0 = r.min.x + 12.0;
        let w = r.width() - 24.0;
        let hy = r.min.y + 50.0;
        let rh = 30.0;
        let cols: Vec<(f32, &str, bool)> = match self.bottom_tab {
            0 => vec![
                (0.0, "Market", false),
                (0.15, "Side", false),
                (0.3, "Size", true),
                (0.45, "Entry", true),
                (0.6, "Mark", true),
                (0.8, "PnL", true),
                (1.0, "", true),
            ],
            1 => vec![
                (0.0, "Market", false),
                (0.15, "Side", false),
                (0.35, "Size", true),
                (0.55, "Limit", true),
                (0.75, "Placed", true),
                (1.0, "", true),
            ],
            _ => vec![(0.0, "Time", false), (0.2, "Side", false), (0.45, "Size", true), (0.7, "Price", true)],
        };
        let col_x = |f: f32| x0 + w * f - if f >= 1.0 { 64.0 } else { 0.0 };
        for (f, name, right) in &cols {
            let x = col_x(*f);
            if *right {
                kit::text_at(&p, pos2(x, hy), Align2::RIGHT_CENTER, name, 10.5, t.muted_foreground, 80.0);
            } else {
                kit::text_at(&p, pos2(x, hy), Align2::LEFT_CENTER, name, 10.5, t.muted_foreground, 80.0);
            }
        }
        let mut y = hy + 12.0;
        let mut close_pos = None;
        let mut cancel = None;
        let p2 = p.with_clip_rect(r.shrink(1.0));
        match self.bottom_tab {
            0 => {
                for (k, ps) in self.positions.iter().enumerate() {
                    let rr = Rect::from_min_max(pos2(r.min.x + 4.0, y), pos2(r.max.x - 4.0, y + rh));
                    if rr.max.y > r.max.y {
                        break;
                    }
                    let (_resp, _) = kit::row(ui, rr, id.with(("prow", k)), 4.0);
                    let cy = rr.center().y;
                    let m = self.watch[ps.symbol].price;
                    let pnl = (m - ps.entry) * ps.qty * if ps.long { 1.0 } else { -1.0 };
                    let pct = pnl / (ps.entry * ps.qty) * 100.0;
                    kit::text_at(
                        &p2,
                        pos2(col_x(0.0), cy),
                        Align2::LEFT_CENTER,
                        &format!("{}-USD", SYMBOLS[ps.symbol].0),
                        12.0,
                        t.foreground,
                        w * 0.14,
                    );
                    pk::tag(
                        &p2,
                        pos2(col_x(0.15), cy),
                        false,
                        if ps.long { "Long 5x" } else { "Short 3x" },
                        if ps.long { up } else { dn },
                        &t,
                        false,
                    );
                    pk::mono(
                        &p2,
                        pos2(col_x(0.3), cy),
                        Align2::RIGHT_CENTER,
                        &format!("{}", ps.qty),
                        11.5,
                        t.foreground,
                        90.0,
                    );
                    pk::mono(&p2, pos2(col_x(0.45), cy), Align2::RIGHT_CENTER, &px(ps.entry), 11.5, t.foreground, 90.0);
                    pk::mono(&p2, pos2(col_x(0.6), cy), Align2::RIGHT_CENTER, &px(m), 11.5, t.foreground, 90.0);
                    let c = if pnl >= 0.0 { up } else { dn };
                    pk::mono(
                        &p2,
                        pos2(col_x(0.8), cy - 6.0),
                        Align2::RIGHT_CENTER,
                        &format!("{}{}", if pnl >= 0.0 { "+" } else { "" }, kit::money(pnl)),
                        11.5,
                        c,
                        110.0,
                    );
                    pk::mono(
                        &p2,
                        pos2(col_x(0.8), cy + 7.0),
                        Align2::RIGHT_CENTER,
                        &format!("{}{:.2}%", if pct >= 0.0 { "+" } else { "" }, pct),
                        10.0,
                        c,
                        110.0,
                    );
                    let br = Rect::from_min_max(pos2(r.max.x - 70.0, cy - 11.0), pos2(r.max.x - 12.0, cy + 11.0));
                    if pk::small_btn(ui, br, id.with(("close", k)), "Close", None, None, None).clicked() {
                        close_pos = Some(k);
                    }
                    y += rh;
                }
                if self.positions.is_empty() {
                    kit::text_at(
                        &p,
                        pos2(r.center().x, y + 30.0),
                        Align2::CENTER_CENTER,
                        "No open positions",
                        12.0,
                        t.muted_foreground,
                        200.0,
                    );
                }
            }
            1 => {
                for (k, o) in self.orders.iter().enumerate() {
                    let rr = Rect::from_min_max(pos2(r.min.x + 4.0, y), pos2(r.max.x - 4.0, y + rh));
                    if rr.max.y > r.max.y {
                        break;
                    }
                    kit::row(ui, rr, id.with(("orow", k)), 4.0);
                    let cy = rr.center().y;
                    kit::text_at(
                        &p2,
                        pos2(col_x(0.0), cy),
                        Align2::LEFT_CENTER,
                        &format!("{}-USD", SYMBOLS[o.symbol].0),
                        12.0,
                        t.foreground,
                        w * 0.14,
                    );
                    pk::tag(
                        &p2,
                        pos2(col_x(0.15), cy),
                        false,
                        if o.buy { "Buy limit" } else { "Sell limit" },
                        if o.buy { up } else { dn },
                        &t,
                        false,
                    );
                    pk::mono(
                        &p2,
                        pos2(col_x(0.35), cy),
                        Align2::RIGHT_CENTER,
                        &format!("{}", o.qty),
                        11.5,
                        t.foreground,
                        90.0,
                    );
                    pk::mono(&p2, pos2(col_x(0.55), cy), Align2::RIGHT_CENTER, &px(o.price), 11.5, t.foreground, 90.0);
                    pk::mono(
                        &p2,
                        pos2(col_x(0.75), cy),
                        Align2::RIGHT_CENTER,
                        &o.placed,
                        11.5,
                        t.muted_foreground,
                        90.0,
                    );
                    let br = Rect::from_min_max(pos2(r.max.x - 70.0, cy - 11.0), pos2(r.max.x - 12.0, cy + 11.0));
                    if pk::small_btn(ui, br, id.with(("cancel", k)), "Cancel", None, None, None).clicked() {
                        cancel = Some(k);
                    }
                    y += rh;
                }
                if self.orders.is_empty() {
                    kit::text_at(
                        &p,
                        pos2(r.center().x, y + 30.0),
                        Align2::CENTER_CENTER,
                        "No open orders",
                        12.0,
                        t.muted_foreground,
                        200.0,
                    );
                }
            }
            _ => {
                for f in &self.fills {
                    if y + 24.0 > r.max.y {
                        break;
                    }
                    let cy = y + 12.0;
                    let fl = (1.0 - ((now - f.born) / 1.0) as f32).clamp(0.0, 1.0);
                    if fl > 0.0 {
                        p2.rect_filled(
                            Rect::from_min_max(pos2(r.min.x + 4.0, y), pos2(r.max.x - 4.0, y + 24.0)),
                            4.0,
                            with_alpha(if f.buy { up } else { dn }, 0.15 * fl),
                        );
                    }
                    pk::mono(&p2, pos2(col_x(0.0), cy), Align2::LEFT_CENTER, &f.time, 11.5, t.muted_foreground, 90.0);
                    pk::tag(
                        &p2,
                        pos2(col_x(0.2), cy),
                        false,
                        if f.buy { "Buy" } else { "Sell" },
                        if f.buy { up } else { dn },
                        &t,
                        false,
                    );
                    pk::mono(
                        &p2,
                        pos2(col_x(0.45), cy),
                        Align2::RIGHT_CENTER,
                        &format!("{}", f.size.abs()),
                        11.5,
                        t.foreground,
                        90.0,
                    );
                    pk::mono(&p2, pos2(col_x(0.7), cy), Align2::RIGHT_CENTER, &px(f.price), 11.5, t.foreground, 90.0);
                    y += 24.0;
                }
                if self.fills.is_empty() {
                    kit::text_at(
                        &p,
                        pos2(r.center().x, y + 30.0),
                        Align2::CENTER_CENTER,
                        "No fills this session",
                        12.0,
                        t.muted_foreground,
                        200.0,
                    );
                }
            }
        }
        if let Some(k) = close_pos {
            let ps = self.positions[k].clone();
            let m = self.watch[ps.symbol].price;
            let pnl = (m - ps.entry) * ps.qty * if ps.long { 1.0 } else { -1.0 };
            self.apply_fill(ps.symbol, !ps.long, ps.qty, m, now);
            self.balance += pnl;
            toast(
                ui.ctx(),
                Toast::success(format!("Closed {}-USD", SYMBOLS[ps.symbol].0)).description(format!(
                    "Realized {}{}",
                    if pnl >= 0.0 { "+" } else { "" },
                    kit::money(pnl)
                )),
            );
        }
        if let Some(k) = cancel {
            self.orders.remove(k);
            toast(ui.ctx(), Toast::new("Order cancelled"));
        }
    }

    // -----------------------------------------------------------------------------------------
    // order entry
    // -----------------------------------------------------------------------------------------

    fn entry(&mut self, ui: &mut Ui, r: Rect, id: Id, now: f64) {
        let t = Theme::of(ui);
        let (up, dn) = Self::up_down(&t);
        let p = ui.painter().clone();
        let x0 = r.min.x + 12.0;
        let x1 = r.max.x - 12.0;
        // buy / sell
        let seg = Rect::from_min_max(pos2(x0, r.min.y + 12.0), pos2(x1, r.min.y + 44.0));
        p.rect_filled(seg, 7.0, mix(t.card, t.foreground, 0.05));
        let halves = kit::cols(seg.shrink(3.0), 2, 3.0);
        let k = kit::follow(ui.ctx(), id.with("side"), self.side as f32, 18.0);
        let ind = halves[0].translate(vec2((halves[1].min.x - halves[0].min.x) * k, 0.0));
        let side_col = if self.side == 0 { up } else { dn };
        p.rect_filled(ind, 5.0, up.lerp_to_gamma(dn, k));
        for (i, (label, key)) in [("Buy", "B"), ("Sell", "S")].iter().enumerate() {
            let resp =
                ui.interact(halves[i], id.with(("s", i)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            kit::tip(&resp, &format!("{label} {}", SYMBOLS[self.symbol].0), Some(key));
            if resp.clicked() {
                self.side = i;
            }
            let on = self.side == i;
            p.text(
                halves[i].center(),
                Align2::CENTER_CENTER,
                *label,
                FontId::proportional(13.0),
                if on { Color32::WHITE } else { t.muted_foreground },
            );
        }
        // type tabs
        let mut ot = self.otype;
        pk::mini_tabs(
            ui,
            Rect::from_min_size(pos2(x0 - 8.0, r.min.y + 48.0), vec2(220.0, 30.0)),
            id.with("otype"),
            &["Limit", "Market", "Stop"],
            &mut ot,
        );
        self.otype = ot;
        let avail = if self.side == 0 {
            self.balance
        } else {
            self.positions.iter().find(|p| p.symbol == self.symbol && p.long).map_or(0.0, |p| p.qty)
        };
        kit::text_at(
            &p,
            pos2(x1, r.min.y + 64.0),
            Align2::RIGHT_CENTER,
            &if self.side == 0 { kit::money(avail) } else { format!("{avail} {}", SYMBOLS[self.symbol].0) },
            11.0,
            t.muted_foreground,
            120.0,
        );
        let mut y = r.min.y + 88.0;
        // price
        let fh = 32.0;
        let label_w = 52.0;
        kit::text_at(&p, pos2(x0, y + fh / 2.0), Align2::LEFT_CENTER, "Price", 12.0, t.muted_foreground, label_w);
        let pr = Rect::from_min_max(pos2(x0 + label_w, y), pos2(x1, y + fh));
        if self.otype == 1 {
            p.rect_filled(pr, 8.0, mix(t.card, t.foreground, 0.04));
            kit::text_at(
                &p,
                pos2(pr.min.x + 12.0, pr.center().y),
                Align2::LEFT_CENTER,
                "Market price",
                12.5,
                t.muted_foreground,
                pr.width() - 20.0,
            );
        } else {
            pk::text_line(
                ui,
                Rect::from_min_max(pr.min, pos2(pr.max.x - 44.0, pr.max.y)),
                id.with("price"),
                &mut self.price_s,
                "0.00",
                true,
            );
            let lb = Rect::from_min_max(pos2(pr.max.x - 40.0, pr.min.y + 4.0), pos2(pr.max.x, pr.max.y - 4.0));
            let b = pk::small_btn(ui, lb, id.with("last"), "Last", None, None, None);
            if b.clicked() {
                self.price_s = px(self.price);
            }
        }
        y += fh + 10.0;
        kit::text_at(&p, pos2(x0, y + fh / 2.0), Align2::LEFT_CENTER, "Size", 12.0, t.muted_foreground, label_w);
        let sr = Rect::from_min_max(pos2(x0 + label_w, y), pos2(x1, y + fh));
        pk::text_line(ui, sr, id.with("qty"), &mut self.qty_s, "0.00", true);
        kit::text_at(
            &p,
            pos2(sr.max.x - 10.0, sr.center().y),
            Align2::RIGHT_CENTER,
            SYMBOLS[self.symbol].0,
            11.5,
            t.muted_foreground,
            40.0,
        );
        y += fh + 10.0;
        // presets
        let chips = kit::cols(Rect::from_min_max(pos2(x0 + label_w, y), pos2(x1, y + 24.0)), 4, 6.0);
        let price_num = if self.otype == 1 {
            self.price
        } else {
            self.price_s.replace(',', "").parse::<f64>().unwrap_or(self.price)
        };
        for (k, c) in chips.iter().enumerate() {
            let f = (k + 1) as f64 * 0.25;
            let label = format!("{}%", (k + 1) * 25);
            if pk::chip(ui, *c, id.with(("pct", k)), &label, false, side_col).clicked() {
                let q = if self.side == 0 { self.balance * 5.0 * f / price_num } else { avail * f };
                self.qty_s = format!("{:.4}", q);
            }
        }
        y += 38.0;
        let qty: f64 = self.qty_s.replace(',', "").parse().unwrap_or(0.0);
        let total = qty * price_num;
        let rows = [
            ("Order value", kit::money(total)),
            ("Margin (5x)", kit::money(total / 5.0)),
            ("Est. fee (0.04%)", kit::money(total * 0.0004)),
        ];
        for (a, b) in rows {
            kit::text_at(&p, pos2(x0, y), Align2::LEFT_CENTER, a, 11.5, t.muted_foreground, 120.0);
            pk::mono(&p, pos2(x1, y), Align2::RIGHT_CENTER, &b, 11.5, t.foreground, 140.0);
            y += 19.0;
        }
        y += 6.0;
        let br = Rect::from_min_max(pos2(x0, y), pos2(x1, (y + 40.0).min(r.max.y - 10.0)));
        let verb = if self.side == 0 { "Buy / Long" } else { "Sell / Short" };
        let label = format!("{verb} {}", SYMBOLS[self.symbol].0);
        let resp = ui.interact(br, id.with("submit"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        let hv = kit::hover(ui, id.with("submit"), resp.hovered());
        let press = if resp.is_pointer_button_down_on() { 0.97 } else { 1.0 };
        let brr = Rect::from_center_size(br.center(), br.size() * press);
        p.rect_filled(brr, 8.0, mix(side_col, Color32::WHITE, hv * 0.08));
        p.text(brr.center(), Align2::CENTER_CENTER, label, FontId::proportional(14.0), Color32::WHITE);
        if resp.clicked() {
            if qty <= 0.0 {
                toast(ui.ctx(), Toast::error("Enter a size").description("Size must be greater than zero."));
            } else if self.otype == 1 {
                let fill = self.price;
                self.apply_fill(self.symbol, self.side == 0, qty, fill, now);
                self.bottom_tab = 0;
                toast(
                    ui.ctx(),
                    Toast::success(format!(
                        "{} {qty} {}",
                        if self.side == 0 { "Bought" } else { "Sold" },
                        SYMBOLS[self.symbol].0
                    ))
                    .description(format!("Market fill at {}", px(fill))),
                );
            } else {
                self.orders.insert(
                    0,
                    OpenOrder {
                        symbol: self.symbol,
                        buy: self.side == 0,
                        qty,
                        price: price_num,
                        placed: self.now_label(0),
                    },
                );
                self.bottom_tab = 1;
                toast(
                    ui.ctx(),
                    Toast::success("Limit order placed").description(format!(
                        "{} {qty} {} @ {}",
                        if self.side == 0 { "Buy" } else { "Sell" },
                        SYMBOLS[self.symbol].0,
                        px(price_num)
                    )),
                );
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // watchlist
    // -----------------------------------------------------------------------------------------

    fn watchlist(&mut self, ui: &mut Ui, r: Rect, id: Id) {
        let t = Theme::of(ui);
        let (up, dn) = Self::up_down(&t);
        let p = ui.painter().clone();
        let x0 = r.min.x + 12.0;
        let x1 = r.max.x - 12.0;
        kit::text_at(&p, pos2(x0, r.min.y + 18.0), Align2::LEFT_CENTER, "Watchlist", 13.0, t.foreground, 100.0);
        AppIcon::Ui(UiIcon::Star).paint(
            &p,
            Rect::from_center_size(pos2(x1 - 6.0, r.min.y + 18.0), vec2(13.0, 13.0)),
            t.warning,
        );
        let mut switch_to = None;
        let lr = Rect::from_min_max(pos2(r.min.x + 1.0, r.min.y + 34.0), pos2(r.max.x - 1.0, r.max.y - 4.0));
        kit::child(ui, lr, |ui| {
            egui::ScrollArea::vertical().id_salt("term_watch").auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let w = ui.available_width();
                for (k, wt) in self.watch.iter().enumerate() {
                    let (rr, resp) = ui.allocate_exact_size(vec2(w, 38.0), Sense::click());
                    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
                    let hv = kit::hover(ui, id.with(("w", k)), resp.hovered());
                    let p = ui.painter();
                    if k == self.symbol {
                        p.rect_filled(rr.shrink2(vec2(5.0, 1.0)), 5.0, kit::active_fill(&t, t.card));
                    } else if hv > 0.01 {
                        p.rect_filled(rr.shrink2(vec2(5.0, 1.0)), 5.0, kit::hover_fill(&t, hv));
                    }
                    let chg = (wt.price / wt.open - 1.0) * 100.0;
                    let c = if chg >= 0.0 { up } else { dn };
                    let cy = rr.center().y;
                    kit::text_at(
                        p,
                        pos2(rr.min.x + 12.0, cy - 7.0),
                        Align2::LEFT_CENTER,
                        wt.sym,
                        12.5,
                        t.foreground,
                        60.0,
                    );
                    kit::text_at(
                        p,
                        pos2(rr.min.x + 12.0, cy + 8.0),
                        Align2::LEFT_CENTER,
                        wt.name,
                        10.5,
                        t.muted_foreground,
                        70.0,
                    );
                    let sr = Rect::from_min_max(
                        pos2(rr.min.x + 84.0, cy - 10.0),
                        pos2(rr.min.x + 84.0 + (w - 190.0).max(30.0), cy + 10.0),
                    );
                    pk::spark(p, sr, &wt.hist, c, true);
                    pk::mono(
                        p,
                        pos2(rr.max.x - 12.0, cy - 7.0),
                        Align2::RIGHT_CENTER,
                        &px(wt.price),
                        11.5,
                        t.foreground,
                        90.0,
                    );
                    pk::mono(
                        p,
                        pos2(rr.max.x - 12.0, cy + 8.0),
                        Align2::RIGHT_CENTER,
                        &format!("{}{:.2}%", if chg >= 0.0 { "+" } else { "" }, chg),
                        10.5,
                        c,
                        90.0,
                    );
                    if resp.clicked() && k != self.symbol {
                        switch_to = Some(k);
                    }
                }
            });
        });
        if let Some(k) = switch_to {
            self.load_symbol(k);
        }
    }
}
